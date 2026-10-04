use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct AnalyticsSubmitRequest {
    pub mod_slug: String,
    pub server_uuid: String,
    pub metrics: Metrics,
}

#[derive(Debug, Deserialize)]
pub struct Metrics {
    pub mc_version: String,
    pub loader: String,
    pub java_version: String,
    pub os: String,
    pub player_count: i32,
    #[serde(default)]
    pub custom_charts: HashMap<String, String>,
}

#[derive(Debug, Serialize)]
pub struct AnalyticsSubmitResponse {
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct AnalyticsGetResponse {
    pub mod_slug: String,
    pub range: String,
    pub daily: Vec<DailyPoint>,
    pub breakdown: Breakdown,
}

#[derive(Debug, Serialize)]
pub struct DailyPoint {
    pub date: String,
    pub active_servers: i32,
    pub active_players: i32,
}

#[derive(Debug, Serialize, Default)]
pub struct Breakdown {
    pub mc_versions: HashMap<String, i32>,
    pub loaders: HashMap<String, i32>,
    pub os: HashMap<String, i32>,
    pub java: HashMap<String, i32>,
    pub custom: HashMap<String, HashMap<String, i32>>,
}

#[derive(Debug, Deserialize)]
pub struct AnalyticsQuery {
    pub range: Option<String>,
}

fn daily_salt_today() -> String {
    Utc::now().format("%Y-%m-%d").to_string()
}

fn hash_server_uuid(uuid: &str, salt: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(uuid.as_bytes());
    hasher.update(salt.as_bytes());
    hex::encode(hasher.finalize())
}

/// POST /api/v1/analytics/submit
pub async fn submit(
    State(state): State<AppState>,
    Json(req): Json<AnalyticsSubmitRequest>,
) -> impl IntoResponse {
    // 1. validate custom_charts
    if req.metrics.custom_charts.len() > 5 {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error":"too many custom_charts"}))).into_response();
    }
    // allow-list (MVP lenient)
    if req.metrics.loader.is_empty() || req.metrics.mc_version.is_empty() {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error":"invalid metrics"}))).into_response();
    }

    // 2. lookup mod_id
    let mod_id: Option<uuid::Uuid> =
        sqlx::query_scalar("SELECT id FROM mods WHERE slug = $1")
            .bind(&req.mod_slug)
            .fetch_optional(&state.db)
            .await
            .unwrap_or(None);

    let mod_id = match mod_id {
        Some(id) => id,
        None => return (StatusCode::NOT_FOUND, Json(serde_json::json!({"error":"mod_not_found"}))).into_response(),
    };

    // 3. daily salt (fetch or create)
    let today = Utc::now().date_naive();
    let salt: String = match sqlx::query_scalar("SELECT salt FROM analytics_salts WHERE date = $1")
        .bind(today)
        .fetch_optional(&state.db)
        .await
        .unwrap_or(None)
    {
        Some(s) => s,
        None => {
            let new_salt = hex::encode(Sha256::digest(daily_salt_today().as_bytes()));
            let _ = sqlx::query("INSERT INTO analytics_salts (date, salt) VALUES ($1, $2) ON CONFLICT DO NOTHING")
                .bind(today)
                .bind(&new_salt)
                .execute(&state.db)
                .await;
            new_salt
        }
    };

    let server_hash = hash_server_uuid(&req.server_uuid, &salt);

    // 4. rate limit via DB: 1 per 15 min per server_hash+mod
    let recent: Option<i64> = sqlx::query_scalar(
        "SELECT COUNT(*) FROM mod_telemetry_pings WHERE server_hash = $1 AND mod_id = $2 AND pinged_at > NOW() - INTERVAL '15 minutes'",
    )
    .bind(&server_hash)
    .bind(mod_id)
    .fetch_one(&state.db)
    .await
    .unwrap_or(Some(0));

    if recent.unwrap_or(0) > 0 {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            Json(serde_json::json!({"error":"rate_limited"})),
        )
            .into_response();
    }

    // 5. insert ping
    let _ = sqlx::query(
        "INSERT INTO mod_telemetry_pings (mod_id, server_hash, mc_version, loader, os, java_version, player_count) VALUES ($1,$2,$3,$4,$5,$6,$7)",
    )
    .bind(mod_id)
    .bind(&server_hash)
    .bind(&req.metrics.mc_version)
    .bind(&req.metrics.loader)
    .bind(&req.metrics.os)
    .bind(&req.metrics.java_version)
    .bind(req.metrics.player_count)
    .execute(&state.db)
    .await;

    (StatusCode::OK, Json(serde_json::json!({"status":"ok"}))).into_response()
}

/// GET /api/v1/mods/{slug}/analytics?range=7d|30d|90d
pub async fn get_analytics(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Query(q): Query<AnalyticsQuery>,
) -> impl IntoResponse {
    let range = q.range.unwrap_or_else(|| "30d".to_string());
    let days: i32 = match range.as_str() {
        "7d" => 7,
        "90d" => 90,
        _ => 30,
    };

    let mod_id: Option<uuid::Uuid> = sqlx::query_scalar("SELECT id FROM mods WHERE slug = $1")
        .bind(&slug)
        .fetch_optional(&state.db)
        .await
        .unwrap_or(None);

    let mod_id = match mod_id {
        Some(id) => id,
        None => return (StatusCode::NOT_FOUND, Json(serde_json::json!({"error":"mod_not_found"}))).into_response(),
    };

    let rows = sqlx::query_as::<_, (chrono::NaiveDate, i32, i32, serde_json::Value)>(
        "SELECT date, active_servers, active_players, breakdown_json FROM mod_daily_stats WHERE mod_id = $1 AND date >= CURRENT_DATE - $2::int ORDER BY date ASC",
    )
    .bind(mod_id)
    .bind(days)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let mut daily = Vec::new();
    let mut breakdown = Breakdown::default();
    for (date, servers, players, json) in rows {
        daily.push(DailyPoint {
            date: date.to_string(),
            active_servers: servers,
            active_players: players,
        });
        // merge last breakdown
        if let Some(obj) = json.as_object() {
            if let Some(v) = obj.get("mc_versions").and_then(|x| x.as_object()) {
                for (k, val) in v {
                    if let Some(n) = val.as_i64() {
                        *breakdown.mc_versions.entry(k.clone()).or_insert(0) += n as i32;
                    }
                }
            }
        }
    }

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "mod_slug": slug,
            "range": range,
            "daily": daily,
            "breakdown": breakdown
        })),
    )
        .into_response()
}
