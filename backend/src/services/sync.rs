use anyhow::{anyhow, Context};
use reqwest::Client;
use serde_json::json;
use std::env;
use tracing::{debug, info, warn};

use crate::services::lastfm::LastfmClient;

#[derive(Clone)]
pub struct SyncService;
impl SyncService {
    pub fn new() -> Self {
        Self
    }
    pub async fn sync_subsonic(&self, pool: &sqlx::SqlitePool) -> anyhow::Result<i64> {
        info!("starting Subsonic sync");
        let result: anyhow::Result<i64> = async {
            let cfg = SubsonicConfig::load(pool).await?;
            let client = Client::new();
            let albums = fetch_subsonic_album_list(&client, &cfg).await?;
            let mut imported_tracks = 0_i64;
            for album in albums {
                imported_tracks += import_subsonic_album(pool, &client, &cfg, &album).await?;
            }
            refresh_library_rollups(pool).await?;
            write_sync_state(pool, 1, "subsonic", "ok", None).await?;
            info!(imported_tracks, "completed Subsonic sync");
            Ok(imported_tracks)
        }
        .await;

        if let Err(error) = &result {
            warn!(error = %error, "Subsonic sync failed");
            if let Err(state_error) =
                write_sync_state(pool, 1, "subsonic", "error", Some(&error.to_string())).await
            {
                warn!(error = %state_error, "failed to write Subsonic sync error state");
            }
        }

        result
    }
    pub async fn sync_lastfm(&self, pool: &sqlx::SqlitePool) -> anyhow::Result<i64> {
        info!("starting Last.fm metadata sync");
        let result: anyhow::Result<i64> = async {
            setting_or_env(pool, "lastfm_api_key", "LASTFM_API_KEY")
                .await?
                .ok_or_else(|| {
                    anyhow!("missing Last.fm API key (settings.lastfm_api_key or LASTFM_API_KEY)")
                })?;
            let client = LastfmClient::new();
            let artists = sqlx::query_as::<_, (i64, String)>("SELECT id, name FROM artists")
                .fetch_all(pool)
                .await?;
            let mut updated = 0_i64;
            for (artist_id, name) in artists {
                if sync_lastfm_artist(pool, &client, artist_id, &name).await? {
                    updated += 1;
                }
            }
            write_sync_state(pool, 2, "lastfm", "ok", None).await?;
            info!(updated_artists = updated, "completed Last.fm sync");
            Ok(updated)
        }
        .await;

        if let Err(error) = &result {
            warn!(error = %error, "Last.fm sync failed");
            if let Err(state_error) =
                write_sync_state(pool, 2, "lastfm", "error", Some(&error.to_string())).await
            {
                warn!(error = %state_error, "failed to write Last.fm sync error state");
            }
        }

        result
    }
    pub async fn status(&self, pool: &sqlx::SqlitePool) -> anyhow::Result<serde_json::Value> {
        let rows = sqlx::query_as::<_, (i64, String, String, String)>(
            "SELECT id,source,COALESCE(last_sync_at,''),COALESCE(status,'unknown') FROM sync_state",
        )
        .fetch_all(pool)
        .await?;
        Ok(json!(rows))
    }

    pub async fn track_count(&self, pool: &sqlx::SqlitePool) -> anyhow::Result<i64> {
        debug!("loading track count from database");
        let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM tracks")
            .fetch_one(pool)
            .await?;
        info!(track_count = count, "loaded track count");
        Ok(count)
    }
}

#[derive(Clone, Debug)]
pub(crate) struct SubsonicConfig {
    pub(crate) base_url: String,
    username: String,
    password: String,
    api_version: String,
}

impl SubsonicConfig {
    pub(crate) async fn load(pool: &sqlx::SqlitePool) -> anyhow::Result<Self> {
        let base_url = setting_or_env_any(
            pool,
            &["subsonic_base_url", "subsonic_url"],
            &["SUBSONIC_BASE_URL", "SUBSONIC_URL"],
        )
        .await?
        .ok_or_else(|| {
            anyhow!(
                "missing Subsonic base URL (settings.subsonic_base_url, settings.subsonic_url, SUBSONIC_BASE_URL, or SUBSONIC_URL)"
            )
        })?;
        let username = setting_or_env(pool, "subsonic_username", "SUBSONIC_USERNAME")
            .await?
            .ok_or_else(|| anyhow!("missing subsonic username"))?;
        let password = setting_or_env(pool, "subsonic_password", "SUBSONIC_PASSWORD")
            .await?
            .ok_or_else(|| anyhow!("missing subsonic password"))?;
        let api_version = setting_or_env(pool, "subsonic_api_version", "SUBSONIC_API_VERSION")
            .await?
            .unwrap_or_else(|| "1.16.1".to_string());
        Ok(Self {
            base_url,
            username,
            password,
            api_version,
        })
    }
}

pub(crate) async fn setting_or_env(
    pool: &sqlx::SqlitePool,
    key: &str,
    env_key: &str,
) -> anyhow::Result<Option<String>> {
    setting_or_env_any(pool, &[key], &[env_key]).await
}

async fn setting_or_env_any(
    pool: &sqlx::SqlitePool,
    keys: &[&str],
    env_keys: &[&str],
) -> anyhow::Result<Option<String>> {
    for key in keys {
        if let Some(value) = setting_value(pool, key).await? {
            return Ok(Some(value));
        }
    }

    for env_key in env_keys {
        if let Ok(value) = env::var(env_key) {
            let value = value.trim().to_string();
            if !value.is_empty() {
                return Ok(Some(value));
            }
        }
    }

    Ok(None)
}

async fn setting_value(pool: &sqlx::SqlitePool, key: &str) -> anyhow::Result<Option<String>> {
    let value: Option<(String,)> = sqlx::query_as("SELECT value FROM settings WHERE key = ?")
        .bind(key)
        .fetch_optional(pool)
        .await?;
    if let Some((v,)) = value {
        let parsed = serde_json::from_str::<String>(&v).unwrap_or(v);
        let parsed = parsed.trim().to_string();
        if !parsed.is_empty() {
            return Ok(Some(parsed));
        }
    }
    Ok(None)
}

async fn write_sync_state(
    pool: &sqlx::SqlitePool,
    id: i64,
    source: &str,
    status: &str,
    error_message: Option<&str>,
) -> anyhow::Result<()> {
    sqlx::query("INSERT OR REPLACE INTO sync_state (id,source,last_sync_at,status,error_message) VALUES (?,?,datetime('now'),?,?)")
        .bind(id)
        .bind(source)
        .bind(status)
        .bind(error_message)
        .execute(pool)
        .await?;
    Ok(())
}

async fn fetch_subsonic_album_list(
    client: &Client,
    cfg: &SubsonicConfig,
) -> anyhow::Result<Vec<serde_json::Value>> {
    let url = format!("{}/rest/getAlbumList2", cfg.base_url.trim_end_matches('/'));
    let page_size = 500_i64;
    let mut offset = 0_i64;
    let mut all_albums = Vec::new();

    loop {
        let response = subsonic_get_json(
            client,
            &url,
            cfg,
            &[
                ("type", "newest".to_string()),
                ("size", page_size.to_string()),
                ("offset", offset.to_string()),
            ],
            "getAlbumList2",
        )
        .await?;

        let album_list = response
            .get("subsonic-response")
            .and_then(|root| root.get("albumList2"))
            .ok_or_else(|| {
                anyhow!("invalid Subsonic getAlbumList2 response: missing albumList2")
            })?;
        let albums = value_list(album_list.get("album"));
        let page_count = albums.len() as i64;
        all_albums.extend(albums);

        if page_count < page_size {
            break;
        }
        offset += page_size;
    }

    Ok(all_albums)
}

async fn import_subsonic_album(
    pool: &sqlx::SqlitePool,
    client: &Client,
    cfg: &SubsonicConfig,
    album: &serde_json::Value,
) -> anyhow::Result<i64> {
    let album_id = album["id"].as_str().unwrap_or_default();
    if album_id.is_empty() {
        return Ok(0);
    }
    let artist_name = album["artist"].as_str().unwrap_or("Unknown Artist");
    let artist_db_id = upsert_artist(pool, album["artistId"].as_str(), artist_name).await?;

    sqlx::query("INSERT INTO albums(subsonic_id,title,artist_id,album_artist_id,year,genre,song_count,duration_seconds,size_bytes,cover_art_id,play_count) VALUES(?,?,?,?,?,?,?,?,?,?,?) ON CONFLICT(subsonic_id) DO UPDATE SET title=excluded.title, artist_id=excluded.artist_id, album_artist_id=excluded.album_artist_id, year=excluded.year, genre=excluded.genre, song_count=excluded.song_count, duration_seconds=excluded.duration_seconds, size_bytes=excluded.size_bytes, cover_art_id=excluded.cover_art_id, play_count=excluded.play_count, updated_at=datetime('now')")
        .bind(album_id)
        .bind(album["name"].as_str().unwrap_or("Unknown Album"))
        .bind(artist_db_id)
        .bind(artist_db_id)
        .bind(album["year"].as_i64())
        .bind(album["genre"].as_str())
        .bind(album["songCount"].as_i64().unwrap_or(0))
        .bind(album["duration"].as_i64().unwrap_or(0))
        .bind(album["size"].as_i64().unwrap_or(0))
        .bind(album["coverArt"].as_str())
        .bind(album["playCount"].as_i64().unwrap_or(0))
        .execute(pool)
        .await?;

    let album_db_id: (i64,) = sqlx::query_as("SELECT id FROM albums WHERE subsonic_id = ?")
        .bind(album_id)
        .fetch_one(pool)
        .await?;

    let url = format!("{}/rest/getAlbum", cfg.base_url.trim_end_matches('/'));
    let response = subsonic_get_json(
        client,
        &url,
        cfg,
        &[("id", album_id.to_string())],
        "getAlbum",
    )
    .await?;
    let songs = value_list(response["subsonic-response"]["album"].get("song"));
    let mut inserted = 0_i64;
    for song in songs {
        let track_artist_name = song["artist"].as_str().unwrap_or(artist_name);
        let track_artist_id = upsert_artist(
            pool,
            song["artistId"].as_str().or(album["artistId"].as_str()),
            track_artist_name,
        )
        .await?;

        let mbid = track_mbid(&song);
        sqlx::query("INSERT INTO tracks(subsonic_id,title,artist_id,album_id,album_artist_id,duration_seconds,track_number,disc_number,year,genre,file_path,suffix,content_type,size_bytes,bit_rate,bit_depth,sampling_rate,channel_count,play_count,mbid,created_at) VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,datetime('now')) ON CONFLICT(subsonic_id) DO UPDATE SET title=excluded.title, artist_id=excluded.artist_id, album_id=excluded.album_id, album_artist_id=excluded.album_artist_id, duration_seconds=excluded.duration_seconds, track_number=excluded.track_number, disc_number=excluded.disc_number, year=excluded.year, genre=excluded.genre, file_path=excluded.file_path, suffix=excluded.suffix, content_type=excluded.content_type, size_bytes=excluded.size_bytes, bit_rate=excluded.bit_rate, bit_depth=excluded.bit_depth, sampling_rate=excluded.sampling_rate, channel_count=excluded.channel_count, play_count=excluded.play_count, mbid=COALESCE(NULLIF(excluded.mbid, ''), tracks.mbid), updated_at=datetime('now')")
            .bind(song["id"].as_str())
            .bind(song["title"].as_str().unwrap_or("Unknown Track"))
            .bind(track_artist_id)
            .bind(album_db_id.0)
            .bind(artist_db_id)
            .bind(song["duration"].as_i64().unwrap_or(0))
            .bind(song["track"].as_i64())
            .bind(song["discNumber"].as_i64())
            .bind(song["year"].as_i64())
            .bind(song["genre"].as_str())
            .bind(song["path"].as_str())
            .bind(song["suffix"].as_str())
            .bind(song["contentType"].as_str())
            .bind(song["size"].as_i64().unwrap_or(0))
            .bind(song["bitRate"].as_i64())
            .bind(song["bitDepth"].as_i64())
            .bind(song["samplingRate"].as_i64())
            .bind(song["channelCount"].as_i64())
            .bind(song["playCount"].as_i64().unwrap_or(0))
            .bind(mbid)
            .execute(pool)
            .await?;
        inserted += 1;
    }
    Ok(inserted)
}

async fn sync_lastfm_artist(
    pool: &sqlx::SqlitePool,
    client: &LastfmClient,
    artist_id: i64,
    artist_name: &str,
) -> anyhow::Result<bool> {
    let response = client.artist_get_info(pool, artist_name).await?;
    if let Some(code) = response.get("error") {
        if code.as_i64() == Some(6) {
            warn!(artist_name, "lastfm artist not found");
            return Ok(false);
        }
        let message = response["message"]
            .as_str()
            .unwrap_or("Last.fm returned an error without a message");
        return Err(anyhow!("Last.fm API error {code}: {message}"));
    }
    let artist = &response["artist"];
    if artist.is_null() {
        warn!(artist_name, "lastfm artist not found");
        return Ok(false);
    }
    let url = artist["url"].as_str();
    let mbid = artist["mbid"].as_str();
    let listeners = artist["stats"]["listeners"]
        .as_str()
        .and_then(|s| s.parse::<i64>().ok());
    let playcount = artist["stats"]["playcount"]
        .as_str()
        .and_then(|s| s.parse::<i64>().ok());
    let bio = clean_lastfm_bio(
        artist["bio"]["summary"].as_str(),
        artist["bio"]["content"].as_str(),
    );
    let image_url = lastfm_image_url(artist);
    sqlx::query("UPDATE artists SET lastfm_artist_url=?, lastfm_listeners=?, lastfm_playcount=?, mbid=COALESCE(NULLIF(?, ''), mbid), image_url=COALESCE(?, image_url), bio_summary=COALESCE(?, bio_summary), updated_at=datetime('now') WHERE id=?")
        .bind(url)
        .bind(listeners)
        .bind(playcount)
        .bind(mbid)
        .bind(image_url)
        .bind(bio)
        .bind(artist_id)
        .execute(pool)
        .await?;
    Ok(true)
}

fn lastfm_image_url(artist: &serde_json::Value) -> Option<String> {
    artist
        .get("image")
        .and_then(|image| image.as_array())
        .and_then(|images| {
            images.iter().rev().find_map(|image| {
                image
                    .get("#text")
                    .and_then(|url| url.as_str())
                    .map(str::trim)
                    .filter(|url| {
                        !url.is_empty()
                            && !url.contains("2a96cbd8b46e442fc41c2b86b821562f")
                            && !url.contains("c6f59c1e5e7240a4c0d427abd71f3dbb")
                    })
                    .map(ToString::to_string)
            })
        })
}

fn clean_lastfm_bio(summary: Option<&str>, content: Option<&str>) -> Option<String> {
    let raw = content
        .and_then(non_empty)
        .or_else(|| summary.and_then(non_empty))?;
    let without_read_more = raw
        .split("Read more on Last.fm")
        .next()
        .unwrap_or(raw)
        .split("User-contributed text is available")
        .next()
        .unwrap_or(raw);
    let cleaned = collapse_whitespace(&strip_html(without_read_more));
    non_empty(&cleaned).map(ToString::to_string)
}

fn non_empty(value: &str) -> Option<&str> {
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

fn strip_html(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut in_tag = false;
    let mut entity = String::new();
    let mut in_entity = false;

    for character in value.chars() {
        match character {
            '<' => in_tag = true,
            '>' => in_tag = false,
            '&' if !in_tag => {
                entity.clear();
                in_entity = true;
            }
            ';' if in_entity => {
                output.push_str(match entity.as_str() {
                    "amp" => "&",
                    "quot" => "\"",
                    "apos" | "#39" => "'",
                    "lt" => "<",
                    "gt" => ">",
                    "nbsp" => " ",
                    _ => "",
                });
                entity.clear();
                in_entity = false;
            }
            _ if in_tag => {}
            _ if in_entity => entity.push(character),
            _ => output.push(character),
        }
    }

    if in_entity {
        output.push('&');
        output.push_str(&entity);
    }

    output
}

fn collapse_whitespace(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub(crate) fn subsonic_auth_query(cfg: &SubsonicConfig) -> Vec<(String, String)> {
    vec![
        ("u".to_string(), cfg.username.clone()),
        ("p".to_string(), cfg.password.clone()),
        ("v".to_string(), cfg.api_version.clone()),
        ("c".to_string(), "music-dashboard".to_string()),
        ("f".to_string(), "json".to_string()),
    ]
}

async fn subsonic_get_json(
    client: &Client,
    url: &str,
    cfg: &SubsonicConfig,
    params: &[(&str, String)],
    endpoint: &str,
) -> anyhow::Result<serde_json::Value> {
    let mut query = subsonic_auth_query(cfg);
    query.extend(
        params
            .iter()
            .map(|(key, value)| ((*key).to_string(), value.clone())),
    );

    let response = client
        .get(url)
        .query(&query)
        .send()
        .await
        .with_context(|| format!("failed to send Subsonic {endpoint} request"))?
        .error_for_status()
        .with_context(|| format!("Subsonic {endpoint} request returned an HTTP error"))?
        .json::<serde_json::Value>()
        .await
        .with_context(|| format!("failed to parse Subsonic {endpoint} response"))?;

    validate_subsonic_response(&response, endpoint)?;
    Ok(response)
}

fn validate_subsonic_response(response: &serde_json::Value, endpoint: &str) -> anyhow::Result<()> {
    let root = response.get("subsonic-response").ok_or_else(|| {
        anyhow!("invalid Subsonic {endpoint} response: missing subsonic-response")
    })?;
    let status = root
        .get("status")
        .and_then(|status| status.as_str())
        .ok_or_else(|| anyhow!("invalid Subsonic {endpoint} response: missing status"))?;

    if status == "ok" {
        return Ok(());
    }

    let error = root.get("error");
    let code = error
        .and_then(|error| error.get("code"))
        .map(|code| code.to_string())
        .unwrap_or_else(|| "unknown".to_string());
    let message = error
        .and_then(|error| error.get("message"))
        .and_then(|message| message.as_str())
        .unwrap_or("Subsonic returned an error without a message");

    Err(anyhow!("Subsonic {endpoint} API error {code}: {message}"))
}

fn value_list(value: Option<&serde_json::Value>) -> Vec<serde_json::Value> {
    match value {
        Some(serde_json::Value::Array(values)) => values.clone(),
        Some(serde_json::Value::Object(_)) => value.cloned().into_iter().collect(),
        _ => Vec::new(),
    }
}

fn track_mbid(song: &serde_json::Value) -> Option<String> {
    first_non_empty_string(
        song,
        &[
            "musicBrainzId",
            "musicBrainzRecordingId",
            "musicbrainz_recordingid",
            "MusicBrainz Recording Id",
        ],
    )
    .or_else(|| metadata_value(song.get("metadata"), "MusicBrainz Recording Id"))
}

fn first_non_empty_string(value: &serde_json::Value, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| non_empty(value.get(*key)?.as_str()?).map(ToString::to_string))
}

fn metadata_value(value: Option<&serde_json::Value>, label: &str) -> Option<String> {
    let metadata = value?;
    match metadata {
        serde_json::Value::Object(tags) => tags
            .iter()
            .find(|(key, _)| metadata_key_matches(key, label))
            .and_then(|(_, value)| non_empty(value.as_str()?).map(ToString::to_string)),
        serde_json::Value::Array(tags) => tags.iter().find_map(|tag| {
            let key = first_non_empty_string(tag, &["name", "key", "field", "label"])?;
            if !metadata_key_matches(&key, label) {
                return None;
            }
            first_non_empty_string(tag, &["value", "text"])
        }),
        _ => None,
    }
}

fn metadata_key_matches(key: &str, expected: &str) -> bool {
    normalize_metadata_key(key) == normalize_metadata_key(expected)
}

fn normalize_metadata_key(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

async fn upsert_artist(
    pool: &sqlx::SqlitePool,
    subsonic_id: Option<&str>,
    name: &str,
) -> anyhow::Result<i64> {
    let subsonic_id = subsonic_id.and_then(|id| {
        let id = id.trim();
        if id.is_empty() {
            None
        } else {
            Some(id)
        }
    });

    if let Some(subsonic_id) = subsonic_id {
        sqlx::query("INSERT INTO artists(name,subsonic_id) VALUES(?,?) ON CONFLICT(subsonic_id) DO UPDATE SET name=excluded.name, updated_at=datetime('now')")
            .bind(name)
            .bind(subsonic_id)
            .execute(pool)
            .await?;
        let (id,): (i64,) = sqlx::query_as("SELECT id FROM artists WHERE subsonic_id = ?")
            .bind(subsonic_id)
            .fetch_one(pool)
            .await?;
        return Ok(id);
    }

    if let Some((id,)) =
        sqlx::query_as::<_, (i64,)>("SELECT id FROM artists WHERE name = ? LIMIT 1")
            .bind(name)
            .fetch_optional(pool)
            .await?
    {
        return Ok(id);
    }

    let result = sqlx::query("INSERT INTO artists(name) VALUES(?)")
        .bind(name)
        .execute(pool)
        .await?;
    Ok(result.last_insert_rowid())
}

async fn refresh_library_rollups(pool: &sqlx::SqlitePool) -> anyhow::Result<()> {
    sqlx::query(
        "UPDATE albums
         SET song_count = (SELECT COUNT(*) FROM tracks WHERE tracks.album_id = albums.id),
             duration_seconds = COALESCE((SELECT SUM(duration_seconds) FROM tracks WHERE tracks.album_id = albums.id), 0),
             size_bytes = COALESCE((SELECT SUM(size_bytes) FROM tracks WHERE tracks.album_id = albums.id), 0),
             play_count = COALESCE((SELECT SUM(play_count) FROM tracks WHERE tracks.album_id = albums.id), 0),
             updated_at = datetime('now')",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "UPDATE artists
         SET album_count = (SELECT COUNT(*) FROM albums WHERE albums.artist_id = artists.id OR albums.album_artist_id = artists.id),
             track_count = (SELECT COUNT(*) FROM tracks WHERE tracks.artist_id = artists.id),
             play_count = COALESCE((SELECT SUM(play_count) FROM tracks WHERE tracks.artist_id = artists.id), 0),
             updated_at = datetime('now')",
    )
    .execute(pool)
    .await?;

    sqlx::query("DELETE FROM genres").execute(pool).await?;
    sqlx::query(
        "INSERT INTO genres(name, track_count, album_count, artist_count)
         SELECT genre,
                COUNT(*),
                COUNT(DISTINCT album_id),
                COUNT(DISTINCT artist_id)
         FROM tracks
         WHERE genre IS NOT NULL AND genre != ''
         GROUP BY genre",
    )
    .execute(pool)
    .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn validates_subsonic_api_errors() {
        let response = json!({
            "subsonic-response": {
                "status": "failed",
                "error": { "code": 40, "message": "Wrong username or password" }
            }
        });

        let error = validate_subsonic_response(&response, "ping")
            .expect_err("failed Subsonic responses must be surfaced");

        assert!(error.to_string().contains("Wrong username or password"));
    }

    #[test]
    fn accepts_single_object_lists_from_subsonic_json() {
        let value = json!({ "id": "one-track" });

        let list = value_list(Some(&value));

        assert_eq!(list, vec![value]);
    }

    #[test]
    fn extracts_track_mbid_from_subsonic_musicbrainz_id() {
        let song = json!({
            "id": "track-1",
            "musicBrainzId": "9f1b7f0f-4d44-41fc-89a8-8f6d3b5b7a4d"
        });

        assert_eq!(
            track_mbid(&song).as_deref(),
            Some("9f1b7f0f-4d44-41fc-89a8-8f6d3b5b7a4d")
        );
    }

    #[test]
    fn extracts_track_mbid_from_raw_metadata_object() {
        let song = json!({
            "id": "track-1",
            "metadata": {
                "MusicBrainz Recording Id": "3cbacb77-0e5e-42b7-8f4a-50d33f63ea2b"
            }
        });

        assert_eq!(
            track_mbid(&song).as_deref(),
            Some("3cbacb77-0e5e-42b7-8f4a-50d33f63ea2b")
        );
    }

    #[test]
    fn extracts_track_mbid_from_raw_metadata_array() {
        let song = json!({
            "id": "track-1",
            "metadata": [
                { "name": "Album", "value": "Example Album" },
                {
                    "name": "MusicBrainz Recording Id",
                    "value": "d577be68-5137-4bb2-b115-d3c1119913ef"
                }
            ]
        });

        assert_eq!(
            track_mbid(&song).as_deref(),
            Some("d577be68-5137-4bb2-b115-d3c1119913ef")
        );
    }

    #[test]
    fn cleans_lastfm_bio_text() {
        let bio = clean_lastfm_bio(
            Some("Artist &amp; friends <a href=\"https://last.fm\">Read more on Last.fm</a>"),
            None,
        );

        assert_eq!(bio.as_deref(), Some("Artist & friends"));
    }

    #[test]
    fn extracts_largest_non_empty_lastfm_artist_image() {
        let artist = json!({
            "image": [
                { "#text": "", "size": "small" },
                { "#text": "https://lastfm.freetls.fastly.net/i/u/300x300/avatar.jpg", "size": "large" }
            ]
        });

        assert_eq!(
            lastfm_image_url(&artist).as_deref(),
            Some("https://lastfm.freetls.fastly.net/i/u/300x300/avatar.jpg")
        );
    }
}
