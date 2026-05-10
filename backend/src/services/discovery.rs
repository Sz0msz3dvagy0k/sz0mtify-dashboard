use chrono::Utc;
use serde_json::{json, Value};
use sqlx::Row;
use tracing::warn;

use crate::services::lastfm::LastfmClient;
use crate::utils::matching::normalize_name;

#[derive(Clone)]
pub struct DiscoveryService;

#[derive(Clone, Copy, Debug)]
pub struct DiscoveryListOptions {
    pub limit: i64,
    pub offset: i64,
    pub include_owned: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct DiscoveryRefreshOptions {
    pub limit: i64,
}

impl Default for DiscoveryListOptions {
    fn default() -> Self {
        Self {
            limit: 100,
            offset: 0,
            include_owned: false,
        }
    }
}

impl Default for DiscoveryRefreshOptions {
    fn default() -> Self {
        Self { limit: 50 }
    }
}

#[derive(Debug)]
struct FavoriteArtist {
    id: i64,
    name: String,
}

#[derive(Debug)]
struct DiscoveryCandidate {
    local_artist_id: Option<i64>,
    local_artist_name: Option<String>,
    discovered_artist_name: String,
    title: String,
    release_type: String,
    release_date: Option<String>,
    external_url: Option<String>,
    cover_url: Option<String>,
    already_in_library: bool,
    match_status: String,
    confidence_score: f64,
    reason: String,
    source_artist_name: Option<String>,
    source_artist_id: Option<i64>,
    raw_json: Value,
}

#[derive(Default)]
struct RefreshCounters {
    analyzed_artists: i64,
    created_count: i64,
    updated_count: i64,
    skipped_count: i64,
    error_count: i64,
    errors: Vec<String>,
}

impl DiscoveryService {
    pub async fn new_releases(
        &self,
        p: &sqlx::SqlitePool,
        options: DiscoveryListOptions,
    ) -> anyhow::Result<serde_json::Value> {
        discovery_rows(
            p,
            "release_type IN ('album','track')",
            if options.include_owned {
                ""
            } else {
                "AND match_status IN ('missing','uncertain','possibly_in_library')"
            },
            options,
            "confidence_score DESC, title ASC",
        )
        .await
    }

    pub async fn missing_albums(
        &self,
        p: &sqlx::SqlitePool,
        options: DiscoveryListOptions,
    ) -> anyhow::Result<serde_json::Value> {
        discovery_rows(
            p,
            "release_type = 'album'",
            if options.include_owned {
                ""
            } else {
                "AND match_status IN ('missing','uncertain','possibly_in_library')"
            },
            options,
            "confidence_score DESC, title ASC",
        )
        .await
    }

    pub async fn similar_artists(
        &self,
        p: &sqlx::SqlitePool,
        options: DiscoveryListOptions,
    ) -> anyhow::Result<serde_json::Value> {
        discovery_rows(
            p,
            "release_type = 'artist'",
            if options.include_owned {
                ""
            } else {
                "AND COALESCE(already_in_library,0)=0"
            },
            options,
            "confidence_score DESC, discovered_artist_name ASC",
        )
        .await
    }

    pub async fn refresh(
        &self,
        p: &sqlx::SqlitePool,
        options: DiscoveryRefreshOptions,
    ) -> anyhow::Result<serde_json::Value> {
        let client = LastfmClient::new();
        let artists = favorite_artists(p, options.limit.clamp(1, 200)).await?;
        let mut counters = RefreshCounters::default();

        for artist in artists {
            counters.analyzed_artists += 1;
            if let Err(error) = refresh_artist(p, &client, &artist, &mut counters).await {
                counters.error_count += 1;
                counters.errors.push(format!("{}: {}", artist.name, error));
                warn!(artist = artist.name, error = %error, "discovery refresh failed for artist");
            }
        }

        Ok(json!({
            "analyzed_artists": counters.analyzed_artists,
            "created_count": counters.created_count,
            "updated_count": counters.updated_count,
            "skipped_count": counters.skipped_count,
            "error_count": counters.error_count,
            "errors": counters.errors,
            "generated_at": Utc::now().to_rfc3339()
        }))
    }
}

async fn favorite_artists(p: &sqlx::SqlitePool, limit: i64) -> anyhow::Result<Vec<FavoriteArtist>> {
    let rows = sqlx::query_as::<_, (i64, String)>(
        "SELECT ar.id, ar.name
         FROM artists ar
         LEFT JOIN tracks t ON t.artist_id = ar.id
         LEFT JOIN albums al ON al.artist_id = ar.id OR al.album_artist_id = ar.id
         GROUP BY ar.id, ar.name
         ORDER BY
           COALESCE(SUM(t.play_count),0) DESC,
           COUNT(t.id) DESC,
           COUNT(DISTINCT al.id) DESC,
           COALESCE(ar.lastfm_playcount,0) DESC,
           ar.name ASC
         LIMIT ?",
    )
    .bind(limit)
    .fetch_all(p)
    .await?;

    Ok(rows
        .into_iter()
        .map(|(id, name)| FavoriteArtist { id, name })
        .collect())
}

async fn refresh_artist(
    p: &sqlx::SqlitePool,
    client: &LastfmClient,
    artist: &FavoriteArtist,
    counters: &mut RefreshCounters,
) -> anyhow::Result<()> {
    let info = client.artist_get_info(p, &artist.name).await?;
    if info.get("error").and_then(|code| code.as_i64()) == Some(6) {
        counters.skipped_count += 1;
        return Ok(());
    }

    let top_albums = client.artist_get_top_albums(p, &artist.name, 20).await?;
    for album in value_list(top_albums["topalbums"].get("album")) {
        if let Some(name) = album["name"]
            .as_str()
            .filter(|name| !name.trim().is_empty())
        {
            let normalized = normalize_name(name);
            if normalized.is_empty() {
                counters.skipped_count += 1;
                continue;
            }
            let match_status = album_match_status(p, artist.id, &normalized).await?;
            let already_in_library = match_status == "already_in_library";
            let candidate = DiscoveryCandidate {
                local_artist_id: Some(artist.id),
                local_artist_name: Some(artist.name.clone()),
                discovered_artist_name: artist.name.clone(),
                title: name.to_string(),
                release_type: "album".to_string(),
                release_date: None,
                external_url: album["url"].as_str().map(ToString::to_string),
                cover_url: lastfm_image_url(&album),
                already_in_library,
                confidence_score: confidence_for_match(&match_status, 0.82),
                match_status,
                reason: format!("Popular Last.fm album by {}", artist.name),
                source_artist_name: Some(artist.name.clone()),
                source_artist_id: Some(artist.id),
                raw_json: album,
            };
            upsert_discovery(p, candidate, counters).await?;
        }
    }

    let top_tracks = client.artist_get_top_tracks(p, &artist.name, 20).await?;
    for track in value_list(top_tracks["toptracks"].get("track")) {
        if let Some(name) = track["name"]
            .as_str()
            .filter(|name| !name.trim().is_empty())
        {
            let normalized = normalize_name(name);
            if normalized.is_empty() {
                counters.skipped_count += 1;
                continue;
            }
            let match_status = track_match_status(p, artist.id, &normalized).await?;
            let already_in_library = match_status == "already_in_library";
            let candidate = DiscoveryCandidate {
                local_artist_id: Some(artist.id),
                local_artist_name: Some(artist.name.clone()),
                discovered_artist_name: artist.name.clone(),
                title: name.to_string(),
                release_type: "track".to_string(),
                release_date: None,
                external_url: track["url"].as_str().map(ToString::to_string),
                cover_url: lastfm_image_url(&track),
                already_in_library,
                confidence_score: confidence_for_match(&match_status, 0.74),
                match_status,
                reason: format!("Popular Last.fm track by {}", artist.name),
                source_artist_name: Some(artist.name.clone()),
                source_artist_id: Some(artist.id),
                raw_json: track,
            };
            upsert_discovery(p, candidate, counters).await?;
        }
    }

    let similar = client.artist_get_similar(p, &artist.name, 20).await?;
    for similar_artist in value_list(similar["similarartists"].get("artist")) {
        if let Some(name) = similar_artist["name"]
            .as_str()
            .filter(|name| !name.trim().is_empty())
        {
            let normalized = normalize_name(name);
            if normalized.is_empty() {
                counters.skipped_count += 1;
                continue;
            }
            let match_status = artist_match_status(p, &normalized).await?;
            let already_in_library = match_status == "already_in_library";
            let candidate = DiscoveryCandidate {
                local_artist_id: None,
                local_artist_name: None,
                discovered_artist_name: name.to_string(),
                title: name.to_string(),
                release_type: "artist".to_string(),
                release_date: None,
                external_url: similar_artist["url"].as_str().map(ToString::to_string),
                cover_url: lastfm_image_url(&similar_artist),
                already_in_library,
                confidence_score: confidence_for_match(&match_status, 0.78),
                match_status,
                reason: format!("Similar to {}", artist.name),
                source_artist_name: Some(artist.name.clone()),
                source_artist_id: Some(artist.id),
                raw_json: similar_artist,
            };
            upsert_discovery(p, candidate, counters).await?;
        }
    }

    Ok(())
}

async fn album_match_status(
    p: &sqlx::SqlitePool,
    artist_id: i64,
    normalized_title: &str,
) -> anyhow::Result<String> {
    let rows = sqlx::query_as::<_, (String,)>(
        "SELECT title FROM albums WHERE artist_id = ? OR album_artist_id = ?",
    )
    .bind(artist_id)
    .bind(artist_id)
    .fetch_all(p)
    .await?;
    Ok(match_name_status(
        rows.into_iter().map(|(name,)| name),
        normalized_title,
    ))
}

async fn track_match_status(
    p: &sqlx::SqlitePool,
    artist_id: i64,
    normalized_title: &str,
) -> anyhow::Result<String> {
    let rows = sqlx::query_as::<_, (String,)>("SELECT title FROM tracks WHERE artist_id = ?")
        .bind(artist_id)
        .fetch_all(p)
        .await?;
    Ok(match_name_status(
        rows.into_iter().map(|(name,)| name),
        normalized_title,
    ))
}

async fn artist_match_status(
    p: &sqlx::SqlitePool,
    normalized_name: &str,
) -> anyhow::Result<String> {
    let rows = sqlx::query_as::<_, (String,)>("SELECT name FROM artists")
        .fetch_all(p)
        .await?;
    Ok(match_name_status(
        rows.into_iter().map(|(name,)| name),
        normalized_name,
    ))
}

fn match_name_status(names: impl Iterator<Item = String>, normalized_candidate: &str) -> String {
    for name in names {
        let normalized = normalize_name(&name);
        if normalized == normalized_candidate {
            return "already_in_library".to_string();
        }
        if !normalized.is_empty()
            && (normalized.contains(normalized_candidate)
                || normalized_candidate.contains(&normalized))
        {
            return "possibly_in_library".to_string();
        }
    }
    "missing".to_string()
}

fn confidence_for_match(match_status: &str, base: f64) -> f64 {
    match match_status {
        "already_in_library" => 1.0,
        "possibly_in_library" => (base - 0.16).max(0.0),
        "uncertain" => (base - 0.28).max(0.0),
        _ => base,
    }
}

async fn upsert_discovery(
    p: &sqlx::SqlitePool,
    candidate: DiscoveryCandidate,
    counters: &mut RefreshCounters,
) -> anyhow::Result<()> {
    let normalized_artist = normalize_name(&candidate.discovered_artist_name);
    let normalized_title = normalize_name(&candidate.title);
    let exists = sqlx::query_as::<_, (i64,)>(
        "SELECT id FROM discovered_releases
         WHERE source='lastfm' AND release_type=? AND normalized_discovered_artist_name=? AND normalized_title=?
         LIMIT 1",
    )
    .bind(&candidate.release_type)
    .bind(&normalized_artist)
    .bind(&normalized_title)
    .fetch_optional(p)
    .await?
    .is_some();

    sqlx::query(
        "INSERT INTO discovered_releases(
            artist_id, local_artist_id, local_artist_name, discovered_artist_name, title,
            release_type, release_date, source, external_url, cover_url, already_in_library,
            match_status, confidence_score, reason, source_artist_name, source_artist_id,
            raw_json, normalized_discovered_artist_name, normalized_title, created_at, updated_at
         )
         VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,datetime('now'),datetime('now'))
         ON CONFLICT(source, release_type, normalized_discovered_artist_name, normalized_title)
         DO UPDATE SET
            artist_id=excluded.artist_id,
            local_artist_id=excluded.local_artist_id,
            local_artist_name=excluded.local_artist_name,
            discovered_artist_name=excluded.discovered_artist_name,
            title=excluded.title,
            release_date=excluded.release_date,
            external_url=excluded.external_url,
            cover_url=excluded.cover_url,
            already_in_library=excluded.already_in_library,
            match_status=excluded.match_status,
            confidence_score=excluded.confidence_score,
            reason=excluded.reason,
            source_artist_name=excluded.source_artist_name,
            source_artist_id=excluded.source_artist_id,
            raw_json=excluded.raw_json,
            updated_at=datetime('now')",
    )
    .bind(candidate.local_artist_id)
    .bind(candidate.local_artist_id)
    .bind(candidate.local_artist_name)
    .bind(candidate.discovered_artist_name)
    .bind(candidate.title)
    .bind(candidate.release_type)
    .bind(candidate.release_date)
    .bind("lastfm")
    .bind(candidate.external_url)
    .bind(candidate.cover_url)
    .bind(if candidate.already_in_library { 1 } else { 0 })
    .bind(candidate.match_status)
    .bind(candidate.confidence_score)
    .bind(candidate.reason)
    .bind(candidate.source_artist_name)
    .bind(candidate.source_artist_id)
    .bind(serde_json::to_string(&candidate.raw_json)?)
    .bind(normalized_artist)
    .bind(normalized_title)
    .execute(p)
    .await?;

    if exists {
        counters.updated_count += 1;
    } else {
        counters.created_count += 1;
    }
    Ok(())
}

async fn discovery_rows(
    p: &sqlx::SqlitePool,
    base_filter: &str,
    ownership_filter: &str,
    options: DiscoveryListOptions,
    order_by: &str,
) -> anyhow::Result<serde_json::Value> {
    let limit = options.limit.clamp(1, 500);
    let offset = options.offset.max(0);
    let count_sql =
        format!("SELECT COUNT(*) FROM discovered_releases WHERE {base_filter} {ownership_filter}");
    let (total,): (i64,) = sqlx::query_as(&count_sql).fetch_one(p).await?;
    let rows_sql = format!(
        "SELECT id, local_artist_id, local_artist_name, discovered_artist_name, title,
                release_type, release_date, source, external_url, cover_url,
                already_in_library, match_status, confidence_score, reason,
                source_artist_name, source_artist_id,
                CASE WHEN release_date IS NULL OR release_date = '' THEN 'unknown' ELSE 'known' END AS release_date_status
         FROM discovered_releases
         WHERE {base_filter} {ownership_filter}
         ORDER BY {order_by}
         LIMIT ? OFFSET ?"
    );
    let rows = sqlx::query(&rows_sql)
        .bind(limit)
        .bind(offset)
        .fetch_all(p)
        .await?;
    let items = rows
        .into_iter()
        .map(|row| {
            json!({
                "id": row.get::<i64, _>("id"),
                "local_artist_id": row.get::<Option<i64>, _>("local_artist_id"),
                "local_artist_name": row.get::<Option<String>, _>("local_artist_name"),
                "discovered_artist_name": row.get::<Option<String>, _>("discovered_artist_name"),
                "title": row.get::<Option<String>, _>("title"),
                "release_type": row.get::<Option<String>, _>("release_type"),
                "release_date": row.get::<Option<String>, _>("release_date"),
                "release_date_status": row.get::<String, _>("release_date_status"),
                "source": row.get::<Option<String>, _>("source"),
                "external_url": row.get::<Option<String>, _>("external_url"),
                "cover_url": row.get::<Option<String>, _>("cover_url"),
                "already_in_library": row.get::<i64, _>("already_in_library") != 0,
                "match_status": row.get::<Option<String>, _>("match_status"),
                "confidence_score": row.get::<Option<f64>, _>("confidence_score"),
                "reason": row.get::<Option<String>, _>("reason"),
                "source_artist_name": row.get::<Option<String>, _>("source_artist_name"),
                "source_artist_id": row.get::<Option<i64>, _>("source_artist_id")
            })
        })
        .collect::<Vec<_>>();

    Ok(json!({
        "items": items,
        "total": total,
        "limit": limit,
        "offset": offset,
        "generated_at": Utc::now().to_rfc3339()
    }))
}

fn value_list(value: Option<&Value>) -> Vec<Value> {
    match value {
        Some(Value::Array(values)) => values.clone(),
        Some(Value::Object(_)) => value.cloned().into_iter().collect(),
        _ => Vec::new(),
    }
}

fn lastfm_image_url(value: &Value) -> Option<String> {
    value
        .get("image")
        .and_then(|image| image.as_array())
        .and_then(|images| {
            images.iter().rev().find_map(|image| {
                image
                    .get("#text")
                    .and_then(|url| url.as_str())
                    .filter(|url| !url.trim().is_empty())
                    .map(ToString::to_string)
            })
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn test_pool() -> sqlx::SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        pool
    }

    #[test]
    fn match_status_finds_exact_and_possible_matches() {
        assert_eq!(
            match_name_status(["The Album".to_string()].into_iter(), "the album"),
            "already_in_library"
        );
        assert_eq!(
            match_name_status(
                ["The Album Deluxe Edition".to_string()].into_iter(),
                "the album"
            ),
            "already_in_library"
        );
        assert_eq!(
            match_name_status(["Long Song Title".to_string()].into_iter(), "long song"),
            "possibly_in_library"
        );
    }

    #[tokio::test]
    async fn discovery_upsert_avoids_duplicates() {
        let pool = test_pool().await;
        let mut counters = RefreshCounters::default();
        let candidate = DiscoveryCandidate {
            local_artist_id: Some(1),
            local_artist_name: Some("Artist".to_string()),
            discovered_artist_name: "Artist".to_string(),
            title: "Missing Album".to_string(),
            release_type: "album".to_string(),
            release_date: None,
            external_url: None,
            cover_url: None,
            already_in_library: false,
            match_status: "missing".to_string(),
            confidence_score: 0.8,
            reason: "Popular Last.fm album by Artist".to_string(),
            source_artist_name: Some("Artist".to_string()),
            source_artist_id: Some(1),
            raw_json: json!({"name":"Missing Album"}),
        };

        upsert_discovery(&pool, candidate, &mut counters)
            .await
            .unwrap();
        let candidate = DiscoveryCandidate {
            local_artist_id: Some(1),
            local_artist_name: Some("Artist".to_string()),
            discovered_artist_name: "artist".to_string(),
            title: "Missing Album".to_string(),
            release_type: "album".to_string(),
            release_date: None,
            external_url: None,
            cover_url: None,
            already_in_library: false,
            match_status: "missing".to_string(),
            confidence_score: 0.9,
            reason: "Popular Last.fm album by Artist".to_string(),
            source_artist_name: Some("Artist".to_string()),
            source_artist_id: Some(1),
            raw_json: json!({"name":"Missing Album"}),
        };
        upsert_discovery(&pool, candidate, &mut counters)
            .await
            .unwrap();

        let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM discovered_releases")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 1);
        assert_eq!(counters.created_count, 1);
        assert_eq!(counters.updated_count, 1);
    }

    #[tokio::test]
    async fn discovery_read_filters_owned_items_by_default() {
        let pool = test_pool().await;
        sqlx::query(
            "INSERT INTO discovered_releases(source,release_type,discovered_artist_name,title,already_in_library,match_status,confidence_score,normalized_discovered_artist_name,normalized_title)
             VALUES('lastfm','artist','Owned','Owned',1,'already_in_library',1.0,'owned','owned'),
                   ('lastfm','artist','Missing','Missing',0,'missing',0.8,'missing','missing')",
        )
        .execute(&pool)
        .await
        .unwrap();

        let value = DiscoveryService
            .similar_artists(&pool, DiscoveryListOptions::default())
            .await
            .unwrap();

        assert_eq!(value["total"], 1);
        assert_eq!(value["items"][0]["discovered_artist_name"], "Missing");
    }
}
