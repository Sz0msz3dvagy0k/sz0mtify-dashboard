mod api;
mod db;
mod services;
mod utils;

use std::{env, net::SocketAddr, sync::Arc};

use axum::{
    routing::{get, post},
    Router,
};
use db::migrate;
use services::{
    analytics::AnalyticsService, discovery::DiscoveryService,
    recommendation::RecommendationService, settings::SettingsService, sync::SyncService, AppState,
};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt().with_env_filter("info").init();

    let database_url =
        env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://music-dashboard.db".into());
    let pool = sqlx::SqlitePool::connect(&database_url).await?;
    migrate(&pool).await?;

    let state = Arc::new(AppState {
        pool: pool.clone(),
        sync: SyncService::new(),
        analytics: AnalyticsService,
        discovery: DiscoveryService,
        recommendations: RecommendationService,
        settings: SettingsService,
    });

    let app = Router::new()
        .route("/api/health", get(api::handlers::health))
        .route(
            "/api/settings",
            get(api::handlers::get_settings).post(api::handlers::save_settings),
        )
        .route("/api/sync/subsonic", post(api::handlers::sync_subsonic))
        .route("/api/sync/lastfm", post(api::handlers::sync_lastfm))
        .route("/api/sync/all", post(api::handlers::sync_all))
        .route("/api/sync/status", get(api::handlers::sync_status))
        .route(
            "/api/library/overview",
            get(api::handlers::library_overview),
        )
        .route("/api/library/tracks", get(api::handlers::tracks))
        .route("/api/library/albums", get(api::handlers::albums))
        .route("/api/library/albums/:id", get(api::handlers::album_by_id))
        .route("/api/library/artists", get(api::handlers::artists))
        .route("/api/library/artists/:id", get(api::handlers::artist_by_id))
        .route("/api/library/genres", get(api::handlers::genres))
        .route(
            "/api/stats/audio-quality",
            get(api::handlers::audio_quality),
        )
        .route("/api/stats/storage", get(api::handlers::storage))
        .route(
            "/api/stats/metadata-health",
            get(api::handlers::metadata_health),
        )
        .route("/api/stats/listening", get(api::handlers::listening))
        .route("/api/stats/timeline", get(api::handlers::timeline))
        .route(
            "/api/discovery/new-releases",
            get(api::handlers::new_releases),
        )
        .route(
            "/api/discovery/missing-albums",
            get(api::handlers::missing_albums),
        )
        .route(
            "/api/discovery/similar-artists",
            get(api::handlers::similar_artists),
        )
        .route(
            "/api/discovery/refresh",
            post(api::handlers::refresh_discovery),
        )
        .route(
            "/api/recommendations/rediscovery",
            get(api::handlers::rediscovery),
        )
        .route(
            "/api/recommendations/current-rotation",
            get(api::handlers::current_rotation),
        )
        .route(
            "/api/recommendations/favorites",
            get(api::handlers::favorites),
        )
        .route("/api/search", get(api::handlers::search))
        .route("/api/cover/:id", get(api::handlers::cover))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let host = env::var("BACKEND_HOST").unwrap_or_else(|_| "0.0.0.0".into());
    let port = env::var("BACKEND_PORT").unwrap_or_else(|_| "8080".into());
    let addr: SocketAddr = format!("{host}:{port}").parse()?;
    info!("backend listening on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
