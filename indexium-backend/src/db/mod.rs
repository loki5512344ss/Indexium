use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Row structs (sqlx::FromRow)
// ---------------------------------------------------------------------------

#[derive(Debug, sqlx::FromRow)]
pub struct ModListRow {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub summary: Option<String>,
    pub author: String,
    pub icon_url: Option<String>,
    pub updated_at: DateTime<Utc>,
    pub latest_version: Option<String>,
    pub game_versions: Option<Vec<String>>,
    pub loaders: Option<Vec<String>>,
    pub download_url: Option<String>,
    pub stars_count: i64,
}

#[derive(Debug, sqlx::FromRow)]
pub struct ModRow {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub summary: Option<String>,
    pub owner: String,
    pub repo: String,
    pub icon_url: Option<String>,
    pub updated_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub stars_count: i64,
}

#[derive(Debug, sqlx::FromRow)]
pub struct VersionRow {
    pub version_number: String,
    pub game_versions: Vec<String>,
    pub loaders: Vec<String>,
    pub download_url: String,
    pub file_sha256: String,
    pub published_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

pub async fn count_mods(
    pool: &PgPool,
    query: Option<&str>,
    game_version: Option<&str>,
    loader: Option<&str>,
) -> Result<i64, sqlx::Error> {
    let row: (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*)
        FROM mods
        WHERE ($1::text IS NULL OR mods.name ILIKE '%' || $1 || '%' OR mods.summary ILIKE '%' || $1 || '%')
          AND ($2::text IS NULL OR EXISTS (
                SELECT 1 FROM mod_versions v
                WHERE v.mod_id = mods.id AND $2 = ANY(v.game_versions)
          ))
          AND ($3::text IS NULL OR EXISTS (
                SELECT 1 FROM mod_versions v2
                WHERE v2.mod_id = mods.id AND $3 = ANY(v2.loaders)
          ))
        "#,
    )
    .bind(query)
    .bind(game_version)
    .bind(loader)
    .fetch_one(pool)
    .await?;
    Ok(row.0)
}

pub async fn fetch_mods_page(
    pool: &PgPool,
    query: Option<&str>,
    game_version: Option<&str>,
    loader: Option<&str>,
    sort_sql: &str,
    limit: i64,
    offset: i64,
) -> Result<Vec<ModListRow>, sqlx::Error> {
    // sort_sql is validated enum -> safe to interpolate
    let sql = format!(
        r#"
        SELECT
            mods.id,
            mods.slug,
            mods.name,
            mods.summary,
            mods.owner AS author,
            mods.icon_url,
            mods.updated_at,
            lv.version_number AS latest_version,
            lv.game_versions,
            lv.loaders,
            lv.download_url,
            (SELECT COUNT(*) FROM stars s WHERE s.mod_id = mods.id) AS stars_count
        FROM mods
        LEFT JOIN LATERAL (
            SELECT version_number, game_versions, loaders, download_url
            FROM mod_versions
            WHERE mod_versions.mod_id = mods.id
            ORDER BY published_at DESC
            LIMIT 1
        ) lv ON true
        WHERE ($1::text IS NULL OR mods.name ILIKE '%' || $1 || '%' OR mods.summary ILIKE '%' || $1 || '%')
          AND ($2::text IS NULL OR EXISTS (
                SELECT 1 FROM mod_versions v
                WHERE v.mod_id = mods.id AND $2 = ANY(v.game_versions)
          ))
          AND ($3::text IS NULL OR EXISTS (
                SELECT 1 FROM mod_versions v2
                WHERE v2.mod_id = mods.id AND $3 = ANY(v2.loaders)
          ))
        ORDER BY {sort_sql}
        LIMIT $4 OFFSET $5
        "#
    );

    let rows = sqlx::query_as::<_, ModListRow>(sqlx::AssertSqlSafe(sql))
        .bind(query)
        .bind(game_version)
        .bind(loader)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?;
    Ok(rows)
}

pub async fn fetch_mod_by_slug(pool: &PgPool, slug: &str) -> Result<Option<ModRow>, sqlx::Error> {
    let row = sqlx::query_as::<_, ModRow>(
        r#"SELECT id, slug, name, summary, owner, repo, icon_url, updated_at, created_at,
                  (SELECT COUNT(*) FROM stars s WHERE s.mod_id = mods.id) AS stars_count
           FROM mods WHERE slug = $1"#,
    )
    .bind(slug)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

pub async fn fetch_versions_for_mod(
    pool: &PgPool,
    mod_id: Uuid,
) -> Result<Vec<VersionRow>, sqlx::Error> {
    let rows = sqlx::query_as::<_, VersionRow>(
        r#"SELECT version_number, game_versions, loaders, download_url, file_sha256, published_at
           FROM mod_versions WHERE mod_id = $1 ORDER BY published_at DESC"#,
    )
    .bind(mod_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}
