use chrono::Utc;
use serde_json::json;

#[derive(Clone)]
pub struct AnalyticsService;

impl AnalyticsService {
    pub async fn overview(&self, p: &sqlx::SqlitePool) -> anyhow::Result<serde_json::Value> {
        let (track_count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM tracks")
            .fetch_one(p)
            .await?;
        let (album_count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM albums")
            .fetch_one(p)
            .await?;
        let (artist_count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM artists")
            .fetch_one(p)
            .await?;
        let (play_count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM plays")
            .fetch_one(p)
            .await?;

        Ok(json!({
            "total_tracks": track_count,
            "total_albums": album_count,
            "total_artists": artist_count,
            "total_plays": play_count
        }))
    }

    pub async fn tracks(&self, p: &sqlx::SqlitePool) -> anyhow::Result<serde_json::Value> {
        let rows = sqlx::query_as::<_, (i64, String, Option<i64>, Option<i64>, Option<i64>, Option<String>)>(
            "SELECT id, title, artist_id, album_id, duration_seconds, genre FROM tracks ORDER BY id DESC LIMIT 500",
        )
        .fetch_all(p)
        .await?;
        Ok(json!(rows))
    }

    pub async fn albums(&self, p: &sqlx::SqlitePool) -> anyhow::Result<serde_json::Value> {
        let rows = sqlx::query_as::<
            _,
            (
                i64,
                String,
                Option<i64>,
                Option<i64>,
                Option<String>,
                Option<String>,
                Option<String>,
                Option<String>,
            ),
        >(
            "SELECT al.id,
                    al.title,
                    al.artist_id,
                    al.year,
                    al.genre,
                    al.cover_art_id,
                    COALESCE(NULLIF(album_artist.name, ''), NULLIF(artist.name, '')) AS artist_name,
                    al.created_at
             FROM albums al
             LEFT JOIN artists artist ON artist.id = al.artist_id
             LEFT JOIN artists album_artist ON album_artist.id = al.album_artist_id
             ORDER BY datetime(al.created_at) DESC, al.id DESC
             LIMIT 300",
        )
        .fetch_all(p)
        .await?;
        Ok(json!(rows))
    }

    pub async fn artists(&self, p: &sqlx::SqlitePool) -> anyhow::Result<serde_json::Value> {
        let rows = sqlx::query_as::<
            _,
            (
                i64,
                String,
                Option<i64>,
                Option<i64>,
                Option<i64>,
                Option<String>,
                Option<String>,
            ),
        >(
            "SELECT ar.id,
                    ar.name,
                    ar.album_count,
                    ar.track_count,
                    ar.play_count,
                    CASE
                        WHEN ar.image_url IS NOT NULL AND ar.image_url != ''
                        THEN '/api/artist-image/' || ar.id
                        ELSE NULL
                    END AS image_url,
                    (
                        SELECT al.cover_art_id
                        FROM albums al
                        LEFT JOIN tracks t ON t.album_id = al.id
                        LEFT JOIN track_artists ta ON ta.track_id = t.id
                        WHERE (al.artist_id = ar.id
                           OR al.album_artist_id = ar.id
                           OR ta.artist_id = ar.id)
                          AND al.cover_art_id IS NOT NULL
                          AND al.cover_art_id != ''
                        ORDER BY al.year DESC, al.id DESC
                        LIMIT 1
                    ) AS representative_cover_art_id
             FROM artists ar
             WHERE COALESCE(ar.album_count,0) > 0
                OR COALESCE(ar.track_count,0) > 0
                OR COALESCE(ar.play_count,0) > 0
             ORDER BY ar.name ASC
             LIMIT 300",
        )
        .fetch_all(p)
        .await?;
        Ok(json!(rows))
    }

    pub async fn album_by_id(
        &self,
        p: &sqlx::SqlitePool,
        id: i64,
    ) -> anyhow::Result<serde_json::Value> {
        let album = sqlx::query_as::<_, (i64, String, Option<i64>, Option<i64>, Option<String>, Option<i64>, Option<String>)>(
            "SELECT id, title, artist_id, year, genre, song_count, cover_art_id FROM albums WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(p)
        .await?;

        let artist_name = sqlx::query_as::<_, (Option<String>,)>(
            "SELECT COALESCE(album_artist.name, artist.name)
             FROM albums al
             LEFT JOIN artists artist ON artist.id = al.artist_id
             LEFT JOIN artists album_artist ON album_artist.id = al.album_artist_id
             WHERE al.id = ?",
        )
        .bind(id)
        .fetch_optional(p)
        .await?
        .and_then(|row| row.0);

        let tracks = sqlx::query_as::<_, (i64, String, Option<i64>, Option<i64>, Option<i64>)>(
            "SELECT id, title, track_number, disc_number, duration_seconds FROM tracks WHERE album_id = ? ORDER BY disc_number, track_number, id",
        )
        .bind(id)
        .fetch_all(p)
        .await?;

        Ok(json!({"album": album, "artist_name": artist_name, "tracks": tracks}))
    }

    pub async fn artist_by_id(
        &self,
        p: &sqlx::SqlitePool,
        id: i64,
    ) -> anyhow::Result<serde_json::Value> {
        let artist = sqlx::query_as::<
            _,
            (
                i64,
                String,
                Option<i64>,
                Option<i64>,
                Option<i64>,
                Option<String>,
                Option<String>,
            ),
        >(
            "SELECT id,
                    name,
                    album_count,
                    track_count,
                    play_count,
                    bio_summary,
                    CASE
                        WHEN image_url IS NOT NULL AND image_url != ''
                        THEN '/api/artist-image/' || id
                        ELSE NULL
                    END AS image_url
             FROM artists
             WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(p)
        .await?;

        let albums = sqlx::query_as::<_, (i64, String, Option<i64>, Option<String>)>(
            "SELECT DISTINCT al.id, al.title, al.year, al.cover_art_id
             FROM albums al
             LEFT JOIN tracks t ON t.album_id = al.id
             LEFT JOIN track_artists ta ON ta.track_id = t.id
             WHERE al.artist_id = ?
                OR al.album_artist_id = ?
                OR ta.artist_id = ?
             ORDER BY al.year DESC, al.title ASC",
        )
        .bind(id)
        .bind(id)
        .bind(id)
        .fetch_all(p)
        .await?;

        Ok(json!({"artist": artist, "albums": albums}))
    }

    pub async fn genres(&self, p: &sqlx::SqlitePool) -> anyhow::Result<serde_json::Value> {
        let rows = sqlx::query_as::<_, (i64, String, Option<i64>, Option<i64>, Option<i64>)>(
            "SELECT id, name, track_count, album_count, artist_count FROM genres ORDER BY track_count DESC, name ASC",
        )
        .fetch_all(p)
        .await?;
        Ok(json!(rows))
    }

    pub async fn audio_quality(&self, p: &sqlx::SqlitePool) -> anyhow::Result<serde_json::Value> {
        let rows = sqlx::query_as::<_, (Option<i64>, Option<i64>, i64)>(
            "SELECT bit_rate, bit_depth, COUNT(*) as count FROM tracks GROUP BY bit_rate, bit_depth ORDER BY count DESC",
        )
        .fetch_all(p)
        .await?;
        Ok(json!(rows))
    }

    pub async fn storage(&self, p: &sqlx::SqlitePool) -> anyhow::Result<serde_json::Value> {
        let (track_bytes, avg_track_bytes, duration_seconds): (
            Option<i64>,
            Option<f64>,
            Option<i64>,
        ) = sqlx::query_as(
            "SELECT SUM(COALESCE(size_bytes,0)),
                        AVG(NULLIF(size_bytes, 0)),
                        SUM(COALESCE(duration_seconds,0))
                 FROM tracks",
        )
        .fetch_one(p)
        .await?;
        let total_storage_bytes = track_bytes.unwrap_or(0);
        let total_duration_minutes = duration_seconds.unwrap_or(0) as f64 / 60.0;
        let average_mb_per_minute = if total_duration_minutes > 0.0 {
            Some((total_storage_bytes as f64 / 1_048_576.0) / total_duration_minutes)
        } else {
            None
        };
        let size_by_format = sqlx::query_as::<_, (Option<String>, i64, i64)>(
            "SELECT suffix, SUM(COALESCE(size_bytes,0)) AS bytes, COUNT(*) AS tracks
             FROM tracks
             GROUP BY suffix
             ORDER BY bytes DESC, suffix ASC",
        )
        .fetch_all(p)
        .await?;
        let size_by_content_type = sqlx::query_as::<_, (Option<String>, i64, i64)>(
            "SELECT content_type, SUM(COALESCE(size_bytes,0)) AS bytes, COUNT(*) AS tracks
             FROM tracks
             GROUP BY content_type
             ORDER BY bytes DESC, content_type ASC",
        )
        .fetch_all(p)
        .await?;
        let size_by_artist = sqlx::query_as::<_, (Option<i64>, Option<String>, i64, i64)>(
            "SELECT t.artist_id, COALESCE(ar.name, 'Unknown Artist'), SUM(COALESCE(t.size_bytes,0)) AS bytes, COUNT(*) AS tracks
             FROM tracks t
             LEFT JOIN artists ar ON ar.id = t.artist_id
             GROUP BY t.artist_id, ar.name
             ORDER BY bytes DESC, ar.name ASC
             LIMIT 50",
        )
        .fetch_all(p)
        .await?;
        let size_by_album = sqlx::query_as::<_, (Option<i64>, Option<String>, Option<i64>, i64, i64)>(
            "SELECT t.album_id, COALESCE(al.title, 'Unknown Album'), al.artist_id, SUM(COALESCE(t.size_bytes,0)) AS bytes, COUNT(*) AS tracks
             FROM tracks t
             LEFT JOIN albums al ON al.id = t.album_id
             GROUP BY t.album_id, al.title, al.artist_id
             ORDER BY bytes DESC, al.title ASC
             LIMIT 50",
        )
        .fetch_all(p)
        .await?;
        let size_by_genre = sqlx::query_as::<_, (Option<String>, i64, i64)>(
            "SELECT genre, SUM(COALESCE(size_bytes,0)) AS bytes, COUNT(*) AS tracks
             FROM tracks
             GROUP BY genre
             ORDER BY bytes DESC, genre ASC",
        )
        .fetch_all(p)
        .await?;
        let largest_tracks = sqlx::query_as::<_, (i64, String, Option<i64>, Option<i64>, i64, Option<i64>, Option<String>, Option<String>)>(
            "SELECT id, title, artist_id, album_id, COALESCE(size_bytes,0), duration_seconds, suffix, content_type
             FROM tracks
             WHERE COALESCE(size_bytes,0) > 0
             ORDER BY COALESCE(size_bytes,0) DESC
             LIMIT 50",
        )
        .fetch_all(p)
        .await?;
        let largest_albums = sqlx::query_as::<_, (Option<i64>, Option<String>, Option<i64>, i64, i64)>(
            "SELECT t.album_id, COALESCE(al.title, 'Unknown Album'), al.artist_id, SUM(COALESCE(t.size_bytes,0)) AS bytes, COUNT(*) AS tracks
             FROM tracks t
             LEFT JOIN albums al ON al.id = t.album_id
             GROUP BY t.album_id, al.title, al.artist_id
             HAVING bytes > 0
             ORDER BY bytes DESC, al.title ASC
             LIMIT 50",
        )
        .fetch_all(p)
        .await?;
        let extension_breakdown = sqlx::query_as::<_, (Option<String>, i64, i64)>(
            "SELECT suffix, COUNT(*) AS tracks, SUM(COALESCE(size_bytes,0)) AS bytes
             FROM tracks
             GROUP BY suffix
             ORDER BY tracks DESC, suffix ASC",
        )
        .fetch_all(p)
        .await?;
        let suspicious_large_tracks = sqlx::query_as::<_, (i64, String, i64, Option<i64>, Option<String>)>(
            "SELECT id, title, COALESCE(size_bytes,0), duration_seconds, suffix
             FROM tracks
             WHERE COALESCE(size_bytes,0) >= 1073741824
                OR (COALESCE(duration_seconds,0) > 0 AND (COALESCE(size_bytes,0) / (duration_seconds / 60.0)) > 52428800)
             ORDER BY COALESCE(size_bytes,0) DESC
             LIMIT 50",
        )
        .fetch_all(p)
        .await?;

        Ok(json!({
            "total_storage_bytes": total_storage_bytes,
            "tracks_size_bytes": total_storage_bytes,
            "average_track_size_bytes": avg_track_bytes.unwrap_or(0.0),
            "average_mb_per_minute": average_mb_per_minute,
            "size_by_format": size_by_format,
            "size_by_content_type": size_by_content_type,
            "size_by_artist": size_by_artist,
            "size_by_album": size_by_album,
            "size_by_genre": size_by_genre,
            "largest_tracks": largest_tracks,
            "largest_albums": largest_albums,
            "extension_breakdown": extension_breakdown,
            "suspicious_large_tracks": suspicious_large_tracks,
            "generated_at": Utc::now().to_rfc3339()
        }))
    }

    pub async fn metadata_health(&self, p: &sqlx::SqlitePool) -> anyhow::Result<serde_json::Value> {
        let (total_tracks,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM tracks")
            .fetch_one(p)
            .await?;
        let (missing_mbid,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM tracks WHERE mbid IS NULL OR mbid = ''")
                .fetch_one(p)
                .await?;
        let (missing_genre,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM tracks WHERE genre IS NULL OR genre = ''")
                .fetch_one(p)
                .await?;
        Ok(
            json!({"total_tracks": total_tracks, "missing_mbid": missing_mbid, "missing_genre": missing_genre}),
        )
    }

    pub async fn listening(&self, p: &sqlx::SqlitePool) -> anyhow::Result<serde_json::Value> {
        let (play_events,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM plays")
            .fetch_one(p)
            .await?;
        let (imported_plays,): (Option<i64>,) =
            sqlx::query_as("SELECT SUM(COALESCE(play_count,0)) FROM tracks")
                .fetch_one(p)
                .await?;
        let imported_plays = imported_plays.unwrap_or(0);
        let has_play_events = play_events > 0;
        let has_imported_play_counts = imported_plays > 0;

        if has_play_events {
            let top_tracks = sqlx::query_as::<_, (i64, String, Option<i64>, i64)>(
                "SELECT t.id, t.title, t.album_id, COUNT(pl.id) AS plays
                 FROM plays pl
                 JOIN tracks t ON t.id = pl.track_id
                 GROUP BY t.id, t.title, t.album_id
                 HAVING plays > 0
                 ORDER BY plays DESC, t.title ASC
                 LIMIT 50",
            )
            .fetch_all(p)
            .await?;
            let top_artists = sqlx::query_as::<_, (Option<i64>, Option<String>, i64)>(
                "SELECT t.artist_id, COALESCE(ar.name, 'Unknown Artist'), COUNT(pl.id) AS plays
                 FROM plays pl
                 JOIN tracks t ON t.id = pl.track_id
                 LEFT JOIN artists ar ON ar.id = t.artist_id
                 GROUP BY t.artist_id, ar.name
                 HAVING plays > 0
                 ORDER BY plays DESC, ar.name ASC
                 LIMIT 50",
            )
            .fetch_all(p)
            .await?;
            let top_albums = sqlx::query_as::<_, (Option<i64>, Option<String>, i64)>(
                "SELECT t.album_id, COALESCE(al.title, 'Unknown Album'), COUNT(pl.id) AS plays
                 FROM plays pl
                 JOIN tracks t ON t.id = pl.track_id
                 LEFT JOIN albums al ON al.id = t.album_id
                 GROUP BY t.album_id, al.title
                 HAVING plays > 0
                 ORDER BY plays DESC, al.title ASC
                 LIMIT 50",
            )
            .fetch_all(p)
            .await?;
            let recently_played = sqlx::query_as::<
                _,
                (
                    i64,
                    Option<i64>,
                    Option<String>,
                    Option<String>,
                    Option<String>,
                ),
            >(
                "SELECT pl.id, pl.track_id, t.title, pl.played_at, pl.source
                     FROM plays pl
                     LEFT JOIN tracks t ON t.id = pl.track_id
                     WHERE pl.played_at IS NOT NULL
                     ORDER BY pl.played_at DESC, pl.id DESC
                     LIMIT 50",
            )
            .fetch_all(p)
            .await?;
            let timeline = sqlx::query_as::<_, (String, i64)>(
                "SELECT substr(played_at, 1, 10) AS day, COUNT(*) AS plays
                 FROM plays
                 WHERE played_at IS NOT NULL
                 GROUP BY day
                 ORDER BY day DESC
                 LIMIT 180",
            )
            .fetch_all(p)
            .await?;

            return Ok(json!({
                "data_source": "plays_table",
                "has_play_events": has_play_events,
                "has_imported_play_counts": has_imported_play_counts,
                "total_plays": play_events,
                "top_tracks": top_tracks,
                "top_artists": top_artists,
                "top_albums": top_albums,
                "recently_played": recently_played,
                "timeline": timeline,
                "generated_at": Utc::now().to_rfc3339()
            }));
        }

        if has_imported_play_counts {
            let top_tracks = sqlx::query_as::<_, (i64, String, Option<i64>, i64)>(
                "SELECT id, title, album_id, COALESCE(play_count,0) AS plays
                 FROM tracks
                 WHERE COALESCE(play_count,0) > 0
                 ORDER BY plays DESC, title ASC
                 LIMIT 50",
            )
            .fetch_all(p)
            .await?;
            let top_artists = sqlx::query_as::<_, (Option<i64>, Option<String>, i64)>(
                "SELECT t.artist_id, COALESCE(ar.name, 'Unknown Artist'), SUM(COALESCE(t.play_count,0)) AS plays
                 FROM tracks t
                 LEFT JOIN artists ar ON ar.id = t.artist_id
                 GROUP BY t.artist_id, ar.name
                 HAVING plays > 0
                 ORDER BY plays DESC, ar.name ASC
                 LIMIT 50",
            )
            .fetch_all(p)
            .await?;
            let top_albums = sqlx::query_as::<_, (Option<i64>, Option<String>, i64)>(
                "SELECT t.album_id, COALESCE(al.title, 'Unknown Album'), SUM(COALESCE(t.play_count,0)) AS plays
                 FROM tracks t
                 LEFT JOIN albums al ON al.id = t.album_id
                 GROUP BY t.album_id, al.title
                 HAVING plays > 0
                 ORDER BY plays DESC, al.title ASC
                 LIMIT 50",
            )
            .fetch_all(p)
            .await?;
            let recently_played = sqlx::query_as::<_, (i64, String, Option<String>, i64)>(
                "SELECT id, title, last_played_at, COALESCE(play_count,0) AS plays
                     FROM tracks
                     WHERE last_played_at IS NOT NULL AND last_played_at != ''
                     ORDER BY last_played_at DESC
                     LIMIT 50",
            )
            .fetch_all(p)
            .await?;

            return Ok(json!({
                "data_source": "subsonic_play_count",
                "has_play_events": has_play_events,
                "has_imported_play_counts": has_imported_play_counts,
                "total_plays": imported_plays,
                "top_tracks": top_tracks,
                "top_artists": top_artists,
                "top_albums": top_albums,
                "recently_played": recently_played,
                "timeline": [],
                "generated_at": Utc::now().to_rfc3339()
            }));
        }

        Ok(json!({
            "data_source": "none",
            "has_play_events": false,
            "has_imported_play_counts": false,
            "total_plays": 0,
            "top_tracks": [],
            "top_artists": [],
            "top_albums": [],
            "recently_played": [],
            "timeline": [],
            "generated_at": Utc::now().to_rfc3339()
        }))
    }

    pub async fn timeline(&self, p: &sqlx::SqlitePool) -> anyhow::Result<serde_json::Value> {
        let rows = sqlx::query_as::<_, (String, i64)>(
            "SELECT substr(played_at, 1, 10) as day, COUNT(*) as plays FROM plays WHERE played_at IS NOT NULL GROUP BY day ORDER BY day DESC LIMIT 180",
        )
        .fetch_all(p)
        .await?;
        Ok(json!(rows))
    }

    pub async fn search(&self, p: &sqlx::SqlitePool, q: &str) -> anyhow::Result<serde_json::Value> {
        let t = format!("%{}%", q);
        let tracks = sqlx::query_as::<
            _,
            (
                i64,
                String,
                Option<String>,
                Option<i64>,
                Option<String>,
                Option<String>,
                Option<i64>,
            ),
        >(
            "SELECT t.id,
                    t.title,
                    COALESCE(ar.name, album_artist.name),
                    t.album_id,
                    al.title,
                    al.cover_art_id,
                    t.duration_seconds
             FROM tracks t
             LEFT JOIN artists ar ON ar.id = t.artist_id
             LEFT JOIN albums al ON al.id = t.album_id
             LEFT JOIN artists album_artist ON album_artist.id = al.artist_id
             WHERE t.title LIKE ?
             ORDER BY COALESCE(t.play_count, 0) DESC, t.title ASC
             LIMIT 30",
        )
        .bind(&t)
        .fetch_all(p)
        .await?;
        let albums = sqlx::query_as::<_, (i64, String, Option<String>, Option<String>)>(
            "SELECT al.id, al.title, ar.name, al.cover_art_id
             FROM albums al
             LEFT JOIN artists ar ON ar.id = al.artist_id
             WHERE al.title LIKE ?
             ORDER BY COALESCE(al.play_count, 0) DESC, al.title ASC
             LIMIT 20",
        )
        .bind(&t)
        .fetch_all(p)
        .await?;
        let artists = sqlx::query_as::<_, (i64, String, Option<String>, Option<String>)>(
            "SELECT ar.id,
                    ar.name,
                    ar.image_url,
                    (
                        SELECT al.cover_art_id
                        FROM albums al
                        WHERE (al.artist_id = ar.id OR al.album_artist_id = ar.id)
                          AND al.cover_art_id IS NOT NULL
                          AND al.cover_art_id != ''
                        ORDER BY COALESCE(al.play_count, 0) DESC, al.id DESC
                        LIMIT 1
                    )
             FROM artists ar
             WHERE ar.name LIKE ?
               AND (
                    COALESCE(ar.album_count,0) > 0
                 OR COALESCE(ar.track_count,0) > 0
                 OR COALESCE(ar.play_count,0) > 0
               )
             ORDER BY COALESCE(ar.play_count, 0) DESC, ar.name ASC
             LIMIT 20",
        )
        .bind(&t)
        .fetch_all(p)
        .await?;
        Ok(json!({"tracks": tracks, "albums": albums, "artists": artists}))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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

    #[tokio::test]
    async fn storage_total_is_not_double_counted_and_album_size_is_track_sum() {
        let pool = test_pool().await;
        sqlx::query("INSERT INTO artists(id,name) VALUES(1,'Artist')")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO albums(id,title,artist_id,size_bytes) VALUES(1,'Album',1,999999)")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO tracks(id,title,artist_id,album_id,size_bytes,duration_seconds,suffix,content_type)
             VALUES(1,'A',1,1,100,60,'flac','audio/flac'),
                   (2,'B',1,1,200,60,'flac','audio/flac')",
        )
        .execute(&pool)
        .await
        .unwrap();

        let value = AnalyticsService.storage(&pool).await.unwrap();

        assert_eq!(value["total_storage_bytes"], 300);
        assert_eq!(value["tracks_size_bytes"], 300);
        assert_eq!(value["size_by_album"][0][3], 300);
    }

    #[tokio::test]
    async fn albums_include_resolved_artist_name_and_created_at() {
        let pool = test_pool().await;
        sqlx::query("INSERT INTO artists(id,name) VALUES(1,'Track Artist'),(2,'Album Artist')")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO albums(id,title,artist_id,album_artist_id,created_at)
             VALUES(1,'Album',1,2,'2026-05-19T12:34:56Z')",
        )
        .execute(&pool)
        .await
        .unwrap();

        let value = AnalyticsService.albums(&pool).await.unwrap();

        assert_eq!(value[0][1], "Album");
        assert_eq!(value[0][6], "Album Artist");
        assert_eq!(value[0][7], "2026-05-19T12:34:56Z");
    }

    #[tokio::test]
    async fn listening_excludes_zero_play_tracks_when_no_play_sources_exist() {
        let pool = test_pool().await;
        sqlx::query("INSERT INTO tracks(id,title,play_count) VALUES(1,'Silent',0)")
            .execute(&pool)
            .await
            .unwrap();

        let value = AnalyticsService.listening(&pool).await.unwrap();

        assert_eq!(value["data_source"], "none");
        assert_eq!(value["total_plays"], 0);
        assert_eq!(value["top_tracks"].as_array().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn listening_uses_subsonic_play_count_fallback() {
        let pool = test_pool().await;
        sqlx::query("INSERT INTO artists(id,name) VALUES(1,'Artist')")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO albums(id,title,artist_id) VALUES(1,'Album',1)")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO tracks(id,title,artist_id,album_id,play_count)
             VALUES(1,'Played',1,1,4),(2,'Silent',1,1,0)",
        )
        .execute(&pool)
        .await
        .unwrap();

        let value = AnalyticsService.listening(&pool).await.unwrap();

        assert_eq!(value["data_source"], "subsonic_play_count");
        assert_eq!(value["total_plays"], 4);
        assert_eq!(value["top_tracks"].as_array().unwrap().len(), 1);
        assert_eq!(value["top_tracks"][0][1], "Played");
    }

    #[tokio::test]
    async fn listening_prefers_plays_table_when_events_exist() {
        let pool = test_pool().await;
        sqlx::query("INSERT INTO artists(id,name) VALUES(1,'Artist')")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO tracks(id,title,artist_id,play_count) VALUES(1,'Imported',1,100),(2,'Evented',1,0)")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO plays(track_id,played_at,source) VALUES(2,'2026-05-10T12:00:00Z','test')",
        )
        .execute(&pool)
        .await
        .unwrap();

        let value = AnalyticsService.listening(&pool).await.unwrap();

        assert_eq!(value["data_source"], "plays_table");
        assert_eq!(value["total_plays"], 1);
        assert_eq!(value["top_tracks"][0][1], "Evented");
    }
}
