use serde_json::json;

#[derive(Clone)]
pub struct DiscoveryService;

impl DiscoveryService {
    pub async fn new_releases(&self, p: &sqlx::SqlitePool) -> anyhow::Result<serde_json::Value> {
        let rows = sqlx::query_as::<_, (i64, Option<i64>, Option<String>, Option<String>, Option<String>, Option<String>, Option<f64>)>(
            "SELECT id, artist_id, title, release_type, release_date, external_url, confidence_score FROM discovered_releases ORDER BY release_date DESC, id DESC LIMIT 100",
        )
        .fetch_all(p)
        .await?;
        Ok(json!(rows))
    }

    pub async fn missing_albums(&self, p: &sqlx::SqlitePool) -> anyhow::Result<serde_json::Value> {
        let rows = sqlx::query_as::<_, (i64, Option<String>, Option<String>, Option<String>)>(
            "SELECT id, title, release_date, external_url FROM discovered_releases WHERE already_in_library=0 ORDER BY confidence_score DESC, release_date DESC LIMIT 100",
        )
        .fetch_all(p)
        .await?;
        Ok(json!(rows))
    }

    pub async fn similar_artists(&self, p: &sqlx::SqlitePool) -> anyhow::Result<serde_json::Value> {
        let rows = sqlx::query_as::<_, (i64, String, Option<String>)>(
            "SELECT id, name, similar_artists_json FROM artists WHERE similar_artists_json IS NOT NULL AND similar_artists_json != '' LIMIT 100",
        )
        .fetch_all(p)
        .await?;
        Ok(json!(rows))
    }

    pub async fn refresh(&self, p: &sqlx::SqlitePool) -> anyhow::Result<()> {
        sqlx::query("UPDATE discovered_releases SET updated_at = CURRENT_TIMESTAMP")
            .execute(p)
            .await?;
        Ok(())
    }
}
