use std::sync::Arc;

use axum::{
    body::Bytes,
    extract::{Path, Query, State},
    http::{
        header::{CACHE_CONTROL, CONTENT_TYPE, ETAG},
        HeaderMap, HeaderValue, StatusCode,
    },
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use tracing::{error, info};

use crate::services::discovery::{DiscoveryListOptions, DiscoveryRefreshOptions};
use crate::services::{sync::SubsonicConfig, AppState};

#[derive(Deserialize)]
pub struct SearchQ {
    pub q: String,
}

#[derive(Deserialize)]
pub struct DiscoveryQ {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub include_owned: Option<bool>,
}

#[derive(Deserialize)]
pub struct DiscoveryRefreshQ {
    pub limit: Option<i64>,
}

fn ok(data: Value) -> Json<Value> {
    Json(json!({"ok": true, "data": data}))
}

fn err(message: &str) -> Json<Value> {
    Json(json!({"ok": false, "error": message}))
}

pub async fn health(State(state): State<Arc<AppState>>) -> Json<Value> {
    let db_ok = sqlx::query("SELECT 1").execute(&state.pool).await.is_ok();
    Json(json!({"ok": db_ok, "status": if db_ok { "ok" } else { "degraded" }}))
}

pub async fn get_settings(State(state): State<Arc<AppState>>) -> Json<Value> {
    match state.settings.get_all(&state.pool).await {
        Ok(settings) => ok(json!(settings)),
        Err(_) => err("failed_to_load_settings"),
    }
}

pub async fn save_settings(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<Value>,
) -> Json<Value> {
    match state.settings.save(&state.pool, payload.clone()).await {
        Ok(_) => ok(payload),
        Err(_) => err("failed_to_save_settings"),
    }
}

pub async fn sync_subsonic(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    if !reserve_sync_jobs(&state, &["subsonic"]).await {
        return (StatusCode::CONFLICT, err("sync_already_running: subsonic"));
    }

    if let Err(error) = state.sync.mark_running(&state.pool, 1, "subsonic").await {
        release_sync_jobs(&state, &["subsonic"]).await;
        return (
            StatusCode::BAD_GATEWAY,
            err(&format!("failed_to_start_subsonic_sync: {error}")),
        );
    }

    let job_state = state.clone();
    tokio::spawn(async move {
        match job_state.sync.sync_subsonic(&job_state.pool).await {
            Ok(imported_tracks) => {
                info!(source = "subsonic", imported_tracks, "sync job completed");
            }
            Err(error) => {
                error!(source = "subsonic", error = %error, "sync job failed");
            }
        }
        release_sync_jobs(&job_state, &["subsonic"]).await;
    });

    (
        StatusCode::ACCEPTED,
        ok(json!({"source": "subsonic", "status": "started"})),
    )
}

pub async fn sync_lastfm(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    if !reserve_sync_jobs(&state, &["lastfm"]).await {
        return (StatusCode::CONFLICT, err("sync_already_running: lastfm"));
    }

    if let Err(error) = state.sync.mark_running(&state.pool, 2, "lastfm").await {
        release_sync_jobs(&state, &["lastfm"]).await;
        return (
            StatusCode::BAD_GATEWAY,
            err(&format!("failed_to_start_lastfm_sync: {error}")),
        );
    }

    let job_state = state.clone();
    tokio::spawn(async move {
        match job_state.sync.sync_lastfm(&job_state.pool).await {
            Ok(updated_artists) => {
                info!(source = "lastfm", updated_artists, "sync job completed");
            }
            Err(error) => {
                error!(source = "lastfm", error = %error, "sync job failed");
            }
        }
        release_sync_jobs(&job_state, &["lastfm"]).await;
    });

    (
        StatusCode::ACCEPTED,
        ok(json!({"source": "lastfm", "status": "started"})),
    )
}

pub async fn sync_all(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    if !reserve_sync_jobs(&state, &["subsonic", "lastfm"]).await {
        return (StatusCode::CONFLICT, err("sync_already_running"));
    }

    if let Err(error) = state.sync.mark_running(&state.pool, 1, "subsonic").await {
        release_sync_jobs(&state, &["subsonic", "lastfm"]).await;
        return (
            StatusCode::BAD_GATEWAY,
            err(&format!("failed_to_start_full_sync: {error}")),
        );
    }
    if let Err(error) = state.sync.mark_running(&state.pool, 2, "lastfm").await {
        release_sync_jobs(&state, &["subsonic", "lastfm"]).await;
        return (
            StatusCode::BAD_GATEWAY,
            err(&format!("failed_to_start_full_sync: {error}")),
        );
    }

    let job_state = state.clone();
    tokio::spawn(async move {
        match job_state.sync.sync_subsonic(&job_state.pool).await {
            Ok(imported_tracks) => {
                info!(
                    source = "subsonic",
                    imported_tracks, "full sync step completed"
                );
            }
            Err(error) => {
                error!(source = "subsonic", error = %error, "full sync step failed");
            }
        }

        match job_state.sync.sync_lastfm(&job_state.pool).await {
            Ok(updated_artists) => {
                info!(
                    source = "lastfm",
                    updated_artists, "full sync step completed"
                );
            }
            Err(error) => {
                error!(source = "lastfm", error = %error, "full sync step failed");
            }
        }
        release_sync_jobs(&job_state, &["subsonic", "lastfm"]).await;
    });

    (
        StatusCode::ACCEPTED,
        ok(json!({"source": "all", "status": "started"})),
    )
}

async fn reserve_sync_jobs(state: &Arc<AppState>, sources: &[&str]) -> bool {
    let mut running = state.sync_jobs.lock().await;
    if sources.iter().any(|source| running.contains(*source)) {
        return false;
    }
    for source in sources {
        running.insert((*source).to_string());
    }
    true
}

async fn release_sync_jobs(state: &Arc<AppState>, sources: &[&str]) {
    let mut running = state.sync_jobs.lock().await;
    for source in sources {
        running.remove(*source);
    }
}

pub async fn sync_status(State(state): State<Arc<AppState>>) -> Json<Value> {
    match state.sync.status(&state.pool).await {
        Ok(status) => ok(status),
        Err(_) => err("failed_to_load_sync_status"),
    }
}

macro_rules! respond_service {
    ($result:expr, $err:literal) => {
        match $result.await {
            Ok(v) => ok(v),
            Err(_) => err($err),
        }
    };
}

pub async fn library_overview(State(state): State<Arc<AppState>>) -> Json<Value> {
    respond_service!(
        state.analytics.overview(&state.pool),
        "failed_to_load_library_overview"
    )
}
pub async fn tracks(State(state): State<Arc<AppState>>) -> Json<Value> {
    respond_service!(state.analytics.tracks(&state.pool), "failed_to_load_tracks")
}
pub async fn albums(State(state): State<Arc<AppState>>) -> Json<Value> {
    respond_service!(state.analytics.albums(&state.pool), "failed_to_load_albums")
}
pub async fn album_by_id(State(state): State<Arc<AppState>>, Path(id): Path<i64>) -> Json<Value> {
    respond_service!(
        state.analytics.album_by_id(&state.pool, id),
        "failed_to_load_album"
    )
}
pub async fn artists(State(state): State<Arc<AppState>>) -> Json<Value> {
    respond_service!(
        state.analytics.artists(&state.pool),
        "failed_to_load_artists"
    )
}
pub async fn artist_by_id(State(state): State<Arc<AppState>>, Path(id): Path<i64>) -> Json<Value> {
    respond_service!(
        state.analytics.artist_by_id(&state.pool, id),
        "failed_to_load_artist"
    )
}
pub async fn genres(State(state): State<Arc<AppState>>) -> Json<Value> {
    respond_service!(state.analytics.genres(&state.pool), "failed_to_load_genres")
}
pub async fn audio_quality(State(state): State<Arc<AppState>>) -> Json<Value> {
    respond_service!(
        state.analytics.audio_quality(&state.pool),
        "failed_to_load_audio_quality"
    )
}
pub async fn storage(State(state): State<Arc<AppState>>) -> Json<Value> {
    respond_service!(
        state.analytics.storage(&state.pool),
        "failed_to_load_storage"
    )
}
pub async fn metadata_health(State(state): State<Arc<AppState>>) -> Json<Value> {
    respond_service!(
        state.analytics.metadata_health(&state.pool),
        "failed_to_load_metadata_health"
    )
}
pub async fn listening(State(state): State<Arc<AppState>>) -> Json<Value> {
    respond_service!(
        state.analytics.listening(&state.pool),
        "failed_to_load_listening_stats"
    )
}
pub async fn timeline(State(state): State<Arc<AppState>>) -> Json<Value> {
    respond_service!(
        state.analytics.timeline(&state.pool),
        "failed_to_load_timeline"
    )
}
pub async fn new_releases(
    State(state): State<Arc<AppState>>,
    Query(params): Query<DiscoveryQ>,
) -> Json<Value> {
    respond_service!(
        state
            .discovery
            .new_releases(&state.pool, discovery_options(params)),
        "failed_to_load_new_releases"
    )
}
pub async fn missing_albums(
    State(state): State<Arc<AppState>>,
    Query(params): Query<DiscoveryQ>,
) -> Json<Value> {
    respond_service!(
        state
            .discovery
            .missing_albums(&state.pool, discovery_options(params)),
        "failed_to_load_missing_albums"
    )
}
pub async fn similar_artists(
    State(state): State<Arc<AppState>>,
    Query(params): Query<DiscoveryQ>,
) -> Json<Value> {
    respond_service!(
        state
            .discovery
            .similar_artists(&state.pool, discovery_options(params)),
        "failed_to_load_similar_artists"
    )
}

pub async fn refresh_discovery(
    State(state): State<Arc<AppState>>,
    Query(params): Query<DiscoveryRefreshQ>,
) -> Json<Value> {
    let options = DiscoveryRefreshOptions {
        limit: params.limit.unwrap_or(50).clamp(1, 200),
    };
    match state.discovery.refresh(&state.pool, options).await {
        Ok(value) => ok(value),
        Err(error) => err(&format!("failed_to_refresh_discovery: {error}")),
    }
}

fn discovery_options(params: DiscoveryQ) -> DiscoveryListOptions {
    DiscoveryListOptions {
        limit: params.limit.unwrap_or(100).clamp(1, 500),
        offset: params.offset.unwrap_or(0).max(0),
        include_owned: params.include_owned.unwrap_or(false),
    }
}

pub async fn rediscovery(State(state): State<Arc<AppState>>) -> Json<Value> {
    respond_service!(
        state.recommendations.rediscovery(&state.pool),
        "failed_to_load_rediscovery"
    )
}
pub async fn current_rotation(State(state): State<Arc<AppState>>) -> Json<Value> {
    respond_service!(
        state.recommendations.current_rotation(&state.pool),
        "failed_to_load_current_rotation"
    )
}

pub async fn search(
    State(state): State<Arc<AppState>>,
    Query(params): Query<SearchQ>,
) -> Json<Value> {
    if params.q.trim().is_empty() {
        return err("missing_query");
    }
    respond_service!(
        state.analytics.search(&state.pool, &params.q),
        "failed_to_search"
    )
}

pub async fn cover(
    State(state): State<Arc<AppState>>,
    Path(cover_art_id): Path<String>,
) -> Response {
    if !is_valid_cover_art_id(&cover_art_id) {
        return (StatusCode::BAD_REQUEST, err("invalid_cover_art_id")).into_response();
    }

    let exists =
        sqlx::query_as::<_, (i64,)>("SELECT id FROM albums WHERE cover_art_id = ? LIMIT 1")
            .bind(&cover_art_id)
            .fetch_optional(&state.pool)
            .await;
    match exists {
        Ok(Some(_)) => {}
        Ok(None) => return (StatusCode::NOT_FOUND, err("cover_art_not_found")).into_response(),
        Err(_) => return err("failed_to_load_cover").into_response(),
    }

    match fetch_cover_art(&state.pool, &cover_art_id).await {
        Ok((content_type, bytes)) => {
            let mut headers = HeaderMap::new();
            headers.insert(
                CONTENT_TYPE,
                HeaderValue::from_str(&content_type)
                    .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream")),
            );
            headers.insert(
                CACHE_CONTROL,
                HeaderValue::from_static("public, max-age=86400, stale-while-revalidate=604800"),
            );
            if let Ok(etag) =
                HeaderValue::from_str(&format!("\"{}-{}\"", cover_art_id, bytes.len()))
            {
                headers.insert(ETAG, etag);
            }
            (StatusCode::OK, headers, bytes).into_response()
        }
        Err(error) if error.to_string().contains("not found") => {
            (StatusCode::NOT_FOUND, err("cover_art_not_found")).into_response()
        }
        Err(error) => (
            StatusCode::BAD_GATEWAY,
            err(&format!("failed_to_fetch_cover_art: {error}")),
        )
            .into_response(),
    }
}

fn is_valid_cover_art_id(cover_art_id: &str) -> bool {
    !cover_art_id.is_empty()
        && cover_art_id.len() <= 256
        && cover_art_id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
}

async fn fetch_cover_art(
    pool: &sqlx::SqlitePool,
    cover_art_id: &str,
) -> anyhow::Result<(String, Bytes)> {
    let cfg = SubsonicConfig::load(pool).await?;
    let url = format!("{}/rest/getCoverArt", cfg.base_url.trim_end_matches('/'));
    let mut query = crate::services::sync::subsonic_auth_query(&cfg);
    query.push(("id".to_string(), cover_art_id.to_string()));
    let response = reqwest::Client::new().get(url).query(&query).send().await?;
    let status = response.status();
    if status == reqwest::StatusCode::NOT_FOUND {
        anyhow::bail!("cover art not found");
    }
    if !status.is_success() {
        anyhow::bail!("Subsonic getCoverArt returned HTTP {status}");
    }
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(infer_image_content_type)
        .unwrap_or_else(|| "application/octet-stream".to_string());
    let bytes = response.bytes().await?;
    if bytes.is_empty() {
        anyhow::bail!("cover art not found");
    }
    Ok((content_type, bytes))
}

fn infer_image_content_type(content_type: &str) -> String {
    let content_type = content_type
        .split(';')
        .next()
        .unwrap_or(content_type)
        .trim();
    match content_type {
        "image/jpeg" | "image/png" | "image/webp" | "image/gif" => content_type.to_string(),
        _ => "application/octet-stream".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    async fn test_pool() -> sqlx::SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        pool
    }

    #[tokio::test]
    async fn cover_fetch_returns_image_bytes_with_image_content_type() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buffer = [0_u8; 2048];
            let _ = socket.read(&mut buffer).await.unwrap();
            let body = b"\x89PNG\r\n\x1a\n";
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: image/png\r\nContent-Length: {}\r\n\r\n",
                body.len()
            );
            socket.write_all(response.as_bytes()).await.unwrap();
            socket.write_all(body).await.unwrap();
        });

        let pool = test_pool().await;
        sqlx::query(
            "INSERT INTO settings(key,value) VALUES
             ('subsonic_base_url', ?),
             ('subsonic_username', 'user'),
             ('subsonic_password', 'pass')",
        )
        .bind(format!("http://{addr}"))
        .execute(&pool)
        .await
        .unwrap();

        let (content_type, bytes) = fetch_cover_art(&pool, "valid-cover").await.unwrap();
        server.await.unwrap();

        assert_eq!(content_type, "image/png");
        assert_eq!(&bytes[..], b"\x89PNG\r\n\x1a\n");
        assert_ne!(content_type, "application/json");
    }

    #[test]
    fn cover_content_type_falls_back_for_non_images() {
        assert_eq!(
            infer_image_content_type("application/json"),
            "application/octet-stream"
        );
        assert_eq!(
            infer_image_content_type("image/jpeg; charset=utf-8"),
            "image/jpeg"
        );
    }
}
