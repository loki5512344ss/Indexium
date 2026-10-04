use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use bytes::Bytes;
use hmac::{Hmac, Mac};
use serde_json::{json, Value};
use sha2::Sha256;

use crate::AppState;

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, thiserror::Error)]
pub enum WebhookError {
    #[error("missing signature")]
    MissingSignature,
    #[error("invalid signature")]
    InvalidSignature,
    #[error("missing delivery id")]
    MissingDelivery,
    #[error("already processed")]
    AlreadyProcessed,
    #[error("database error: {0}")]
    Database(String),
}

impl IntoResponse for WebhookError {
    fn into_response(self) -> axum::response::Response {
        let (status, msg) = match &self {
            Self::MissingSignature | Self::InvalidSignature => {
                (StatusCode::UNAUTHORIZED, self.to_string())
            }
            Self::MissingDelivery => (StatusCode::BAD_REQUEST, self.to_string()),
            Self::AlreadyProcessed => (StatusCode::CONFLICT, self.to_string()),
            Self::Database(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
        };
        let body = Json(json!({ "error": msg }));
        (status, body).into_response()
    }
}

/// Verify `X-Hub-Signature-256` = `sha256=` + hex(HMAC_SHA256(payload, secret)).
///
/// Uses `hmac` crate's constant-time `verify_slice` (subtle).
pub fn verify_signature(payload: &[u8], signature_header: &str, secret: &str) -> bool {
    let Some(hex_part) = signature_header.strip_prefix("sha256=") else {
        return false;
    };
    let Ok(expected) = hex::decode(hex_part) else {
        return false;
    };
    let Ok(mut mac) = HmacSha256::new_from_slice(secret.as_bytes()) else {
        return false;
    };
    mac.update(payload);
    mac.verify_slice(&expected).is_ok()
}

/// Helper to compute signature for tests / examples.
#[allow(dead_code)]
pub fn compute_signature(payload: &[u8], secret: &str) -> String {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).expect("valid key length");
    mac.update(payload);
    let result = mac.finalize().into_bytes();
    format!("sha256={}", hex::encode(result))
}

/// POST /api/v1/webhooks/github
///
/// - HMAC check via `X-Hub-Signature-256`
/// - Idempotency via `X-GitHub-Delivery` + `webhook_deliveries` PK
/// - Only `release` + `published` is queued, others are 202 ignored
/// - Returns 202 `{status:"accepted", delivery_id}`, 401, 409
pub async fn github_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    let signature = headers
        .get("x-hub-signature-256")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if signature.is_empty() {
        return WebhookError::MissingSignature.into_response();
    }

    let secret = std::env::var("WEBHOOK_SECRET").unwrap_or_default();
    if secret.is_empty() {
        tracing::warn!("WEBHOOK_SECRET not set, rejecting webhook");
        return WebhookError::InvalidSignature.into_response();
    }

    if !verify_signature(&body, signature, &secret) {
        return WebhookError::InvalidSignature.into_response();
    }

    let delivery_id = headers
        .get("x-github-delivery")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    if delivery_id.is_empty() {
        return WebhookError::MissingDelivery.into_response();
    }

    let event = headers
        .get("x-github-event")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    let payload: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
    let action = payload
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    // Idempotency: INSERT ... ON CONFLICT DO NOTHING
    let insert = sqlx::query(
        "INSERT INTO webhook_deliveries (delivery_id, event, action, payload) \
         VALUES ($1, $2, $3, $4::jsonb) ON CONFLICT (delivery_id) DO NOTHING",
    )
    .bind(&delivery_id)
    .bind(&event)
    .bind(&action)
    .bind(&payload)
    .execute(&state.db)
    .await;

    match insert {
        Ok(res) if res.rows_affected() == 0 => {
            return WebhookError::AlreadyProcessed.into_response();
        }
        Err(e) => {
            tracing::error!(delivery_id = %delivery_id, error = %e, "webhook db insert failed");
            return WebhookError::Database(e.to_string()).into_response();
        }
        _ => {}
    }

    // Non-release events are accepted but ignored (no queue push).
    if event != "release" || action != "published" {
        tracing::info!(delivery_id = %delivery_id, event = %event, action = %action, "webhook ignored (not release.published)");
        let body = Json(json!({ "status": "accepted", "delivery_id": delivery_id }));
        return (StatusCode::ACCEPTED, body).into_response();
    }

    // Stub for Redis Streams push - in future: XADD indexium:webhook ...
    tracing::info!(delivery_id = %delivery_id, event = %event, "webhook accepted, push to redis (stub)");

    let body = Json(json!({ "status": "accepted", "delivery_id": delivery_id }));
    (StatusCode::ACCEPTED, body).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_signature_ok() {
        let secret = "test_secret_123";
        let payload = br#"{"action":"published"}"#;
        let sig = compute_signature(payload, secret);
        assert!(verify_signature(payload, &sig, secret));
    }

    #[test]
    fn invalid_signature_rejected() {
        let secret = "test_secret_123";
        let payload = br#"{"action":"published"}"#;
        let sig = compute_signature(payload, secret);
        // Tamper payload
        assert!(!verify_signature(br#"{"action":"tampered"}"#, &sig, secret));
        // Wrong secret
        assert!(!verify_signature(payload, &sig, "wrong_secret"));
        // Malformed header
        assert!(!verify_signature(payload, "sha256=zzzz", secret));
        assert!(!verify_signature(payload, "invalid", secret));
    }
}
