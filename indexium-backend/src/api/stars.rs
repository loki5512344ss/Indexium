use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use serde::Serialize;
use serde_json::json;

use crate::{db, AppState};

#[derive(Serialize)]
struct StarsCount { count: i64 }

fn github_id_from_headers(headers: &HeaderMap) -> i64 {
    headers
        .get("x-github-id")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(1)
}

pub async fn get_stars(State(state): State<AppState>, Path(slug): Path<String>) -> impl IntoResponse {
    let m = match db::fetch_mod_by_slug(&state.db, &slug).await {
        Ok(v) => v,
        Err(e) => {
            tracing::error!(error=%e, slug=%slug, "get_stars fetch_mod failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error":"internal_error"}))).into_response();
        }
    };
    let Some(m) = m else {
        return (StatusCode::NOT_FOUND, Json(json!({"error":"mod_not_found"}))).into_response();
    };
    let row: Result<(i64,), _> = sqlx::query_as("SELECT COUNT(*) FROM stars WHERE mod_id=$1")
        .bind(m.id)
        .fetch_one(&state.db)
        .await;
    match row {
        Ok((c,)) => (StatusCode::OK, Json(json!(StarsCount { count: c }))).into_response(),
        Err(e) => {
            tracing::error!(error=%e, "get_stars count failed");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error":"internal_error"}))).into_response()
        }
    }
}

pub async fn star_mod(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let github_id = github_id_from_headers(&headers);
    let m = match db::fetch_mod_by_slug(&state.db, &slug).await {
        Ok(v) => v,
        Err(e) => {
            tracing::error!(error=%e, slug=%slug, "star_mod fetch_mod failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error":"internal_error"}))).into_response();
        }
    };
    let Some(m) = m else {
        return (StatusCode::NOT_FOUND, Json(json!({"error":"mod_not_found"}))).into_response();
    };
    if let Err(e) = sqlx::query("INSERT INTO stars (github_id, mod_id) VALUES ($1,$2) ON CONFLICT DO NOTHING")
        .bind(github_id)
        .bind(m.id)
        .execute(&state.db)
        .await
    {
        tracing::error!(error=%e, "star_mod insert failed");
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error":"internal_error"}))).into_response();
    }
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM stars WHERE mod_id=$1")
        .bind(m.id)
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);
    (StatusCode::OK, Json(json!({"count": count}))).into_response()
}

pub async fn unstar_mod(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let github_id = github_id_from_headers(&headers);
    let m = match db::fetch_mod_by_slug(&state.db, &slug).await {
        Ok(v) => v,
        Err(e) => {
            tracing::error!(error=%e, slug=%slug, "unstar_mod fetch_mod failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error":"internal_error"}))).into_response();
        }
    };
    let Some(m) = m else {
        return (StatusCode::NOT_FOUND, Json(json!({"error":"mod_not_found"}))).into_response();
    };
    if let Err(e) = sqlx::query("DELETE FROM stars WHERE github_id=$1 AND mod_id=$2")
        .bind(github_id)
        .bind(m.id)
        .execute(&state.db)
        .await
    {
        tracing::error!(error=%e, "unstar_mod delete failed");
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error":"internal_error"}))).into_response();
    }
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM stars WHERE mod_id=$1")
        .bind(m.id)
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);
    (StatusCode::OK, Json(json!({"count": count}))).into_response()
}
