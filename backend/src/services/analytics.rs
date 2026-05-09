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
        let rows = sqlx::query_as::<_, (i64, String, Option<i64>, Option<i64>, Option<String>)>(
            "SELECT id, title, artist_id, year, genre FROM albums ORDER BY id DESC LIMIT 300",
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

        let tracks = sqlx::query_as::<_, (i64, String, Option<i64>, Option<i64>)>(
            "SELECT id, title, track_number, disc_number FROM tracks WHERE album_id = ? ORDER BY disc_number, track_number, id",
        )
        .bind(id)
        .fetch_all(p)
        .await?;

        Ok(json!({"album": album, "tracks": tracks}))
    }

    pub async fn artists(&self, p: &sqlx::SqlitePool) -> anyhow::Result<serde_json::Value> {
        let rows = sqlx::query_as::<_, (i64, String, Option<i64>, Option<i64>, Option<i64>)>(
            "SELECT id, name, album_count, track_count, play_count FROM artists ORDER BY name ASC LIMIT 300",
        )
        .fetch_all(p)
        .await?;
        Ok(json!(rows))
    }

    pub async fn artist_by_id(
        &self,
        p: &sqlx::SqlitePool,
        id: i64,
    ) -> anyhow::Result<serde_json::Value> {
        let artist = sqlx::query_as::<_, (i64, String, Option<i64>, Option<i64>, Option<i64>, Option<String>)>(
            "SELECT id, name, album_count, track_count, play_count, bio_summary FROM artists WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(p)
        .await?;

        let albums = sqlx::query_as::<_, (i64, String, Option<i64>)>(
            "SELECT id, title, year FROM albums WHERE artist_id = ? ORDER BY year DESC, title ASC",
        )
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
        let (track_bytes,): (Option<i64>,) = sqlx::query_as("SELECT SUM(size_bytes) FROM tracks")
            .fetch_one(p)
            .await?;
        let (album_bytes,): (Option<i64>,) = sqlx::query_as("SELECT SUM(size_bytes) FROM albums")
            .fetch_one(p)
            .await?;
        Ok(json!({
            "tracks_size_bytes": track_bytes.unwrap_or(0),
            "albums_size_bytes": album_bytes.unwrap_or(0),
            "total_size_bytes": track_bytes.unwrap_or(0) + album_bytes.unwrap_or(0)
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
        let (total_plays,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM plays")
            .fetch_one(p)
            .await?;
        let top_tracks = sqlx::query_as::<_, (i64, String, i64)>(
            "SELECT t.id, t.title, COUNT(pl.id) as plays FROM tracks t LEFT JOIN plays pl ON pl.track_id=t.id GROUP BY t.id, t.title ORDER BY plays DESC LIMIT 20",
        )
        .fetch_all(p)
        .await?;
        Ok(json!({"total_plays": total_plays, "top_tracks": top_tracks}))
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
        let tracks = sqlx::query_as::<_, (i64, String)>(
            "SELECT id,title FROM tracks WHERE title LIKE ? LIMIT 30",
        )
        .bind(&t)
        .fetch_all(p)
        .await?;
        let albums = sqlx::query_as::<_, (i64, String)>(
            "SELECT id,title FROM albums WHERE title LIKE ? LIMIT 20",
        )
        .bind(&t)
        .fetch_all(p)
        .await?;
        let artists = sqlx::query_as::<_, (i64, String)>(
            "SELECT id,name FROM artists WHERE name LIKE ? LIMIT 20",
        )
        .bind(&t)
        .fetch_all(p)
        .await?;
        Ok(json!({"tracks": tracks, "albums": albums, "artists": artists}))
    }
}
