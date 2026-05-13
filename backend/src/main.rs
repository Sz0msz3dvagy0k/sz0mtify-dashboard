mod api;
mod auth;
mod db;
mod services;
mod utils;

use std::{collections::HashSet, env, net::SocketAddr, sync::Arc, time::Duration};

use axum::{
    http::{header, HeaderValue, Method},
    middleware,
    routing::{get, post},
    Router,
};
use db::migrate;
use services::{
    analytics::AnalyticsService, discovery::DiscoveryService,
    recommendation::RecommendationService, settings::SettingsService, sync::SyncService, AppState,
};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use std::str::FromStr;
use tower_http::{
    cors::{AllowOrigin, CorsLayer},
    trace::TraceLayer,
};
use tracing::info;
use tracing_subscriber::EnvFilter;

fn ensure_sqlite_parent_dir(database_url: &str) -> anyhow::Result<()> {
    if !database_url.starts_with("sqlite:") {
        return Ok(());
    }

    let raw_path = database_url
        .trim_start_matches("sqlite://")
        .trim_start_matches("sqlite:");

    if raw_path.is_empty() || raw_path == ":memory:" {
        return Ok(());
    }

    let db_path = raw_path.split('?').next().unwrap_or(raw_path);
    let path = std::path::Path::new(db_path);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    Ok(())
}

async fn connect_sqlite_pool(database_url: &str) -> anyhow::Result<sqlx::SqlitePool> {
    let options = SqliteConnectOptions::from_str(database_url)?
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .busy_timeout(Duration::from_secs(5))
        .foreign_keys(true);
    Ok(SqlitePoolOptions::new()
        .max_connections(sqlite_pool_max_connections())
        .connect_with(options)
        .await?)
}

fn sqlite_pool_max_connections() -> u32 {
    env::var("SQLITE_MAX_CONNECTIONS")
        .ok()
        .and_then(|value| value.parse().ok())
        .filter(|value| (1..=32).contains(value))
        .unwrap_or(5)
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("backend startup failed: {error:#}");
        std::process::exit(1);
    }
}

async fn run() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let log_filter = env::var("RUST_LOG").unwrap_or_else(|_| {
        "debug,tower_http=debug,sqlx=info,music_listening_dashboard=debug".into()
    });
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new(log_filter))
        .with_target(true)
        .init();

    let database_url =
        env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://music-dashboard.db".into());
    ensure_sqlite_parent_dir(&database_url)?;
    let pool = connect_sqlite_pool(&database_url).await?;
    migrate(&pool).await?;
    let sync = SyncService::new();
    let auth = auth::AppAuth::from_env()?;
    let normalized_artist_credits = sync.normalize_artist_credits(&pool).await?;
    if normalized_artist_credits > 0 {
        info!(
            normalized_artist_credits,
            "normalized existing artist credits"
        );
    }

    let state = Arc::new(AppState {
        auth,
        pool: pool.clone(),
        sync,
        sync_jobs: Arc::new(tokio::sync::Mutex::new(HashSet::new())),
        analytics: AnalyticsService,
        discovery: DiscoveryService,
        recommendations: RecommendationService,
        settings: SettingsService,
    });

    let app = Router::new()
        .route("/api/health", get(api::handlers::health))
        .route("/api/auth/login", post(api::handlers::login))
        .route("/api/auth/logout", post(api::handlers::logout))
        .route("/api/auth/me", get(api::handlers::me))
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
        .route("/api/playlists", get(api::handlers::playlists))
        .route("/api/playlists/:id", get(api::handlers::playlist_by_id))
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
        .route("/api/search", get(api::handlers::search))
        .route(
            "/api/tracks/:track_id/stream",
            get(api::handlers::stream_track),
        )
        .route(
            "/api/tracks/:track_id/now-playing",
            post(api::handlers::track_now_playing),
        )
        .route(
            "/api/artist-image/:artist_id",
            get(api::handlers::artist_image),
        )
        .route("/api/cover/:cover_art_id", get(api::handlers::cover))
        .layer(middleware::from_fn_with_state(
            state.auth.clone(),
            auth::require_app_session,
        ));

    let app = app
        .layer(TraceLayer::new_for_http())
        .layer(cors_layer())
        .with_state(state);

    let host = env::var("BACKEND_HOST").unwrap_or_else(|_| "0.0.0.0".into());
    let port = env::var("BACKEND_PORT").unwrap_or_else(|_| "8080".into());
    let addr: SocketAddr = format!("{host}:{port}").parse()?;
    info!("backend listening on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

fn cors_layer() -> CorsLayer {
    CorsLayer::new()
        .allow_origin(AllowOrigin::list(frontend_allowed_origins()))
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE, header::RANGE])
        .expose_headers([
            header::ACCEPT_RANGES,
            header::CONTENT_LENGTH,
            header::CONTENT_RANGE,
            header::CONTENT_TYPE,
            header::ETAG,
            header::LAST_MODIFIED,
        ])
        .allow_credentials(true)
        .max_age(Duration::from_secs(3600))
}

fn frontend_allowed_origins() -> Vec<HeaderValue> {
    let defaults = [
        "http://localhost:5173",
        "http://127.0.0.1:5173",
        "http://localhost:3000",
        "http://127.0.0.1:3000",
        "capacitor://localhost",
        "ionic://localhost",
    ];

    env::var("FRONTEND_ALLOWED_ORIGINS")
        .unwrap_or_default()
        .split(',')
        .chain(defaults)
        .filter_map(normalize_cors_origin)
        .filter_map(|origin| origin.parse().ok())
        .collect()
}

fn normalize_cors_origin(value: &str) -> Option<String> {
    let origin = value.trim().trim_end_matches('/');
    if origin.is_empty() {
        return None;
    }

    let scheme_end = origin.find("://")?;
    let authority_start = scheme_end + 3;
    let authority_end = origin[authority_start..]
        .find('/')
        .map(|index| authority_start + index)
        .unwrap_or(origin.len());

    Some(origin[..authority_end].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn sqlite_pool_creates_missing_database_file() {
        let db_path = std::env::temp_dir()
            .join("music-dashboard-tests")
            .join(format!("{}.db", uuid::Uuid::new_v4()));
        let database_url = format!("sqlite://{}", db_path.display());

        ensure_sqlite_parent_dir(&database_url).expect("parent directory should be created");
        let pool = connect_sqlite_pool(&database_url)
            .await
            .expect("missing sqlite database should be created");
        pool.close().await;

        assert!(db_path.exists());

        let _ = std::fs::remove_file(db_path);
    }

    #[test]
    fn cors_origin_normalization_removes_paths_and_trailing_slashes() {
        assert_eq!(
            normalize_cors_origin("https://kaori.szomszed.me/").as_deref(),
            Some("https://kaori.szomszed.me")
        );
        assert_eq!(
            normalize_cors_origin(" https://kaori.szomszed.me/login "),
            Some("https://kaori.szomszed.me".to_string())
        );
        assert_eq!(
            normalize_cors_origin("capacitor://localhost/").as_deref(),
            Some("capacitor://localhost")
        );
    }
}
