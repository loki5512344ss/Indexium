use axum::{
    extract::{Query, State},
    http::{header, HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use cookie::{Cookie, SameSite};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct CallbackQuery {
    pub code: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: i64,
    login: String,
    exp: usize,
}

#[derive(Debug, Deserialize)]
struct TokenResp {
    access_token: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GhUser {
    id: i64,
    login: String,
    avatar_url: Option<String>,
}

fn jwt_secret() -> String {
    std::env::var("JWT_SECRET").unwrap_or_else(|_| "dev_jwt_secret_change_me".to_string())
}

fn parse_jwt_from_headers(headers: &HeaderMap) -> Option<String> {
    let cookie_header = headers.get(header::COOKIE)?.to_str().ok()?;
    for part in cookie_header.split(';') {
        let trimmed = part.trim();
        if let Ok(c) = Cookie::parse(trimmed.to_string()) {
            if c.name() == "jwt" {
                return Some(c.value().to_string());
            }
        }
        if let Some(v) = trimmed.strip_prefix("jwt=") {
            return Some(v.to_string());
        }
    }
    None
}

fn build_jwt_cookie(token: &str) -> String {
    Cookie::build(("jwt", token.to_string()))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .max_age(cookie::time::Duration::days(7))
        .build()
        .to_string()
}

fn clear_jwt_cookie() -> String {
    Cookie::build(("jwt", ""))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .max_age(cookie::time::Duration::seconds(0))
        .build()
        .to_string()
}

/// GET /api/v1/auth/github -> 302 GitHub authorize
pub async fn github_login() -> impl IntoResponse {
    let client_id = std::env::var("GITHUB_CLIENT_ID").unwrap_or_default();
    if client_id.is_empty() {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error":"oauth_not_configured"}))).into_response();
    }
    let url = format!(
        "https://github.com/login/oauth/authorize?client_id={}&scope=read:user",
        client_id
    );
    (StatusCode::FOUND, [(header::LOCATION, url)]).into_response()
}

/// GET /api/v1/auth/github/callback?code=...
pub async fn github_callback(
    State(state): State<AppState>,
    Query(q): Query<CallbackQuery>,
) -> impl IntoResponse {
    let code = match q.code {
        Some(c) if !c.is_empty() => c,
        _ => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error":"missing_code"}))).into_response(),
    };
    let client_id = std::env::var("GITHUB_CLIENT_ID").unwrap_or_default();
    let client_secret = std::env::var("GITHUB_CLIENT_SECRET").unwrap_or_default();
    if client_id.is_empty() || client_secret.is_empty() {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error":"oauth_not_configured"}))).into_response();
    }

    let client = reqwest::Client::new();
    let token_resp = client
        .post("https://github.com/login/oauth/access_token")
        .header("Accept", "application/json")
        .form(&[("client_id", &client_id), ("client_secret", &client_secret), ("code", &code)])
        .send()
        .await;

    let token_resp = match token_resp {
        Ok(r) => r,
        Err(e) => {
            tracing::error!(error=%e, "oauth token exchange failed");
            return (StatusCode::BAD_GATEWAY, Json(serde_json::json!({"error":"oauth_exchange_failed"}))).into_response();
        }
    };
    let token_data: TokenResp = match token_resp.json().await {
        Ok(v) => v,
        Err(e) => {
            tracing::error!(error=%e, "oauth token parse failed");
            return (StatusCode::BAD_GATEWAY, Json(serde_json::json!({"error":"oauth_parse_failed"}))).into_response();
        }
    };
    let access_token = match token_data.access_token {
        Some(t) if !t.is_empty() => t,
        _ => return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error":"oauth_no_token"}))).into_response(),
    };

    let user_resp = client
        .get("https://api.github.com/user")
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "Indexium")
        .bearer_auth(&access_token)
        .send()
        .await;

    let user_resp = match user_resp {
        Ok(r) => r,
        Err(e) => {
            tracing::error!(error=%e, "github user fetch failed");
            return (StatusCode::BAD_GATEWAY, Json(serde_json::json!({"error":"github_fetch_failed"}))).into_response();
        }
    };
    let gh_user: GhUser = match user_resp.json().await {
        Ok(v) => v,
        Err(e) => {
            tracing::error!(error=%e, "github user parse failed");
            return (StatusCode::BAD_GATEWAY, Json(serde_json::json!({"error":"github_parse_failed"}))).into_response();
        }
    };

    let _ = sqlx::query(
        "INSERT INTO authors (github_id, login, avatar_url) VALUES ($1,$2,$3) \
         ON CONFLICT (github_id) DO UPDATE SET login=EXCLUDED.login, avatar_url=EXCLUDED.avatar_url",
    )
    .bind(gh_user.id)
    .bind(&gh_user.login)
    .bind(&gh_user.avatar_url)
    .execute(&state.db)
    .await;

    let exp = (chrono::Utc::now().timestamp() as usize) + 7 * 24 * 3600;
    let claims = Claims { sub: gh_user.id, login: gh_user.login, exp };
    let token = match encode(&Header::default(), &claims, &EncodingKey::from_secret(jwt_secret().as_bytes())) {
        Ok(t) => t,
        Err(e) => {
            tracing::error!(error=%e, "jwt encode failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error":"jwt_failed"}))).into_response();
        }
    };
    let cookie_str = build_jwt_cookie(&token);
    let headers = [(header::LOCATION, "/".to_string()), (header::SET_COOKIE, cookie_str)];
    (StatusCode::FOUND, headers).into_response()
}

/// GET /api/v1/auth/me
pub async fn me(headers: HeaderMap) -> impl IntoResponse {
    let token = match parse_jwt_from_headers(&headers) {
        Some(t) => t,
        None => return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error":"unauthorized"}))).into_response(),
    };
    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = true;
    let data = decode::<Claims>(&token, &DecodingKey::from_secret(jwt_secret().as_bytes()), &validation);
    match data {
        Ok(d) => (StatusCode::OK, Json(serde_json::json!({"login": d.claims.login, "github_id": d.claims.sub}))).into_response(),
        Err(_) => (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error":"unauthorized"}))).into_response(),
    }
}

/// POST /api/v1/auth/logout
pub async fn logout() -> impl IntoResponse {
    let cookie_str = clear_jwt_cookie();
    let mut headers = HeaderMap::new();
    headers.insert(header::SET_COOKIE, cookie_str.parse().unwrap());
    (StatusCode::OK, headers, Json(serde_json::json!({"status":"ok"}))).into_response()
}
