use std::{sync::Arc, time::Duration};

use axum::{
    body::{Body, Bytes},
    extract::{Extension, Path, Query, State},
    http::{
        header::{
            ACCEPT_RANGES, CACHE_CONTROL, CONTENT_DISPOSITION, CONTENT_LENGTH, CONTENT_RANGE,
            CONTENT_TYPE, ETAG, IF_MODIFIED_SINCE, IF_NONE_MATCH, IF_RANGE, LAST_MODIFIED, RANGE,
        },
        HeaderMap, HeaderName, HeaderValue, StatusCode,
    },
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tracing::{error, info, warn};

use crate::auth::{bearer_token, AuthUser};
use crate::services::discovery::{DiscoveryListOptions, DiscoveryRefreshOptions};
use crate::services::settings::SettingsService;
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

#[derive(Deserialize)]
pub struct StreamQ {
    pub network: Option<String>,
    pub lossless: Option<String>,
}

#[derive(Deserialize)]
pub struct LoginPayload {
    pub username: String,
    pub password: String,
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

pub async fn login(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<LoginPayload>,
) -> impl IntoResponse {
    match state.auth.login(&payload.username, &payload.password).await {
        Ok(Some(session)) => (StatusCode::OK, ok(json!(session))),
        Ok(None) => (StatusCode::UNAUTHORIZED, err("invalid_credentials")),
        Err(error) => {
            warn!(error = %error, "failed to create login session");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                err("failed_to_create_session"),
            )
        }
    }
}

pub async fn logout(State(state): State<Arc<AppState>>, headers: HeaderMap) -> impl IntoResponse {
    if let Some(token) = bearer_token(&headers) {
        if let Err(error) = state.auth.logout(token).await {
            warn!(error = %error, "failed to revoke session");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                err("failed_to_revoke_session"),
            );
        }
    }

    (StatusCode::OK, ok(json!({"status": "signed_out"})))
}

pub async fn me(Extension(user): Extension<AuthUser>) -> Json<Value> {
    ok(json!({"username": user.username}))
}

pub async fn active_sessions(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    match state.auth.active_sessions(bearer_token(&headers)).await {
        Ok(sessions) => (StatusCode::OK, ok(json!(sessions))),
        Err(error) => {
            warn!(error = %error, "failed to load active sessions");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                err("failed_to_load_sessions"),
            )
        }
    }
}

pub async fn stream_token(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<AuthUser>,
) -> Json<Value> {
    ok(json!(state.auth.issue_stream_token(&user.username).await))
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
        Err(error) => {
            warn!(error = %error, "failed to save settings");
            err("failed_to_save_settings")
        }
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
pub async fn playlists(State(state): State<Arc<AppState>>) -> Json<Value> {
    match fetch_subsonic_playlists(&state.pool).await {
        Ok(playlists) => ok(playlists),
        Err(error) => {
            warn!(error = %error, "failed to load playlists");
            err("failed_to_load_playlists")
        }
    }
}
pub async fn playlist_by_id(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<Value> {
    if !is_valid_subsonic_id(&id) {
        return err("invalid_playlist_id");
    }

    match fetch_subsonic_playlist(&state.pool, &id).await {
        Ok(playlist) => ok(playlist),
        Err(error) if error.to_string().contains("not found") => err("playlist_not_found"),
        Err(error) => {
            warn!(playlist_id = id, error = %error, "failed to load playlist");
            err("failed_to_load_playlist")
        }
    }
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

pub async fn stream_track(
    State(state): State<Arc<AppState>>,
    Path(track_id): Path<i64>,
    Query(params): Query<StreamQ>,
    headers: HeaderMap,
) -> Response {
    if track_id <= 0 {
        return (StatusCode::BAD_REQUEST, err("invalid_track_id")).into_response();
    }

    match fetch_track_stream(
        &state.pool,
        &state.settings,
        track_id,
        params.network.as_deref(),
        lossless_query_enabled(params.lossless.as_deref()),
        &headers,
    )
    .await
    {
        Ok(response) => response,
        Err(TrackStreamError::NotFound) => {
            (StatusCode::NOT_FOUND, err("track_not_found")).into_response()
        }
        Err(TrackStreamError::NotStreamable) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            err("track_not_streamable"),
        )
            .into_response(),
        Err(TrackStreamError::UpstreamNotFound) => {
            (StatusCode::NOT_FOUND, err("track_stream_not_found")).into_response()
        }
        Err(TrackStreamError::UpstreamRangeNotSatisfiable(response)) => response,
        Err(TrackStreamError::Upstream(error)) => {
            warn!(track_id, error = %error, "failed to stream track from Subsonic");
            (StatusCode::BAD_GATEWAY, err("failed_to_stream_track")).into_response()
        }
    }
}

pub async fn track_now_playing(
    State(state): State<Arc<AppState>>,
    Path(track_id): Path<i64>,
) -> impl IntoResponse {
    if track_id <= 0 {
        return (StatusCode::BAD_REQUEST, err("invalid_track_id")).into_response();
    }

    match register_track_now_playing(&state.pool, track_id).await {
        Ok(()) => ok(json!({"track_id": track_id, "status": "registered"})).into_response(),
        Err(TrackStreamError::NotFound) => {
            (StatusCode::NOT_FOUND, err("track_not_found")).into_response()
        }
        Err(TrackStreamError::NotStreamable) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            err("track_not_streamable"),
        )
            .into_response(),
        Err(TrackStreamError::UpstreamNotFound) => {
            (StatusCode::NOT_FOUND, err("track_not_found_upstream")).into_response()
        }
        Err(TrackStreamError::UpstreamRangeNotSatisfiable(response)) => response,
        Err(TrackStreamError::Upstream(error)) => {
            warn!(track_id, error = %error, "failed to register Subsonic now playing");
            (
                StatusCode::BAD_GATEWAY,
                err("failed_to_register_now_playing"),
            )
                .into_response()
        }
    }
}

pub async fn track_scrobble(
    State(state): State<Arc<AppState>>,
    Path(track_id): Path<i64>,
) -> impl IntoResponse {
    if track_id <= 0 {
        return (StatusCode::BAD_REQUEST, err("invalid_track_id")).into_response();
    }

    match register_track_scrobble(&state.pool, track_id).await {
        Ok(()) => ok(json!({"track_id": track_id, "status": "scrobbled"})).into_response(),
        Err(TrackStreamError::NotFound) => {
            (StatusCode::NOT_FOUND, err("track_not_found")).into_response()
        }
        Err(TrackStreamError::NotStreamable) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            err("track_not_streamable"),
        )
            .into_response(),
        Err(TrackStreamError::UpstreamNotFound) => {
            (StatusCode::NOT_FOUND, err("track_not_found_upstream")).into_response()
        }
        Err(TrackStreamError::UpstreamRangeNotSatisfiable(response)) => response,
        Err(TrackStreamError::Upstream(error)) => {
            warn!(track_id, error = %error, "failed to scrobble Subsonic track");
            (StatusCode::BAD_GATEWAY, err("failed_to_scrobble_track")).into_response()
        }
    }
}

pub async fn track_lyrics(
    State(state): State<Arc<AppState>>,
    Path(track_id): Path<i64>,
) -> impl IntoResponse {
    if track_id <= 0 {
        return (StatusCode::BAD_REQUEST, err("invalid_track_id")).into_response();
    }

    match fetch_track_lyrics(&state.pool, track_id).await {
        Ok(Some(lyrics)) => ok(json!(lyrics)).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, err("lyrics_not_found")).into_response(),
        Err(TrackLyricsError::NotFound) => {
            (StatusCode::NOT_FOUND, err("track_not_found")).into_response()
        }
        Err(TrackLyricsError::Upstream(error)) => {
            warn!(track_id, error = %error, "failed to fetch track lyrics");
            (StatusCode::BAD_GATEWAY, err("failed_to_fetch_lyrics")).into_response()
        }
    }
}

pub async fn cover(
    State(state): State<Arc<AppState>>,
    Path(cover_art_id): Path<String>,
) -> Response {
    if !is_valid_cover_art_id(&cover_art_id) {
        return (StatusCode::BAD_REQUEST, err("invalid_cover_art_id")).into_response();
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
        Err(error) => {
            warn!(cover_art_id, error = %error, "failed to fetch cover art");
            (StatusCode::BAD_GATEWAY, err("failed_to_fetch_cover_art")).into_response()
        }
    }
}

pub async fn artist_image(
    State(state): State<Arc<AppState>>,
    Path(artist_id): Path<i64>,
) -> Response {
    if artist_id <= 0 {
        return (StatusCode::BAD_REQUEST, err("invalid_artist_id")).into_response();
    }

    let image_url = sqlx::query_as::<_, (Option<String>,)>(
        "SELECT image_url FROM artists WHERE id = ? LIMIT 1",
    )
    .bind(artist_id)
    .fetch_optional(&state.pool)
    .await;

    let image_url = match image_url {
        Ok(Some((Some(image_url),))) if !image_url.trim().is_empty() => image_url,
        Ok(Some(_)) | Ok(None) => {
            return (StatusCode::NOT_FOUND, err("artist_image_not_found")).into_response();
        }
        Err(_) => return err("failed_to_load_artist_image").into_response(),
    };

    match fetch_artist_image(&state.pool, &image_url).await {
        Ok((content_type, bytes)) => image_response(
            &content_type,
            &format!("\"artist-{}-{}\"", artist_id, bytes.len()),
            bytes,
        ),
        Err(error) if error.to_string().contains("not found") => {
            (StatusCode::NOT_FOUND, err("artist_image_not_found")).into_response()
        }
        Err(error) => {
            warn!(artist_id, error = %error, "failed to fetch artist image");
            (StatusCode::BAD_GATEWAY, err("failed_to_fetch_artist_image")).into_response()
        }
    }
}

fn is_valid_cover_art_id(cover_art_id: &str) -> bool {
    !cover_art_id.is_empty()
        && cover_art_id.len() <= 256
        && cover_art_id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | ':' | '~'))
}

fn is_valid_subsonic_id(id: &str) -> bool {
    !id.trim().is_empty()
        && id.len() <= 256
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | ':' | '~'))
}

async fn fetch_subsonic_playlists(pool: &sqlx::SqlitePool) -> anyhow::Result<Value> {
    let response = subsonic_json(pool, "getPlaylists", &[]).await?;
    let playlists = value_list(response["subsonic-response"]["playlists"].get("playlist"))
        .into_iter()
        .map(|playlist| {
            json!({
                "id": playlist["id"].as_str().unwrap_or_default(),
                "name": playlist["name"].as_str().unwrap_or("Untitled Playlist"),
                "song_count": playlist["songCount"].as_i64().or_else(|| playlist["entryCount"].as_i64()).unwrap_or(0),
                "duration_seconds": playlist["duration"].as_i64().unwrap_or(0),
                "cover_art_id": playlist["coverArt"].as_str()
            })
        })
        .collect::<Vec<_>>();
    Ok(json!(playlists))
}

async fn fetch_subsonic_playlist(pool: &sqlx::SqlitePool, id: &str) -> anyhow::Result<Value> {
    let response = subsonic_json(pool, "getPlaylist", &[("id", id.to_string())]).await?;
    let playlist = &response["subsonic-response"]["playlist"];
    if playlist.is_null() {
        anyhow::bail!("playlist not found");
    }

    let mut tracks = Vec::new();
    for entry in value_list(playlist.get("entry")) {
        let subsonic_id = entry["id"].as_str().unwrap_or_default();
        let local = sqlx::query_as::<_, (i64, Option<i64>, Option<String>, Option<i64>)>(
            "SELECT id, album_id, genre, duration_seconds FROM tracks WHERE subsonic_id = ? LIMIT 1",
        )
        .bind(subsonic_id)
        .fetch_optional(pool)
        .await?;
        let Some((track_id, album_id, genre, duration_seconds)) = local else {
            continue;
        };
        tracks.push(json!([
            track_id,
            entry["title"].as_str().unwrap_or("Unknown Track"),
            entry["artist"].as_str(),
            album_id,
            entry["album"].as_str(),
            entry["coverArt"].as_str(),
            duration_seconds.or_else(|| entry["duration"].as_i64()),
            genre.or_else(|| entry["genre"].as_str().map(ToString::to_string))
        ]));
    }

    Ok(json!({
        "playlist": {
            "id": playlist["id"].as_str().unwrap_or(id),
            "name": playlist["name"].as_str().unwrap_or("Untitled Playlist"),
            "song_count": playlist["songCount"].as_i64().or_else(|| playlist["entryCount"].as_i64()).unwrap_or(tracks.len() as i64),
            "duration_seconds": playlist["duration"].as_i64().unwrap_or(0),
            "cover_art_id": playlist["coverArt"].as_str()
        },
        "tracks": tracks
    }))
}

async fn subsonic_json(
    pool: &sqlx::SqlitePool,
    endpoint: &str,
    params: &[(&str, String)],
) -> anyhow::Result<Value> {
    subsonic_json_with_client(&reqwest::Client::new(), pool, endpoint, params).await
}

async fn subsonic_json_with_client(
    client: &reqwest::Client,
    pool: &sqlx::SqlitePool,
    endpoint: &str,
    params: &[(&str, String)],
) -> anyhow::Result<Value> {
    let cfg = SubsonicConfig::load(pool).await?;
    let url = format!("{}/rest/{endpoint}", cfg.base_url.trim_end_matches('/'));
    let mut query = crate::services::sync::subsonic_auth_query(&cfg);
    query.extend(
        params
            .iter()
            .map(|(key, value)| ((*key).to_string(), value.clone())),
    );

    let response = client.get(url).query(&query).send().await?;
    let status = response.status();
    let body = response.text().await?;
    if !status.is_success() {
        anyhow::bail!("Subsonic {endpoint} returned HTTP {status}");
    }
    let value = serde_json::from_str::<Value>(&body)?;
    if value["subsonic-response"]["status"].as_str() == Some("failed") {
        let message = value["subsonic-response"]["error"]["message"]
            .as_str()
            .unwrap_or("Subsonic returned an error");
        anyhow::bail!("Subsonic {endpoint} failed: {message}");
    }
    Ok(value)
}

fn value_list(value: Option<&Value>) -> Vec<Value> {
    match value {
        Some(Value::Array(values)) => values.clone(),
        Some(value) if !value.is_null() => vec![value.clone()],
        _ => Vec::new(),
    }
}

enum TrackStreamError {
    NotFound,
    NotStreamable,
    UpstreamNotFound,
    UpstreamRangeNotSatisfiable(Response),
    Upstream(anyhow::Error),
}

impl std::fmt::Debug for TrackStreamError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound => formatter.write_str("NotFound"),
            Self::NotStreamable => formatter.write_str("NotStreamable"),
            Self::UpstreamNotFound => formatter.write_str("UpstreamNotFound"),
            Self::UpstreamRangeNotSatisfiable(_) => {
                formatter.write_str("UpstreamRangeNotSatisfiable")
            }
            Self::Upstream(error) => formatter.debug_tuple("Upstream").field(error).finish(),
        }
    }
}

impl From<anyhow::Error> for TrackStreamError {
    fn from(error: anyhow::Error) -> Self {
        Self::Upstream(error)
    }
}

struct TrackStreamMetadata {
    subsonic_id: String,
    title: String,
    content_type: Option<String>,
    suffix: Option<String>,
}

async fn fetch_track_stream(
    pool: &sqlx::SqlitePool,
    settings: &SettingsService,
    track_id: i64,
    network_type: Option<&str>,
    lossless: bool,
    request_headers: &HeaderMap,
) -> Result<Response, TrackStreamError> {
    let track = sqlx::query_as::<_, (Option<String>, String, Option<String>, Option<String>)>(
        "SELECT subsonic_id, title, content_type, suffix FROM tracks WHERE id = ? LIMIT 1",
    )
    .bind(track_id)
    .fetch_optional(pool)
    .await
    .map_err(|error| TrackStreamError::Upstream(error.into()))?;

    let Some((subsonic_id, title, content_type, suffix)) = track else {
        return Err(TrackStreamError::NotFound);
    };
    let Some(subsonic_id) = subsonic_id.and_then(|id| non_empty_owned(&id)) else {
        return Err(TrackStreamError::NotStreamable);
    };

    let metadata = TrackStreamMetadata {
        subsonic_id,
        title,
        content_type,
        suffix,
    };
    fetch_subsonic_track_stream(
        pool,
        settings,
        &metadata,
        network_type,
        lossless,
        request_headers,
    )
    .await
}

async fn register_track_now_playing(
    pool: &sqlx::SqlitePool,
    track_id: i64,
) -> Result<(), TrackStreamError> {
    register_subsonic_scrobble(pool, track_id, false).await
}

async fn register_track_scrobble(
    pool: &sqlx::SqlitePool,
    track_id: i64,
) -> Result<(), TrackStreamError> {
    register_subsonic_scrobble(pool, track_id, true).await
}

async fn register_subsonic_scrobble(
    pool: &sqlx::SqlitePool,
    track_id: i64,
    submission: bool,
) -> Result<(), TrackStreamError> {
    let subsonic_id = track_subsonic_id(pool, track_id).await?;
    let time = chrono::Utc::now().timestamp_millis().to_string();
    let submission = if submission { "true" } else { "false" }.to_string();
    subsonic_json(
        pool,
        "scrobble",
        &[
            ("id", subsonic_id),
            ("time", time),
            ("submission", submission),
        ],
    )
    .await
    .map(|_| ())
    .map_err(|error| TrackStreamError::Upstream(error.into()))
}

async fn track_subsonic_id(
    pool: &sqlx::SqlitePool,
    track_id: i64,
) -> Result<String, TrackStreamError> {
    let track = sqlx::query_as::<_, (Option<String>,)>(
        "SELECT subsonic_id FROM tracks WHERE id = ? LIMIT 1",
    )
    .bind(track_id)
    .fetch_optional(pool)
    .await
    .map_err(|error| TrackStreamError::Upstream(error.into()))?;

    let Some((subsonic_id,)) = track else {
        return Err(TrackStreamError::NotFound);
    };
    subsonic_id
        .and_then(|id| non_empty_owned(&id))
        .ok_or(TrackStreamError::NotStreamable)
}

#[derive(Debug)]
enum TrackLyricsError {
    NotFound,
    Upstream(anyhow::Error),
}

impl From<anyhow::Error> for TrackLyricsError {
    fn from(error: anyhow::Error) -> Self {
        Self::Upstream(error)
    }
}

#[derive(Debug)]
struct TrackLyricsMetadata {
    track_id: i64,
    subsonic_id: Option<String>,
    title: String,
    artist: Option<String>,
    album: Option<String>,
    duration_seconds: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
struct LyricsLine {
    start_ms: Option<i64>,
    text: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
struct TrackLyrics {
    track_id: i64,
    source: String,
    synced: bool,
    instrumental: bool,
    title: String,
    artist: Option<String>,
    lines: Vec<LyricsLine>,
    text: Option<String>,
}

async fn fetch_track_lyrics(
    pool: &sqlx::SqlitePool,
    track_id: i64,
) -> Result<Option<TrackLyrics>, TrackLyricsError> {
    let metadata = track_lyrics_metadata(pool, track_id).await?;
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .user_agent(format!(
            "music-dashboard-backend/{}",
            env!("CARGO_PKG_VERSION")
        ))
        .build()
        .map_err(|error| TrackLyricsError::Upstream(error.into()))?;

    if let Some(lyrics) = fetch_subsonic_track_lyrics(&client, pool, &metadata).await {
        return Ok(Some(lyrics));
    }

    match fetch_lrclib_lyrics(&client, &metadata).await {
        Ok(lyrics) => Ok(lyrics),
        Err(error) => {
            warn!(track_id = metadata.track_id, error = %error, "LRCLIB lyrics lookup failed");
            Ok(None)
        }
    }
}

async fn track_lyrics_metadata(
    pool: &sqlx::SqlitePool,
    track_id: i64,
) -> Result<TrackLyricsMetadata, TrackLyricsError> {
    let track = sqlx::query_as::<
        _,
        (
            Option<String>,
            String,
            Option<String>,
            Option<String>,
            Option<i64>,
        ),
    >(
        "SELECT t.subsonic_id,
                t.title,
                COALESCE(NULLIF(t.raw_artist, ''), ar.name),
                al.title,
                t.duration_seconds
         FROM tracks t
         LEFT JOIN artists ar ON ar.id = t.artist_id
         LEFT JOIN albums al ON al.id = t.album_id
         WHERE t.id = ?
         LIMIT 1",
    )
    .bind(track_id)
    .fetch_optional(pool)
    .await
    .map_err(|error| TrackLyricsError::Upstream(error.into()))?;

    let Some((subsonic_id, title, artist, album, duration_seconds)) = track else {
        return Err(TrackLyricsError::NotFound);
    };

    Ok(TrackLyricsMetadata {
        track_id,
        subsonic_id: subsonic_id.and_then(|id| non_empty_owned(&id)),
        title,
        artist: artist.and_then(|artist| non_empty_owned(&artist)),
        album: album.and_then(|album| non_empty_owned(&album)),
        duration_seconds,
    })
}

async fn fetch_subsonic_track_lyrics(
    client: &reqwest::Client,
    pool: &sqlx::SqlitePool,
    metadata: &TrackLyricsMetadata,
) -> Option<TrackLyrics> {
    if let Some(subsonic_id) = metadata.subsonic_id.as_deref() {
        match subsonic_json_with_client(
            client,
            pool,
            "getLyricsBySongId",
            &[("id", subsonic_id.to_string())],
        )
        .await
        {
            Ok(response) => {
                if let Some(lyrics) = open_subsonic_lyrics_from_value(metadata, &response) {
                    return Some(lyrics);
                }
            }
            Err(error) => {
                info!(track_id = metadata.track_id, error = %error, "OpenSubsonic lyrics lookup did not return lyrics");
            }
        }
    }

    let Some(artist) = metadata.artist.as_deref() else {
        return None;
    };

    match subsonic_json_with_client(
        client,
        pool,
        "getLyrics",
        &[
            ("artist", artist.to_string()),
            ("title", metadata.title.clone()),
        ],
    )
    .await
    {
        Ok(response) => subsonic_lyrics_from_value(metadata, &response),
        Err(error) => {
            info!(track_id = metadata.track_id, error = %error, "Subsonic lyrics lookup did not return lyrics");
            None
        }
    }
}

fn open_subsonic_lyrics_from_value(
    metadata: &TrackLyricsMetadata,
    response: &Value,
) -> Option<TrackLyrics> {
    let lyrics_list = &response["subsonic-response"]["lyricsList"];
    let structured = value_list(lyrics_list.get("structuredLyrics"));
    let selected = structured
        .iter()
        .find(|entry| {
            entry["synced"].as_bool().unwrap_or(false)
                && matches!(entry["kind"].as_str(), Some("main") | None)
        })
        .or_else(|| {
            structured
                .iter()
                .find(|entry| matches!(entry["kind"].as_str(), Some("main") | None))
        })?;
    let offset = selected["offset"].as_i64().unwrap_or(0);
    let lines = value_list(selected.get("line"))
        .into_iter()
        .filter_map(|line| {
            let text = line["value"].as_str()?.trim().to_string();
            let start_ms = line["start"].as_i64().map(|start| (start + offset).max(0));
            Some(LyricsLine { start_ms, text })
        })
        .collect::<Vec<_>>();
    lyrics_from_lines(metadata, "subsonic", lines, false)
}

fn subsonic_lyrics_from_value(
    metadata: &TrackLyricsMetadata,
    response: &Value,
) -> Option<TrackLyrics> {
    let lyrics = &response["subsonic-response"]["lyrics"];
    let text = lyrics
        .as_str()
        .or_else(|| lyrics["value"].as_str())
        .or_else(|| lyrics["text"].as_str())
        .or_else(|| lyrics["_value"].as_str())
        .or_else(|| lyrics["$text"].as_str())?;
    lyrics_from_text(metadata, "subsonic", text, false)
}

async fn fetch_lrclib_lyrics(
    client: &reqwest::Client,
    metadata: &TrackLyricsMetadata,
) -> anyhow::Result<Option<TrackLyrics>> {
    let Some(artist) = metadata.artist.as_deref() else {
        return Ok(None);
    };
    let mut query = vec![
        ("track_name", metadata.title.clone()),
        ("artist_name", artist.to_string()),
    ];
    if let Some(album) = metadata.album.as_deref() {
        query.push(("album_name", album.to_string()));
    }
    if let Some(duration) = metadata.duration_seconds.filter(|duration| *duration > 0) {
        query.push(("duration", duration.to_string()));
    }

    let response = client
        .get("https://lrclib.net/api/get")
        .query(&query)
        .send()
        .await?;
    let status = response.status();
    if status == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    if !status.is_success() {
        anyhow::bail!("LRCLIB returned HTTP {status}");
    }

    let value = response.json::<Value>().await?;
    let instrumental = value["instrumental"].as_bool().unwrap_or(false);
    if instrumental {
        return Ok(Some(TrackLyrics {
            track_id: metadata.track_id,
            source: "lrclib".to_string(),
            synced: false,
            instrumental: true,
            title: metadata.title.clone(),
            artist: metadata.artist.clone(),
            lines: Vec::new(),
            text: None,
        }));
    }

    let text = value["syncedLyrics"]
        .as_str()
        .and_then(non_empty_owned)
        .or_else(|| value["plainLyrics"].as_str().and_then(non_empty_owned));
    Ok(text.and_then(|text| lyrics_from_text(metadata, "lrclib", &text, false)))
}

fn lyrics_from_text(
    metadata: &TrackLyricsMetadata,
    source: &str,
    text: &str,
    instrumental: bool,
) -> Option<TrackLyrics> {
    let instrumental = instrumental || text.lines().any(is_instrumental_lrc_tag);
    if instrumental {
        return Some(TrackLyrics {
            track_id: metadata.track_id,
            source: source.to_string(),
            synced: false,
            instrumental: true,
            title: metadata.title.clone(),
            artist: metadata.artist.clone(),
            lines: Vec::new(),
            text: None,
        });
    }

    let mut lines = parse_lrc_lines(text);
    let synced = lines.iter().any(|line| line.start_ms.is_some());
    if synced {
        lines.sort_by_key(|line| line.start_ms.unwrap_or(i64::MAX));
    }
    lyrics_from_lines(metadata, source, lines, false)
}

fn lyrics_from_lines(
    metadata: &TrackLyricsMetadata,
    source: &str,
    lines: Vec<LyricsLine>,
    instrumental: bool,
) -> Option<TrackLyrics> {
    let lines = lines
        .into_iter()
        .filter(|line| !line.text.trim().is_empty())
        .collect::<Vec<_>>();
    if lines.is_empty() && !instrumental {
        return None;
    }

    let synced = lines.iter().any(|line| line.start_ms.is_some());
    let text = if lines.is_empty() {
        None
    } else {
        Some(
            lines
                .iter()
                .map(|line| line.text.as_str())
                .collect::<Vec<_>>()
                .join("\n"),
        )
    };

    Some(TrackLyrics {
        track_id: metadata.track_id,
        source: source.to_string(),
        synced,
        instrumental,
        title: metadata.title.clone(),
        artist: metadata.artist.clone(),
        lines,
        text,
    })
}

fn parse_lrc_lines(text: &str) -> Vec<LyricsLine> {
    let mut lines = Vec::new();
    for raw_line in text.lines() {
        let trimmed = raw_line.trim();
        if trimmed.is_empty() || is_lrc_metadata_line(trimmed) {
            continue;
        }

        let (timestamps, lyric_text) = split_lrc_timestamps(trimmed);
        if timestamps.is_empty() {
            lines.push(LyricsLine {
                start_ms: None,
                text: trimmed.to_string(),
            });
            continue;
        }

        let lyric_text = lyric_text.trim();
        if lyric_text.is_empty() {
            continue;
        }
        for start_ms in timestamps {
            lines.push(LyricsLine {
                start_ms: Some(start_ms),
                text: lyric_text.to_string(),
            });
        }
    }
    lines
}

fn split_lrc_timestamps(line: &str) -> (Vec<i64>, &str) {
    let mut rest = line;
    let mut timestamps = Vec::new();
    while let Some(stripped) = rest.strip_prefix('[') {
        let Some(end) = stripped.find(']') else {
            break;
        };
        let tag = &stripped[..end];
        let Some(start_ms) = parse_lrc_timestamp(tag) else {
            break;
        };
        timestamps.push(start_ms);
        rest = &stripped[end + 1..];
    }
    (timestamps, rest)
}

fn parse_lrc_timestamp(tag: &str) -> Option<i64> {
    let parts = tag.split(':').collect::<Vec<_>>();
    if !(2..=3).contains(&parts.len()) {
        return None;
    }

    let seconds_part = parts.last()?.trim();
    let (seconds, millis) = match seconds_part.split_once('.') {
        Some((seconds, fraction)) => (seconds, fraction_to_millis(fraction)?),
        None => (seconds_part, 0),
    };
    let seconds = seconds.parse::<i64>().ok()?;
    if !(0..60).contains(&seconds) {
        return None;
    }

    let minutes = parts[parts.len() - 2].trim().parse::<i64>().ok()?;
    let hours = if parts.len() == 3 {
        parts[0].trim().parse::<i64>().ok()?
    } else {
        0
    };
    if minutes < 0 || hours < 0 {
        return None;
    }

    Some(((hours * 60 + minutes) * 60 + seconds) * 1000 + millis)
}

fn fraction_to_millis(fraction: &str) -> Option<i64> {
    if fraction.is_empty() || !fraction.chars().all(|character| character.is_ascii_digit()) {
        return None;
    }
    let mut padded = fraction.chars().take(3).collect::<String>();
    while padded.len() < 3 {
        padded.push('0');
    }
    padded.parse::<i64>().ok()
}

fn is_lrc_metadata_line(line: &str) -> bool {
    let Some(inner) = line
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
    else {
        return false;
    };
    if parse_lrc_timestamp(inner).is_some() {
        return false;
    }
    let Some((key, _)) = inner.split_once(':') else {
        return false;
    };
    matches!(
        key.trim().to_ascii_lowercase().as_str(),
        "al" | "ar" | "au" | "by" | "length" | "offset" | "re" | "ti" | "ve"
    )
}

fn is_instrumental_lrc_tag(line: &str) -> bool {
    line.trim().eq_ignore_ascii_case("[au: instrumental]")
}

async fn fetch_subsonic_track_stream(
    pool: &sqlx::SqlitePool,
    settings: &SettingsService,
    metadata: &TrackStreamMetadata,
    network_type: Option<&str>,
    lossless: bool,
    request_headers: &HeaderMap,
) -> Result<Response, TrackStreamError> {
    let cfg = SubsonicConfig::load(pool).await?;
    let url = format!("{}/rest/stream", cfg.base_url.trim_end_matches('/'));
    let mut query = crate::services::sync::subsonic_auth_query(&cfg);
    query.push(("id".to_string(), metadata.subsonic_id.clone()));
    if let Some(quality) = stream_transcode_quality(pool, settings, network_type, lossless).await? {
        query.push(("maxBitRate".to_string(), quality.to_string()));
        query.push(("format".to_string(), "mp3".to_string()));
    }

    let mut request = reqwest::Client::new().get(url).query(&query);
    for header in [RANGE, IF_RANGE, IF_NONE_MATCH, IF_MODIFIED_SINCE] {
        if let Some(value) = request_headers.get(&header).and_then(header_value_to_str) {
            request = request.header(header.as_str(), value);
        }
    }

    let response = request
        .send()
        .await
        .map_err(|error| TrackStreamError::Upstream(error.into()))?;
    let status = response.status();

    if status == reqwest::StatusCode::NOT_FOUND {
        return Err(TrackStreamError::UpstreamNotFound);
    }

    let downstream_status =
        StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
    if status == reqwest::StatusCode::RANGE_NOT_SATISFIABLE {
        return Err(TrackStreamError::UpstreamRangeNotSatisfiable(
            streaming_response(downstream_status, response, metadata),
        ));
    }
    if !status.is_success() && status != reqwest::StatusCode::NOT_MODIFIED {
        return Err(TrackStreamError::Upstream(anyhow::anyhow!(
            "Subsonic stream returned HTTP {status}"
        )));
    }

    Ok(streaming_response(downstream_status, response, metadata))
}

async fn stream_transcode_quality(
    pool: &sqlx::SqlitePool,
    settings: &SettingsService,
    network_type: Option<&str>,
    lossless: bool,
) -> Result<Option<u16>, TrackStreamError> {
    if lossless {
        return Ok(None);
    }

    let mode = settings
        .get_value(pool, "stream_transcode_mode")
        .await
        .map_err(|error| TrackStreamError::Upstream(error.into()))?
        .unwrap_or_else(|| "never".to_string());
    let should_transcode = match mode.as_str() {
        "always" => true,
        "cellular" => network_type
            .map(|value| value.eq_ignore_ascii_case("cellular"))
            .unwrap_or(false),
        _ => false,
    };

    if !should_transcode {
        return Ok(None);
    }

    let quality = settings
        .get_value(pool, "stream_transcode_quality")
        .await
        .map_err(|error| TrackStreamError::Upstream(error.into()))?
        .and_then(|value| value.parse::<u16>().ok())
        .filter(|value| matches!(value, 96 | 128 | 192 | 256 | 320))
        .unwrap_or(192);

    Ok(Some(quality))
}

fn streaming_response(
    status: StatusCode,
    upstream: reqwest::Response,
    metadata: &TrackStreamMetadata,
) -> Response {
    let mut headers = HeaderMap::new();
    copy_upstream_header(&mut headers, upstream.headers(), CONTENT_TYPE);
    copy_upstream_header(&mut headers, upstream.headers(), CONTENT_LENGTH);
    copy_upstream_header(&mut headers, upstream.headers(), CONTENT_RANGE);
    copy_upstream_header(&mut headers, upstream.headers(), ACCEPT_RANGES);
    copy_upstream_header(&mut headers, upstream.headers(), ETAG);
    copy_upstream_header(&mut headers, upstream.headers(), LAST_MODIFIED);

    if !headers.contains_key(CONTENT_TYPE) {
        headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_str(&track_content_type(metadata))
                .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream")),
        );
    }
    if !headers.contains_key(ACCEPT_RANGES) {
        headers.insert(ACCEPT_RANGES, HeaderValue::from_static("bytes"));
    }
    headers.insert(
        CACHE_CONTROL,
        HeaderValue::from_static("private, max-age=86400"),
    );
    if let Ok(disposition) = HeaderValue::from_str(&format!(
        "inline; filename=\"{}\"",
        quoted_filename(&metadata.title, metadata.suffix.as_deref())
    )) {
        headers.insert(CONTENT_DISPOSITION, disposition);
    }

    (status, headers, Body::from_stream(upstream.bytes_stream())).into_response()
}

fn copy_upstream_header(
    headers: &mut HeaderMap,
    upstream: &reqwest::header::HeaderMap,
    name: HeaderName,
) {
    if let Some(value) = upstream
        .get(name.as_str())
        .and_then(|value| value.to_str().ok())
        .and_then(|value| HeaderValue::from_str(value).ok())
    {
        headers.insert(name, value);
    }
}

fn header_value_to_str(value: &HeaderValue) -> Option<&str> {
    value.to_str().ok().filter(|value| !value.trim().is_empty())
}

fn lossless_query_enabled(value: Option<&str>) -> bool {
    matches!(
        value.map(|value| value.trim().to_ascii_lowercase()),
        Some(value) if matches!(value.as_str(), "1" | "true" | "yes" | "on")
    )
}

fn non_empty_owned(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn track_content_type(metadata: &TrackStreamMetadata) -> String {
    metadata
        .content_type
        .as_deref()
        .and_then(non_empty_owned)
        .unwrap_or_else(|| content_type_for_suffix(metadata.suffix.as_deref()))
}

fn content_type_for_suffix(suffix: Option<&str>) -> String {
    match suffix
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "flac" => "audio/flac",
        "m4a" | "mp4" | "aac" => "audio/mp4",
        "mp3" => "audio/mpeg",
        "ogg" | "oga" => "audio/ogg",
        "opus" => "audio/opus",
        "wav" => "audio/wav",
        "webm" => "audio/webm",
        _ => "application/octet-stream",
    }
    .to_string()
}

fn quoted_filename(title: &str, suffix: Option<&str>) -> String {
    let mut filename = title
        .chars()
        .map(|character| match character {
            '"' | '\\' | '/' | '\0'..='\u{1f}' => '_',
            character => character,
        })
        .collect::<String>();
    if filename.trim().is_empty() {
        filename = "track".to_string();
    }
    if let Some(suffix) = suffix.and_then(non_empty_owned) {
        let suffix = suffix.trim_start_matches('.');
        if !suffix.is_empty() {
            filename.push('.');
            filename.push_str(suffix);
        }
    }
    filename
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

async fn fetch_artist_image(
    pool: &sqlx::SqlitePool,
    image_url: &str,
) -> anyhow::Result<(String, Bytes)> {
    let cfg = SubsonicConfig::load(pool).await?;
    let url = resolve_artist_image_url(&cfg.base_url, image_url)?;
    let response = reqwest::Client::new().get(url).send().await?;
    let status = response.status();
    if status == reqwest::StatusCode::NOT_FOUND {
        anyhow::bail!("artist image not found");
    }
    if !status.is_success() {
        anyhow::bail!("Navidrome artist image returned HTTP {status}");
    }
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(infer_image_content_type)
        .unwrap_or_else(|| "application/octet-stream".to_string());
    let bytes = response.bytes().await?;
    if bytes.is_empty() {
        anyhow::bail!("artist image not found");
    }
    Ok((content_type, bytes))
}

fn resolve_artist_image_url(base_url: &str, image_url: &str) -> anyhow::Result<reqwest::Url> {
    let base = reqwest::Url::parse(&format!("{}/", base_url.trim_end_matches('/')))?;
    let resolved = base.join(image_url.trim())?;
    if resolved.scheme() != base.scheme()
        || resolved.host_str() != base.host_str()
        || resolved.port_or_known_default() != base.port_or_known_default()
    {
        anyhow::bail!("artist image URL is outside the configured Subsonic origin");
    }
    if resolved.path() != "/share/img" && !resolved.path().starts_with("/share/img/") {
        anyhow::bail!("artist image URL is not a Navidrome /share/img URL");
    }
    Ok(resolved)
}

fn image_response(content_type: &str, etag: &str, bytes: Bytes) -> Response {
    let mut headers = HeaderMap::new();
    headers.insert(
        CONTENT_TYPE,
        HeaderValue::from_str(content_type)
            .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream")),
    );
    headers.insert(
        CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=86400, stale-while-revalidate=604800"),
    );
    if let Ok(etag) = HeaderValue::from_str(etag) {
        headers.insert(ETAG, etag);
    }
    (StatusCode::OK, headers, bytes).into_response()
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

    fn lyrics_metadata() -> TrackLyricsMetadata {
        TrackLyricsMetadata {
            track_id: 7,
            subsonic_id: Some("subsonic-track".to_string()),
            title: "Song".to_string(),
            artist: Some("Artist".to_string()),
            album: Some("Album".to_string()),
            duration_seconds: Some(123),
        }
    }

    #[test]
    fn lrc_parser_extracts_synced_lines_and_skips_metadata() {
        let lyrics = lyrics_from_text(
            &lyrics_metadata(),
            "lrclib",
            "[ar:Artist]\n[00:01.50]First line\n[00:03.000][00:04.25]Repeated\nPlain line",
            false,
        )
        .unwrap();

        assert!(lyrics.synced);
        assert_eq!(
            lyrics.lines,
            vec![
                LyricsLine {
                    start_ms: Some(1500),
                    text: "First line".to_string()
                },
                LyricsLine {
                    start_ms: Some(3000),
                    text: "Repeated".to_string()
                },
                LyricsLine {
                    start_ms: Some(4250),
                    text: "Repeated".to_string()
                },
                LyricsLine {
                    start_ms: None,
                    text: "Plain line".to_string()
                }
            ]
        );
    }

    #[test]
    fn open_subsonic_lyrics_prefers_synced_main_entry() {
        let response = json!({
            "subsonic-response": {
                "lyricsList": {
                    "structuredLyrics": [
                        {
                            "kind": "main",
                            "synced": false,
                            "line": [{"value": "Plain"}]
                        },
                        {
                            "kind": "main",
                            "offset": -250,
                            "synced": true,
                            "line": [
                                {"start": 1000, "value": "Timed"},
                                {"start": 2000, "value": "Again"}
                            ]
                        }
                    ]
                }
            }
        });

        let lyrics = open_subsonic_lyrics_from_value(&lyrics_metadata(), &response).unwrap();

        assert_eq!(lyrics.source, "subsonic");
        assert!(lyrics.synced);
        assert_eq!(lyrics.lines[0].start_ms, Some(750));
        assert_eq!(lyrics.lines[0].text, "Timed");
    }

    #[test]
    fn classic_subsonic_lyrics_extracts_plain_value() {
        let response = json!({
            "subsonic-response": {
                "lyrics": {
                    "artist": "Artist",
                    "title": "Song",
                    "value": "Line one\nLine two"
                }
            }
        });

        let lyrics = subsonic_lyrics_from_value(&lyrics_metadata(), &response).unwrap();

        assert_eq!(lyrics.source, "subsonic");
        assert!(!lyrics.synced);
        assert_eq!(
            lyrics.lines,
            vec![
                LyricsLine {
                    start_ms: None,
                    text: "Line one".to_string()
                },
                LyricsLine {
                    start_ms: None,
                    text: "Line two".to_string()
                }
            ]
        );
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

    #[test]
    fn artist_image_url_resolves_only_configured_share_img_urls() {
        let url =
            resolve_artist_image_url("http://music.example", "/share/img/token?size=600").unwrap();
        assert_eq!(
            url.as_str(),
            "http://music.example/share/img/token?size=600"
        );

        let absolute = resolve_artist_image_url(
            "http://music.example",
            "http://music.example/share/img/token",
        )
        .unwrap();
        assert_eq!(absolute.as_str(), "http://music.example/share/img/token");

        assert!(resolve_artist_image_url(
            "http://music.example",
            "http://other.example/share/img/token"
        )
        .is_err());
        assert!(
            resolve_artist_image_url("http://music.example", "/rest/getCoverArt?id=1").is_err()
        );
    }

    #[test]
    fn lossless_query_accepts_browser_and_flag_values() {
        assert!(lossless_query_enabled(Some("1")));
        assert!(lossless_query_enabled(Some("true")));
        assert!(lossless_query_enabled(Some("yes")));
        assert!(lossless_query_enabled(Some("on")));
        assert!(!lossless_query_enabled(Some("0")));
        assert!(!lossless_query_enabled(Some("false")));
        assert!(!lossless_query_enabled(None));
    }

    #[tokio::test]
    async fn artist_image_fetch_returns_navidrome_share_image_bytes() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buffer = [0_u8; 2048];
            let read = socket.read(&mut buffer).await.unwrap();
            let request = String::from_utf8_lossy(&buffer[..read]);
            assert!(request.starts_with("GET /share/img/token?size=600 HTTP/1.1"));

            let body = b"\xff\xd8\xff";
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: image/jpeg; charset=utf-8\r\nContent-Length: {}\r\n\r\n",
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

        let (content_type, bytes) = fetch_artist_image(&pool, "/share/img/token?size=600")
            .await
            .unwrap();
        server.await.unwrap();

        assert_eq!(content_type, "image/jpeg");
        assert_eq!(&bytes[..], b"\xff\xd8\xff");
    }

    #[tokio::test]
    async fn track_stream_proxies_range_request_without_buffering_to_disk() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buffer = [0_u8; 4096];
            let read = socket.read(&mut buffer).await.unwrap();
            let request = String::from_utf8_lossy(&buffer[..read]);
            assert!(request.starts_with("GET /rest/stream?"));
            assert!(request.contains("id=subsonic-track"));
            assert!(
                request.contains("\r\nrange: bytes=2-5\r\n")
                    || request.contains("\r\nRange: bytes=2-5\r\n")
            );

            let body = b"cdef";
            let response = format!(
                "HTTP/1.1 206 Partial Content\r\nContent-Type: audio/flac\r\nContent-Length: {}\r\nContent-Range: bytes 2-5/10\r\nAccept-Ranges: bytes\r\nETag: \"song\"\r\n\r\n",
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
        sqlx::query(
            "INSERT INTO tracks(id,subsonic_id,title,content_type,suffix)
             VALUES(1,'subsonic-track','Song','audio/flac','flac')",
        )
        .execute(&pool)
        .await
        .unwrap();
        let mut request_headers = HeaderMap::new();
        request_headers.insert(RANGE, HeaderValue::from_static("bytes=2-5"));

        let settings = SettingsService;
        let response = fetch_track_stream(&pool, &settings, 1, None, false, &request_headers)
            .await
            .unwrap();
        server.await.unwrap();
        let status = response.status();
        let headers = response.headers().clone();
        let body = axum::body::to_bytes(response.into_body(), 1024)
            .await
            .unwrap();

        assert_eq!(status, StatusCode::PARTIAL_CONTENT);
        assert_eq!(
            headers
                .get(CONTENT_TYPE)
                .and_then(|value| value.to_str().ok()),
            Some("audio/flac")
        );
        assert_eq!(
            headers
                .get(CONTENT_RANGE)
                .and_then(|value| value.to_str().ok()),
            Some("bytes 2-5/10")
        );
        assert_eq!(
            headers
                .get(CACHE_CONTROL)
                .and_then(|value| value.to_str().ok()),
            Some("private, max-age=86400")
        );
        assert_eq!(&body[..], b"cdef");
    }

    #[tokio::test]
    async fn track_stream_applies_cellular_transcoding_settings() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buffer = [0_u8; 4096];
            let read = socket.read(&mut buffer).await.unwrap();
            let request = String::from_utf8_lossy(&buffer[..read]);
            assert!(request.starts_with("GET /rest/stream?"));
            assert!(request.contains("id=subsonic-track"));
            assert!(request.contains("maxBitRate=128"));
            assert!(request.contains("format=mp3"));

            let body = b"mp3";
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: audio/mpeg\r\nContent-Length: {}\r\n\r\n",
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
             ('subsonic_password', 'pass'),
             ('stream_transcode_mode', 'cellular'),
             ('stream_transcode_quality', '128')",
        )
        .bind(format!("http://{addr}"))
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO tracks(id,subsonic_id,title,content_type,suffix)
             VALUES(1,'subsonic-track','Song','audio/flac','flac')",
        )
        .execute(&pool)
        .await
        .unwrap();

        let settings = SettingsService;
        let response = fetch_track_stream(
            &pool,
            &settings,
            1,
            Some("cellular"),
            false,
            &HeaderMap::new(),
        )
        .await
        .unwrap();
        server.await.unwrap();
        let body = axum::body::to_bytes(response.into_body(), 1024)
            .await
            .unwrap();

        assert_eq!(&body[..], b"mp3");
    }

    #[tokio::test]
    async fn track_stream_lossless_query_bypasses_transcoding_settings() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buffer = [0_u8; 4096];
            let read = socket.read(&mut buffer).await.unwrap();
            let request = String::from_utf8_lossy(&buffer[..read]);
            assert!(request.starts_with("GET /rest/stream?"));
            assert!(request.contains("id=subsonic-track"));
            assert!(!request.contains("maxBitRate="));
            assert!(!request.contains("format=mp3"));

            let body = b"flac";
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: audio/flac\r\nContent-Length: {}\r\n\r\n",
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
             ('subsonic_password', 'pass'),
             ('stream_transcode_mode', 'always'),
             ('stream_transcode_quality', '128')",
        )
        .bind(format!("http://{addr}"))
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO tracks(id,subsonic_id,title,content_type,suffix)
             VALUES(1,'subsonic-track','Song','audio/flac','flac')",
        )
        .execute(&pool)
        .await
        .unwrap();

        let settings = SettingsService;
        let response = fetch_track_stream(
            &pool,
            &settings,
            1,
            Some("cellular"),
            true,
            &HeaderMap::new(),
        )
        .await
        .unwrap();
        server.await.unwrap();
        let body = axum::body::to_bytes(response.into_body(), 1024)
            .await
            .unwrap();

        assert_eq!(&body[..], b"flac");
    }

    #[tokio::test]
    async fn track_stream_rejects_tracks_without_subsonic_id() {
        let pool = test_pool().await;
        sqlx::query("INSERT INTO tracks(id,title) VALUES(1,'Local Only')")
            .execute(&pool)
            .await
            .unwrap();

        let settings = SettingsService;
        let result = fetch_track_stream(&pool, &settings, 1, None, false, &HeaderMap::new()).await;

        assert!(matches!(result, Err(TrackStreamError::NotStreamable)));
    }

    #[tokio::test]
    async fn track_playback_uses_now_playing_and_completed_scrobble_submissions() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            for expected_submission in ["submission=false", "submission=true"] {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut buffer = [0_u8; 4096];
                let read = socket.read(&mut buffer).await.unwrap();
                let request = String::from_utf8_lossy(&buffer[..read]);
                assert!(request.starts_with("GET /rest/scrobble?"));
                assert!(request.contains("id=subsonic-track"));
                assert!(request.contains(expected_submission));

                let body = br#"{"subsonic-response":{"status":"ok"}}"#;
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n",
                    body.len()
                );
                socket.write_all(response.as_bytes()).await.unwrap();
                socket.write_all(body).await.unwrap();
            }
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
        sqlx::query(
            "INSERT INTO tracks(id,subsonic_id,title)
             VALUES(1,'subsonic-track','Song')",
        )
        .execute(&pool)
        .await
        .unwrap();

        register_track_now_playing(&pool, 1).await.unwrap();
        register_track_scrobble(&pool, 1).await.unwrap();
        server.await.unwrap();
    }
}
