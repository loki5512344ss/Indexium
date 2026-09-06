use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::AppState;

// ---------------------------------------------------------------------------
// DTOs
// ---------------------------------------------------------------------------

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

#[derive(Debug, Serialize)]
pub struct Breakdown {
    pub mc_versions: HashMap<String, i32>,
    pub loaders: HashMap<String, i32>,
    pub os: HashMap<String, i32>,
    pub java: HashMap<String, i32>,
    pub custom: HashMap<String, HashMap<String, i32>>,
}

// ---------------------------------------------------------------------------
// Handlers — skeleton (без Redis/bcrypt на MVP, логика в services/analytics.rs)
// ---------------------------------------------------------------------------

/// POST /api/v1/analytics/submit
/// Валидация allow-list, хеш server_uuid + daily_salt, Redis 1/15м, INSERT pings.
/// Сейчас — заглушка, возвращает 200 без БД, чтобы SDK мог теститься.
pub async fn submit(
    State(_state): State<AppState>,
    Json(_req): Json<AnalyticsSubmitRequest>,
) -> Json<AnalyticsSubmitResponse> {
    // TODO:
    // 1. lookup mods.id by slug (404 if not found)
    // 2. validate mc_version/loader/os/java_version allow-list, custom_charts ≤5
    // 3. fetch daily_salt from analytics_salts (or generate sha256(today))
    // 4. server_hash = sha256(server_uuid + salt)
    // 5. Redis SET NX EX 900 server_hash:mod_id → 429 if exists
    // 6. INSERT mod_telemetry_pings
    Json(AnalyticsSubmitResponse {
        status: "ok".into(),
    })
}

/// GET /api/v1/mods/:slug/analytics?range=30d
pub async fn get_analytics(
    State(_state): State<AppState>,
    // TODO: extract slug + query range
) -> Json<AnalyticsGetResponse> {
    // TODO: SELECT * FROM mod_daily_stats WHERE mod_id = ? AND date >= NOW() - range
    Json(AnalyticsGetResponse {
        mod_slug: "sodium-extra".into(),
        range: "30d".into(),
        daily: vec![],
        breakdown: Breakdown {
            mc_versions: HashMap::new(),
            loaders: HashMap::new(),
            os: HashMap::new(),
            java: HashMap::new(),
            custom: HashMap::new(),
        },
    })
}
