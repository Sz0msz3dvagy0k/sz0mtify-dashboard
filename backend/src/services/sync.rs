use anyhow::{anyhow, Context};
use reqwest::Client;
use serde_json::json;
use std::env;
use tracing::{debug, info, warn};

#[derive(Clone)]
pub struct SyncService;
impl SyncService {
    pub fn new() -> Self {
        Self
    }
    pub async fn sync_subsonic(&self, pool: &sqlx::SqlitePool) -> anyhow::Result<()> {
        info!("starting Subsonic sync");
        let cfg = SubsonicConfig::load(pool).await?;
        let client = Client::new();
        let albums = fetch_subsonic_album_list(&client, &cfg).await?;
        let mut imported_tracks = 0_i64;
        for album in albums {
            imported_tracks += import_subsonic_album(pool, &client, &cfg, &album).await?;
        }
        write_sync_state(pool, 1, "subsonic", "ok", None).await?;
        info!(imported_tracks, "completed Subsonic sync");
        Ok(())
    }
    pub async fn sync_lastfm(&self, pool: &sqlx::SqlitePool) -> anyhow::Result<()> {
        info!("starting Last.fm metadata sync");
        let api_key = setting_or_env(pool, "lastfm_api_key", "LASTFM_API_KEY")
            .await?
            .ok_or_else(|| anyhow!("missing Last.fm API key (settings.lastfm_api_key or LASTFM_API_KEY)"))?;
        let client = Client::new();
        let artists = sqlx::query_as::<_, (i64, String)>("SELECT id, name FROM artists")
            .fetch_all(pool)
            .await?;
        let mut updated = 0_i64;
        for (artist_id, name) in artists {
            if sync_lastfm_artist(pool, &client, &api_key, artist_id, &name).await? {
                updated += 1;
            }
        }
        write_sync_state(pool, 2, "lastfm", "ok", None).await?;
        info!(updated_artists = updated, "completed Last.fm sync");
        Ok(())
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
struct SubsonicConfig {
    base_url: String,
    username: String,
    password: String,
}

impl SubsonicConfig {
    async fn load(pool: &sqlx::SqlitePool) -> anyhow::Result<Self> {
        let base_url = setting_or_env(pool, "subsonic_url", "SUBSONIC_URL")
            .await?
            .ok_or_else(|| anyhow!("missing subsonic url (settings.subsonic_url or SUBSONIC_URL)"))?;
        let username = setting_or_env(pool, "subsonic_username", "SUBSONIC_USERNAME")
            .await?
            .ok_or_else(|| anyhow!("missing subsonic username"))?;
        let password = setting_or_env(pool, "subsonic_password", "SUBSONIC_PASSWORD")
            .await?
            .ok_or_else(|| anyhow!("missing subsonic password"))?;
        Ok(Self { base_url, username, password })
    }
}

async fn setting_or_env(pool: &sqlx::SqlitePool, key: &str, env_key: &str) -> anyhow::Result<Option<String>> {
    let value: Option<(String,)> = sqlx::query_as("SELECT value FROM settings WHERE key = ?")
        .bind(key)
        .fetch_optional(pool)
        .await?;
    if let Some((v,)) = value {
        return Ok(Some(v.trim_matches('"').to_string()));
    }
    Ok(env::var(env_key).ok())
}

async fn write_sync_state(pool: &sqlx::SqlitePool, id: i64, source: &str, status: &str, error_message: Option<&str>) -> anyhow::Result<()> {
    sqlx::query("INSERT OR REPLACE INTO sync_state (id,source,last_sync_at,status,error_message) VALUES (?,?,datetime('now'),?,?)")
        .bind(id)
        .bind(source)
        .bind(status)
        .bind(error_message)
        .execute(pool)
        .await?;
    Ok(())
}

async fn fetch_subsonic_album_list(client: &Client, cfg: &SubsonicConfig) -> anyhow::Result<Vec<serde_json::Value>> {
    let url = format!("{}/rest/getAlbumList2", cfg.base_url.trim_end_matches('/'));
    let page_size = 500_i64;
    let mut offset = 0_i64;
    let mut all_albums = Vec::new();

    loop {
        let response = client
            .get(&url)
            .query(&[
                ("u", cfg.username.as_str()),
                ("p", cfg.password.as_str()),
                ("v", "1.16.1"),
                ("c", "music-dashboard"),
                ("f", "json"),
                ("type", "newest"),
                ("size", "500"),
                ("offset", &offset.to_string()),
            ])
            .send()
            .await?
            .error_for_status()?
            .json::<serde_json::Value>()
            .await?;

        let albums = response["subsonic-response"]["albumList2"]["album"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        let page_count = albums.len() as i64;
        all_albums.extend(albums);

        if page_count < page_size {
            break;
        }
        offset += page_size;
    }

    Ok(all_albums)
}

async fn import_subsonic_album(pool: &sqlx::SqlitePool, client: &Client, cfg: &SubsonicConfig, album: &serde_json::Value) -> anyhow::Result<i64> {
    let album_id = album["id"].as_str().unwrap_or_default();
    if album_id.is_empty() {
        return Ok(0);
    }
    let artist_name = album["artist"].as_str().unwrap_or("Unknown Artist");
    sqlx::query("INSERT INTO artists(name,subsonic_id) VALUES(?,?) ON CONFLICT(subsonic_id) DO UPDATE SET name=excluded.name")
        .bind(artist_name)
        .bind(album["artistId"].as_str())
        .execute(pool)
        .await?;
    let artist_db_id: (i64,) = sqlx::query_as("SELECT id FROM artists WHERE subsonic_id = ?")
        .bind(album["artistId"].as_str())
        .fetch_one(pool)
        .await?;

    sqlx::query("INSERT INTO albums(subsonic_id,title,artist_id,year,genre,song_count,cover_art_id) VALUES(?,?,?,?,?,?,?) ON CONFLICT(subsonic_id) DO UPDATE SET title=excluded.title, artist_id=excluded.artist_id, year=excluded.year, genre=excluded.genre, song_count=excluded.song_count, cover_art_id=excluded.cover_art_id")
        .bind(album_id)
        .bind(album["name"].as_str().unwrap_or("Unknown Album"))
        .bind(artist_db_id.0)
        .bind(album["year"].as_i64())
        .bind(album["genre"].as_str())
        .bind(album["songCount"].as_i64().unwrap_or(0))
        .bind(album["coverArt"].as_str())
        .execute(pool)
        .await?;

    let album_db_id: (i64,) = sqlx::query_as("SELECT id FROM albums WHERE subsonic_id = ?")
        .bind(album_id)
        .fetch_one(pool)
        .await?;

    let url = format!("{}/rest/getAlbum", cfg.base_url.trim_end_matches('/'));
    let response = client
        .get(url)
        .query(&[
            ("u", cfg.username.as_str()),
            ("p", cfg.password.as_str()),
            ("v", "1.16.1"),
            ("c", "music-dashboard"),
            ("f", "json"),
            ("id", album_id),
        ])
        .send()
        .await?
        .error_for_status()?
        .json::<serde_json::Value>()
        .await
        .context("failed to parse getAlbum response")?;
    let songs = response["subsonic-response"]["album"]["song"].as_array().cloned().unwrap_or_default();
    let mut inserted = 0_i64;
    for song in songs {
        sqlx::query("INSERT INTO tracks(subsonic_id,title,artist_id,album_id,duration_seconds,track_number,disc_number,year,genre,file_path,suffix,content_type,size_bytes,bit_rate,sampling_rate,play_count,created_at) VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,datetime('now')) ON CONFLICT(subsonic_id) DO UPDATE SET title=excluded.title, artist_id=excluded.artist_id, album_id=excluded.album_id, duration_seconds=excluded.duration_seconds, track_number=excluded.track_number, disc_number=excluded.disc_number, year=excluded.year, genre=excluded.genre, file_path=excluded.file_path, suffix=excluded.suffix, content_type=excluded.content_type, size_bytes=excluded.size_bytes, bit_rate=excluded.bit_rate, sampling_rate=excluded.sampling_rate, play_count=excluded.play_count")
            .bind(song["id"].as_str())
            .bind(song["title"].as_str().unwrap_or("Unknown Track"))
            .bind(artist_db_id.0)
            .bind(album_db_id.0)
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
            .bind(song["samplingRate"].as_i64())
            .bind(song["playCount"].as_i64().unwrap_or(0))
            .execute(pool)
            .await?;
        inserted += 1;
    }
    Ok(inserted)
}

async fn sync_lastfm_artist(pool: &sqlx::SqlitePool, client: &Client, api_key: &str, artist_id: i64, artist_name: &str) -> anyhow::Result<bool> {
    let response = client
        .get("https://ws.audioscrobbler.com/2.0/")
        .query(&[
            ("method", "artist.getinfo"),
            ("artist", artist_name),
            ("api_key", api_key),
            ("format", "json"),
        ])
        .send()
        .await?
        .error_for_status()?
        .json::<serde_json::Value>()
        .await?;
    let artist = &response["artist"];
    if artist.is_null() {
        warn!(artist_name, "lastfm artist not found");
        return Ok(false);
    }
    let url = artist["url"].as_str();
    let mbid = artist["mbid"].as_str();
    let listeners = artist["stats"]["listeners"].as_str().and_then(|s| s.parse::<i64>().ok());
    let playcount = artist["stats"]["playcount"].as_str().and_then(|s| s.parse::<i64>().ok());
    let bio = artist["bio"]["summary"].as_str();
    sqlx::query("UPDATE artists SET lastfm_artist_url=?, lastfm_listeners=?, lastfm_playcount=?, mbid=COALESCE(NULLIF(?, ''), mbid), bio_summary=COALESCE(?, bio_summary), updated_at=datetime('now') WHERE id=?")
        .bind(url)
        .bind(listeners)
        .bind(playcount)
        .bind(mbid)
        .bind(bio)
        .bind(artist_id)
        .execute(pool)
        .await?;
    Ok(true)
}
