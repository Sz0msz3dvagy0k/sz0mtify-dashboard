use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::services::AppState;

#[derive(Serialize)]
pub struct ApiResponse<T> {
    pub data: T,
}
#[derive(Deserialize)]
pub struct SearchQ {
    pub q: String,
}

pub async fn health() -> Json<Value> {
    Json(json!({"status":"ok"}))
}
pub async fn get_settings(State(state): State<Arc<AppState>>) -> Json<Value> {
    Json(json!({"data": state.settings.get_all(&state.pool).await.unwrap_or_default()}))
}
pub async fn save_settings(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<Value>,
) -> Json<Value> {
    state.settings.save(&state.pool, payload.clone()).await.ok();
    Json(json!({"ok":true,"data":payload}))
}
pub async fn sync_subsonic(State(state): State<Arc<AppState>>) -> Json<Value> {
    Json(json!({"ok": state.sync.sync_subsonic(&state.pool).await.is_ok()}))
}
pub async fn sync_lastfm(State(state): State<Arc<AppState>>) -> Json<Value> {
    Json(json!({"ok": state.sync.sync_lastfm(&state.pool).await.is_ok()}))
}
pub async fn sync_all(State(state): State<Arc<AppState>>) -> Json<Value> {
    state.sync.sync_subsonic(&state.pool).await.ok();
    state.sync.sync_lastfm(&state.pool).await.ok();
    Json(json!({"ok":true}))
}
pub async fn sync_status(State(state): State<Arc<AppState>>) -> Json<Value> {
    Json(json!({"data": state.sync.status(&state.pool).await.unwrap_or_default()}))
}
pub async fn library_overview(State(state): State<Arc<AppState>>) -> Json<Value> {
    Json(json!({"data": state.analytics.overview(&state.pool).await.unwrap()}))
}
pub async fn tracks(State(state): State<Arc<AppState>>) -> Json<Value> {
    Json(json!({"data": state.analytics.tracks(&state.pool).await.unwrap()}))
}
pub async fn albums(State(state): State<Arc<AppState>>) -> Json<Value> {
    Json(json!({"data": state.analytics.albums(&state.pool).await.unwrap()}))
}
pub async fn album_by_id(State(state): State<Arc<AppState>>, Path(id): Path<i64>) -> Json<Value> {
    Json(json!({"data": state.analytics.album_by_id(&state.pool,id).await.unwrap()}))
}
pub async fn artists(State(state): State<Arc<AppState>>) -> Json<Value> {
    Json(json!({"data": state.analytics.artists(&state.pool).await.unwrap()}))
}
pub async fn artist_by_id(State(state): State<Arc<AppState>>, Path(id): Path<i64>) -> Json<Value> {
    Json(json!({"data": state.analytics.artist_by_id(&state.pool,id).await.unwrap()}))
}
pub async fn genres(State(state): State<Arc<AppState>>) -> Json<Value> {
    Json(json!({"data": state.analytics.genres(&state.pool).await.unwrap()}))
}
pub async fn audio_quality(State(state): State<Arc<AppState>>) -> Json<Value> {
    Json(json!({"data": state.analytics.audio_quality(&state.pool).await.unwrap()}))
}
pub async fn storage(State(state): State<Arc<AppState>>) -> Json<Value> {
    Json(json!({"data": state.analytics.storage(&state.pool).await.unwrap()}))
}
pub async fn metadata_health(State(state): State<Arc<AppState>>) -> Json<Value> {
    Json(json!({"data": state.analytics.metadata_health(&state.pool).await.unwrap()}))
}
pub async fn listening(State(state): State<Arc<AppState>>) -> Json<Value> {
    Json(json!({"data": state.analytics.listening(&state.pool).await.unwrap()}))
}
pub async fn timeline(State(state): State<Arc<AppState>>) -> Json<Value> {
    Json(json!({"data": state.analytics.timeline(&state.pool).await.unwrap()}))
}
pub async fn new_releases(State(state): State<Arc<AppState>>) -> Json<Value> {
    Json(json!({"data": state.discovery.new_releases(&state.pool).await.unwrap()}))
}
pub async fn missing_albums(State(state): State<Arc<AppState>>) -> Json<Value> {
    Json(json!({"data": state.discovery.missing_albums(&state.pool).await.unwrap()}))
}
pub async fn similar_artists(State(state): State<Arc<AppState>>) -> Json<Value> {
    Json(json!({"data": state.discovery.similar_artists(&state.pool).await.unwrap()}))
}
pub async fn refresh_discovery(State(state): State<Arc<AppState>>) -> Json<Value> {
    state.discovery.refresh(&state.pool).await.ok();
    Json(json!({"ok":true}))
}
pub async fn rediscovery(State(state): State<Arc<AppState>>) -> Json<Value> {
    Json(json!({"data": state.recommendations.rediscovery(&state.pool).await.unwrap()}))
}
pub async fn current_rotation(State(state): State<Arc<AppState>>) -> Json<Value> {
    Json(json!({"data": state.recommendations.current_rotation(&state.pool).await.unwrap()}))
}
pub async fn favorites(State(state): State<Arc<AppState>>) -> Json<Value> {
    Json(json!({"data": state.recommendations.favorites(&state.pool).await.unwrap()}))
}
pub async fn search(
    State(state): State<Arc<AppState>>,
    Query(params): Query<SearchQ>,
) -> Json<Value> {
    Json(json!({"data": state.analytics.search(&state.pool,&params.q).await.unwrap()}))
}
pub async fn cover(Path(id): Path<String>) -> Json<Value> {
    Json(json!({"cover_art_id": id, "url": format!("/mock/cover/{id}")}))
}
