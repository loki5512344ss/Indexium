use axum::{
    extract::{Path, Query, State},
    http::{header, HeaderValue, StatusCode},
    response::IntoResponse,
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use crate::{db, AppState};

#[derive(Debug, Clone, Deserialize)]
pub struct ModSearchParams {
    pub query: Option<String>,
    #[serde(rename = "gameVersion")] pub game_version: Option<String>,
    pub loader: Option<String>,
    pub page: Option<i64>,
    pub limit: Option<i64>,
    pub sort: Option<ModSort>,
}
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ModSort { Relevance, Newest, Popular, ActiveServers }
impl Default for ModSort { fn default() -> Self { Self::Relevance } }
fn sort_to_sql(s: ModSort) -> &'static str {
    match s {
        ModSort::Relevance => "mods.updated_at DESC, mods.slug ASC",
        ModSort::Newest => "mods.updated_at DESC, mods.slug ASC",
        ModSort::Popular => "mods.updated_at DESC, mods.slug ASC",
        ModSort::ActiveServers => "mods.updated_at DESC, mods.slug ASC",
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModListItem {
    pub slug: String, pub name: String, pub summary: Option<String>,
    pub author: String, pub icon_url: Option<String>,
    pub game_versions: Vec<String>, pub loaders: Vec<String>,
    pub latest_version: Option<String>, pub download_url: Option<String>,
    pub updated_at: DateTime<Utc>,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct Pagination { pub page: i64, pub limit: i64, pub total: i64, pub pages: i64 }
#[derive(Debug, Serialize, Deserialize)]
pub struct ModListResponse { pub data: Vec<ModListItem>, pub pagination: Pagination }

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthorDto { pub login: String, pub avatar_url: Option<String> }
#[derive(Debug, Serialize, Deserialize)]
pub struct VersionDto {
    pub version_number: String, pub game_versions: Vec<String>, pub loaders: Vec<String>,
    pub download_url: String, pub file_sha256: String, pub file_size: Option<i64>,
    pub published_at: DateTime<Utc>,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct ModDetailResponse {
    pub slug: String, pub name: String, pub summary: Option<String>,
    pub description: Option<String>, pub github_repo: String,
    pub author: AuthorDto, pub icon_url: Option<String>, pub verified: bool,
    pub versions: Vec<VersionDto>, pub updated_at: DateTime<Utc>,
}

pub async fn list_mods(State(state): State<AppState>, Query(p): Query<ModSearchParams>) -> impl IntoResponse {
    let page = p.page.unwrap_or(1);
    let limit = p.limit.unwrap_or(20);
    let sort = p.sort.unwrap_or_default();
    if page < 1 {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error":"validation_error","message":"page must be >=1"}))).into_response();
    }
    if !(1..=50).contains(&limit) {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error":"validation_error","message":"limit must be 1..50"}))).into_response();
    }
    let q = p.query.as_deref().filter(|s| !s.trim().is_empty());
    let gv = p.game_version.as_deref().filter(|s| !s.trim().is_empty());
    let loader = p.loader.as_deref().filter(|s| !s.trim().is_empty());
    let offset = (page - 1) * limit;
    let sort_sql = sort_to_sql(sort);
    let total = match db::count_mods(&state.db, q, gv, loader).await {
        Ok(v) => v,
        Err(e) => {
            tracing::error!(error=%e,"count_mods failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error":"internal_error"}))).into_response();
        }
    };
    let rows = match db::fetch_mods_page(&state.db, q, gv, loader, sort_sql, limit, offset).await {
        Ok(v) => v,
        Err(e) => {
            tracing::error!(error=%e,"fetch_mods_page failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error":"internal_error"}))).into_response();
        }
    };
    let data = rows.into_iter().map(|r| ModListItem {
        slug: r.slug, name: r.name, summary: r.summary, author: r.author, icon_url: r.icon_url,
        game_versions: r.game_versions.unwrap_or_default(), loaders: r.loaders.unwrap_or_default(),
        latest_version: r.latest_version, download_url: r.download_url, updated_at: r.updated_at,
    }).collect::<Vec<_>>();
    let pages = if total == 0 { 0 } else { (total + limit - 1) / limit };
    let body = ModListResponse { data, pagination: Pagination { page, limit, total, pages } };
    let mut res = (StatusCode::OK, Json(body)).into_response();
    res.headers_mut().insert(header::CACHE_CONTROL, HeaderValue::from_static("public, max-age=60"));
    res
}

pub async fn get_mod(State(state): State<AppState>, Path(slug): Path<String>) -> impl IntoResponse {
    let m = match db::fetch_mod_by_slug(&state.db, &slug).await {
        Ok(v) => v,
        Err(e) => {
            tracing::error!(error=%e, slug=%slug,"fetch_mod_by_slug failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error":"internal_error"}))).into_response();
        }
    };
    let Some(m) = m else {
        return (StatusCode::NOT_FOUND, Json(serde_json::json!({"error":"mod_not_found"}))).into_response();
    };
    let versions = match db::fetch_versions_for_mod(&state.db, m.id).await {
        Ok(v) => v,
        Err(e) => {
            tracing::error!(error=%e, slug=%slug,"fetch_versions_for_mod failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error":"internal_error"}))).into_response();
        }
    };
    let versions_dto = versions.into_iter().map(|v| VersionDto {
        version_number: v.version_number, game_versions: v.game_versions, loaders: v.loaders,
        download_url: v.download_url, file_sha256: v.file_sha256, file_size: None, published_at: v.published_at,
    }).collect::<Vec<_>>();
    let resp = ModDetailResponse {
        slug: m.slug.clone(), name: m.name, summary: m.summary.clone(), description: m.summary,
        github_repo: format!("{}/{}", m.owner, m.repo),
        author: AuthorDto { login: m.owner, avatar_url: None },
        icon_url: m.icon_url, verified: false, versions: versions_dto, updated_at: m.updated_at,
    };
    let mut res = (StatusCode::OK, Json(resp)).into_response();
    res.headers_mut().insert(header::CACHE_CONTROL, HeaderValue::from_static("public, max-age=60"));
    res
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    #[test]
    fn dto_serialize_smoke() {
        let list = ModListResponse {
            data: vec![ModListItem {
                slug: "sodium-extra".into(), name: "Sodium Extra".into(),
                summary: Some("Extra".into()), author: "flashy".into(),
                icon_url: None, game_versions: vec!["1.20.1".into()], loaders: vec!["fabric".into()],
                latest_version: Some("1.2.3".into()), download_url: Some("https://example.com/mod.jar".into()),
                updated_at: Utc::now(),
            }],
            pagination: Pagination { page: 1, limit: 20, total: 1, pages: 1 },
        };
        let j = serde_json::to_value(&list).unwrap();
        assert_eq!(j["data"][0]["slug"], "sodium-extra");
        assert_eq!(j["pagination"]["total"], 1);
        let detail = ModDetailResponse {
            slug: "sodium-extra".into(), name: "Sodium Extra".into(), summary: Some("s".into()),
            description: Some("desc".into()), github_repo: "owner/repo".into(),
            author: AuthorDto { login: "flashy".into(), avatar_url: None },
            icon_url: None, verified: false,
            versions: vec![VersionDto {
                version_number: "1.2.3".into(), game_versions: vec!["1.20.1".into()], loaders: vec!["fabric".into()],
                download_url: "https://example.com/mod.jar".into(), file_sha256: "abc".into(), file_size: Some(123), published_at: Utc::now(),
            }],
            updated_at: Utc::now(),
        };
        let jd = serde_json::to_value(&detail).unwrap();
        assert_eq!(jd["versions"][0]["version_number"], "1.2.3");
        // sort enum deserialize
        let p: ModSearchParams = serde_json::from_value(serde_json::json!({"sort":"newest","page":2})).unwrap();
        assert_eq!(p.sort, Some(ModSort::Newest));
    }
}
