use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::AppState;

// ---------------------------------------------------------------------------
// DTOs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionMod {
    pub slug: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Collection {
    pub id: Uuid,
    pub slug: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub mods: Vec<CollectionMod>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author_id: Option<i64>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateCollectionRequest {
    pub slug: String,
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub mods: Vec<CollectionMod>,
    #[serde(default)]
    pub author_id: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct ExportQuery {
    pub format: Option<String>,
}

// Prism Launcher export format (minimal)
#[derive(Debug, Serialize)]
struct PrismExport {
    #[serde(rename = "formatVersion")]
    format_version: u8,
    name: String,
    summary: Option<String>,
    components: Vec<PrismComponent>,
}

#[derive(Debug, Serialize)]
struct PrismComponent {
    uid: String,
    version: Option<String>,
}

// ---------------------------------------------------------------------------
// DB row
// ---------------------------------------------------------------------------
#[derive(Debug, sqlx::FromRow)]
struct CollectionRow {
    id: Uuid,
    slug: String,
    title: String,
    description: Option<String>,
    mods: serde_json::Value,
    author_id: Option<i64>,
    created_at: DateTime<Utc>,
}

fn row_to_collection(r: CollectionRow) -> Collection {
    let mods: Vec<CollectionMod> = serde_json::from_value(r.mods).unwrap_or_default();
    Collection {
        id: r.id,
        slug: r.slug,
        title: r.title,
        description: r.description,
        mods,
        author_id: r.author_id,
        created_at: r.created_at,
    }
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// GET /api/v1/collections
pub async fn list_collections(State(state): State<AppState>) -> impl IntoResponse {
    let rows = sqlx::query_as::<_, CollectionRow>(
        "SELECT id, slug, title, description, mods, author_id, created_at FROM collections ORDER BY created_at DESC",
    )
    .fetch_all(&state.db)
    .await;

    match rows {
        Ok(rows) => {
            let data: Vec<Collection> = rows.into_iter().map(row_to_collection).collect();
            (StatusCode::OK, Json(json!(data))).into_response()
        }
        Err(e) => {
            tracing::error!(error=%e, "list_collections failed");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error":"internal_error"}))).into_response()
        }
    }
}

/// POST /api/v1/collections - auth stub: без проверки токена, просто 201
pub async fn create_collection(
    State(state): State<AppState>,
    Json(req): Json<CreateCollectionRequest>,
) -> impl IntoResponse {
    if req.slug.is_empty() || req.slug.len() > 64 {
        return (StatusCode::BAD_REQUEST, Json(json!({"error":"validation_error","message":"slug 1..64"}))).into_response();
    }
    if req.title.is_empty() || req.title.len() > 128 {
        return (StatusCode::BAD_REQUEST, Json(json!({"error":"validation_error","message":"title 1..128"}))).into_response();
    }

    let id = Uuid::new_v4();
    let mods_json = serde_json::to_value(&req.mods).unwrap_or(json!([]));

    let res = sqlx::query(
        "INSERT INTO collections (id, slug, title, description, mods, author_id) VALUES ($1,$2,$3,$4,$5,$6)",
    )
    .bind(id)
    .bind(&req.slug)
    .bind(&req.title)
    .bind(&req.description)
    .bind(&mods_json)
    .bind(req.author_id)
    .execute(&state.db)
    .await;

    match res {
        Ok(_) => {
            let collection = Collection {
                id,
                slug: req.slug,
                title: req.title,
                description: req.description,
                mods: req.mods,
                author_id: req.author_id,
                created_at: Utc::now(),
            };
            (StatusCode::CREATED, Json(json!(collection))).into_response()
        }
        Err(e) if e.to_string().contains("duplicate") || e.to_string().contains("Unique") => {
            (StatusCode::CONFLICT, Json(json!({"error":"slug_conflict"}))).into_response()
        }
        Err(e) => {
            tracing::error!(error=%e, "create_collection failed");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error":"internal_error"}))).into_response()
        }
    }
}

/// GET /api/v1/collections/:slug
pub async fn get_collection(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> impl IntoResponse {
    let row = sqlx::query_as::<_, CollectionRow>(
        "SELECT id, slug, title, description, mods, author_id, created_at FROM collections WHERE slug=$1",
    )
    .bind(&slug)
    .fetch_optional(&state.db)
    .await;

    match row {
        Ok(Some(r)) => (StatusCode::OK, Json(json!(row_to_collection(r)))).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, Json(json!({"error":"collection_not_found"}))).into_response(),
        Err(e) => {
            tracing::error!(error=%e, slug=%slug, "get_collection failed");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error":"internal_error"}))).into_response()
        }
    }
}

/// GET /api/v1/collections/:slug/export?format=prism
pub async fn export_collection(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Query(q): Query<ExportQuery>,
) -> impl IntoResponse {
    let row = sqlx::query_as::<_, CollectionRow>(
        "SELECT id, slug, title, description, mods, author_id, created_at FROM collections WHERE slug=$1",
    )
    .bind(&slug)
    .fetch_optional(&state.db)
    .await;

    let collection = match row {
        Ok(Some(r)) => row_to_collection(r),
        Ok(None) => return (StatusCode::NOT_FOUND, Json(json!({"error":"collection_not_found"}))).into_response(),
        Err(e) => {
            tracing::error!(error=%e, "export_collection fetch failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error":"internal_error"}))).into_response();
        }
    };

    if q.format.as_deref() == Some("prism") {
        let prism = PrismExport {
            format_version: 1,
            name: collection.title.clone(),
            summary: collection.description.clone(),
            components: collection.mods.iter().map(|m| PrismComponent { uid: m.slug.clone(), version: m.version.clone() }).collect(),
        };
        return (StatusCode::OK, Json(json!(prism))).into_response();
    }

    (StatusCode::OK, Json(json!(collection))).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dto_serde() {
        let c = Collection {
            id: Uuid::new_v4(),
            slug: "my-pack".into(),
            title: "My Pack".into(),
            description: Some("desc".into()),
            mods: vec![CollectionMod { slug: "sodium".into(), version: Some("1.0.0".into()) }],
            author_id: Some(42),
            created_at: Utc::now(),
        };
        let v = serde_json::to_value(&c).unwrap();
        assert_eq!(v["slug"], "my-pack");
        assert_eq!(v["mods"][0]["slug"], "sodium");
        let prism = PrismExport { format_version: 1, name: "My Pack".into(), summary: None, components: vec![PrismComponent { uid: "sodium".into(), version: None }] };
        let pv = serde_json::to_value(&prism).unwrap();
        assert_eq!(pv["formatVersion"], 1);
    }
}
