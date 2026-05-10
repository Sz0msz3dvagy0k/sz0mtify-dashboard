use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::services::AppState;

#[derive(Deserialize)]
pub struct SearchQ {
    pub q: String,
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
    match state.sync.sync_subsonic(&state.pool).await {
        Ok(imported_tracks) => (
            StatusCode::OK,
            ok(json!({"source": "subsonic", "imported_tracks": imported_tracks})),
        ),
        Err(error) => (
            StatusCode::BAD_GATEWAY,
            err(&format!("failed_to_sync_subsonic: {error}")),
        ),
    }
}

pub async fn sync_lastfm(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.sync.sync_lastfm(&state.pool).await {
        Ok(updated_artists) => (
            StatusCode::OK,
            ok(json!({"source": "lastfm", "updated_artists": updated_artists})),
        ),
        Err(error) => (
            StatusCode::BAD_GATEWAY,
            err(&format!("failed_to_sync_lastfm: {error}")),
        ),
    }
}

pub async fn sync_all(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let subsonic_result = state.sync.sync_subsonic(&state.pool).await;
    let lastfm_result = state.sync.sync_lastfm(&state.pool).await;
    let track_count = state
        .sync
        .track_count(&state.pool)
        .await
        .unwrap_or_default();
    let ok = subsonic_result.is_ok() && lastfm_result.is_ok();
    let status = if ok {
        StatusCode::OK
    } else {
        StatusCode::BAD_GATEWAY
    };

    let subsonic_error = subsonic_result.as_ref().err().map(ToString::to_string);
    let lastfm_error = lastfm_result.as_ref().err().map(ToString::to_string);
    let subsonic_imported_tracks = subsonic_result.unwrap_or_default();
    let lastfm_updated_artists = lastfm_result.unwrap_or_default();

    (
        status,
        Json(json!({
            "ok": ok,
            "data": {
                "subsonic": subsonic_error.is_none(),
                "lastfm": lastfm_error.is_none(),
                "track_count": track_count,
                "subsonic_imported_tracks": subsonic_imported_tracks,
                "lastfm_updated_artists": lastfm_updated_artists,
                "errors": {
                    "subsonic": subsonic_error,
                    "lastfm": lastfm_error
                }
            }
        })),
    )
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
pub async fn new_releases(State(state): State<Arc<AppState>>) -> Json<Value> {
    respond_service!(
        state.discovery.new_releases(&state.pool),
        "failed_to_load_new_releases"
    )
}
pub async fn missing_albums(State(state): State<Arc<AppState>>) -> Json<Value> {
    respond_service!(
        state.discovery.missing_albums(&state.pool),
        "failed_to_load_missing_albums"
    )
}
pub async fn similar_artists(State(state): State<Arc<AppState>>) -> Json<Value> {
    respond_service!(
        state.discovery.similar_artists(&state.pool),
        "failed_to_load_similar_artists"
    )
}

pub async fn refresh_discovery(State(state): State<Arc<AppState>>) -> Json<Value> {
    match state.discovery.refresh(&state.pool).await {
        Ok(_) => ok(json!({"refreshed": true})),
        Err(_) => err("failed_to_refresh_discovery"),
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
) -> Json<Value> {
    let album_cover = sqlx::query_as::<_, (i64, String, Option<String>)>(
        "SELECT id, title, cover_art_url FROM albums WHERE cover_art_id = ? LIMIT 1",
    )
    .bind(&cover_art_id)
    .fetch_optional(&state.pool)
    .await;

    match album_cover {
        Ok(Some((album_id, title, cover_art_url))) => ok(
            json!({"cover_art_id": cover_art_id, "album_id": album_id, "album_title": title, "cover_art_url": cover_art_url}),
        ),
        Ok(None) => ok(
            json!({"cover_art_id": cover_art_id, "cover_art_url": format!("/mock/cover/{cover_art_id}")}),
        ),
        Err(_) => err("failed_to_load_cover"),
    }
}
