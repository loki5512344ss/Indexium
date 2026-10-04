use axum::{
    extract::State,
    http::{HeaderValue, Method},
    routing::{get, post},
    Json, Router,
};
use dotenvy::dotenv;
use serde_json::{json, Value};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::env;
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod api;
mod db;
mod services;
mod worker;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set in .env");
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&db_url)
        .await?;

    // Автоматический запуск миграций при старте
    sqlx::migrate!("./migrations").run(&pool).await?;
    tracing::info!("Database migrations applied successfully");

    // Крон агрегации телеметрии: каждый час aggregate_daily + cleanup_old_pings
    {
        let cron_pool = pool.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(3600));
            loop {
                interval.tick().await;
                if let Err(e) = services::analytics_agg::aggregate_daily(&cron_pool).await {
                    tracing::error!(error = %e, "analytics aggregation failed");
                }
                if let Err(e) = services::analytics_agg::cleanup_old_pings(&cron_pool).await {
                    tracing::error!(error = %e, "cleanup old pings failed");
                }
            }
        });
    }

    let cors = CorsLayer::new()
        .allow_origin("http://localhost:5173".parse::<HeaderValue>()?)
        .allow_methods([Method::GET, Method::POST, Method::DELETE, Method::OPTIONS])
        .allow_headers([axum::http::header::CONTENT_TYPE, axum::http::header::HeaderName::from_static("x-github-id")]);

    let state = AppState { db: pool };

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/api/v1/mods", get(api::mods::list_mods))
        .route("/api/v1/mods/{slug}", get(api::mods::get_mod))
        .route("/api/v1/mods/{slug}/stars", get(api::stars::get_stars))
        .route("/api/v1/mods/{slug}/star", post(api::stars::star_mod).delete(api::stars::unstar_mod))
        .route("/api/v1/collections", get(api::collections::list_collections).post(api::collections::create_collection))
        .route("/api/v1/collections/{slug}", get(api::collections::get_collection))
        .route("/api/v1/collections/{slug}/export", get(api::collections::export_collection))
        .route(
            "/api/v1/badges/{slug}/downloads.svg",
            get(api::badges::get_downloads_badge),
        )
        .route(
            "/api/v1/badges/{slug}/servers.svg",
            get(api::badges::get_servers_badge),
        )
        .route(
            "/api/v1/webhooks/github",
            post(api::webhooks::github_webhook),
        )
        .route("/api/v1/auth/github", get(api::auth::github_login))
        .route("/api/v1/auth/github/callback", get(api::auth::github_callback))
        .route("/api/v1/auth/me", get(api::auth::me))
        .route("/api/v1/auth/logout", post(api::auth::logout))
        .route(
            "/api/v1/analytics/submit",
            post(api::analytics::submit),
        )
        .route(
            "/api/v1/mods/{slug}/analytics",
            get(api::analytics::get_analytics),
        )
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state);

    let port: u16 = env::var("SERVER_PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse()?;

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    tracing::info!("Indexium backend listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn health_check(State(state): State<AppState>) -> Json<Value> {
    let db_status = match sqlx::query("SELECT 1").execute(&state.db).await {
        Ok(_) => "ok",
        Err(_) => "error",
    };

    Json(json!({
        "status": "online",
        "database": db_status
    }))
}
