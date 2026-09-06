use chrono::{Duration, Utc};
use serde_json::json;
use sqlx::PgPool;
use std::collections::HashMap;
use uuid::Uuid;

/// Row for latest ping per (mod_id, server_hash) for yesterday.
#[derive(Debug, sqlx::FromRow)]
struct LatestPing {
    mod_id: Uuid,
    server_hash: String,
    mc_version: String,
    loader: String,
    os: String,
    java_version: String,
    player_count: i32,
}

fn inc(map: &mut HashMap<String, i32>, key: &str) {
    *map.entry(key.to_string()).or_insert(0) += 1;
}

/// Агрегирует пинги за вчера по mod_id.
///
/// - `active_servers` = COUNT(DISTINCT server_hash)
/// - `active_players` = SUM последнего player_count per server_hash
/// - `breakdown_json` = {mc_versions, loaders, os, java_counts}
/// Делает UPSERT в `mod_daily_stats`.
pub async fn aggregate_daily(pool: &PgPool) -> Result<(), sqlx::Error> {
    let yesterday = Utc::now().date_naive() - Duration::days(1);
    let start = yesterday.and_hms_opt(0, 0, 0).unwrap().and_utc();
    let end = start + Duration::days(1);

    // DISTINCT ON (mod_id, server_hash) -> последний пинг per сервер за вчера
    let rows = sqlx::query_as::<_, LatestPing>(
        r#"
        SELECT DISTINCT ON (mod_id, server_hash)
            mod_id, server_hash, mc_version, loader, os, java_version, player_count
        FROM mod_telemetry_pings
        WHERE pinged_at >= $1 AND pinged_at < $2
        ORDER BY mod_id, server_hash, pinged_at DESC
        "#,
    )
    .bind(start)
    .bind(end)
    .fetch_all(pool)
    .await?;

    if rows.is_empty() {
        tracing::info!(date = %yesterday, "no telemetry to aggregate");
        return Ok(());
    }

    let mut by_mod: HashMap<Uuid, Vec<LatestPing>> = HashMap::new();
    for r in rows {
        by_mod.entry(r.mod_id).or_default().push(r);
    }

    for (mod_id, pings) in by_mod {
        let active_servers = pings.len() as i32;
        let active_players: i32 = pings.iter().map(|p| p.player_count).sum();

        let mut mc_versions: HashMap<String, i32> = HashMap::new();
        let mut loaders: HashMap<String, i32> = HashMap::new();
        let mut os_counts: HashMap<String, i32> = HashMap::new();
        let mut java_counts: HashMap<String, i32> = HashMap::new();

        for p in &pings {
            inc(&mut mc_versions, &p.mc_version);
            inc(&mut loaders, &p.loader);
            inc(&mut os_counts, &p.os);
            inc(&mut java_counts, &p.java_version);
        }

        let breakdown = json!({
            "mc_versions": mc_versions,
            "loaders": loaders,
            "os": os_counts,
            "java": java_counts,
            "java_counts": java_counts,
        });

        sqlx::query(
            r#"
            INSERT INTO mod_daily_stats (mod_id, date, active_servers, active_players, breakdown_json)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (mod_id, date) DO UPDATE SET
                active_servers = EXCLUDED.active_servers,
                active_players = EXCLUDED.active_players,
                breakdown_json = EXCLUDED.breakdown_json
            "#,
        )
        .bind(mod_id)
        .bind(yesterday)
        .bind(active_servers)
        .bind(active_players)
        .bind(&breakdown)
        .execute(pool)
        .await?;
    }

    tracing::info!(date = %yesterday, "analytics aggregation completed");
    Ok(())
}

/// Удаляет пинги старше 30 дней (TTL).
pub async fn cleanup_old_pings(pool: &PgPool) -> Result<(), sqlx::Error> {
    let res = sqlx::query(
        "DELETE FROM mod_telemetry_pings WHERE pinged_at < NOW() - INTERVAL '30 days'",
    )
    .execute(pool)
    .await?;

    let deleted = res.rows_affected();
    if deleted > 0 {
        tracing::info!(deleted = deleted, "cleaned old telemetry pings");
    }
    Ok(())
}
