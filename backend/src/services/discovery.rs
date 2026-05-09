use serde_json::json;
#[derive(Clone)]
pub struct DiscoveryService;
impl DiscoveryService {
    pub async fn new_releases(&self, p: &sqlx::SqlitePool) -> anyhow::Result<serde_json::Value> {
        Ok(json!(sqlx::query_as::<_,(i64,String,String)>("SELECT id,title,COALESCE(release_date,'') FROM discovered_releases ORDER BY release_date DESC LIMIT 100").fetch_all(p).await?))
    }
    pub async fn missing_albums(&self, p: &sqlx::SqlitePool) -> anyhow::Result<serde_json::Value> {
        Ok(json!(
            sqlx::query_as::<_, (i64, String)>(
                "SELECT id,title FROM discovered_releases WHERE already_in_library=0 LIMIT 100"
            )
            .fetch_all(p)
            .await?
        ))
    }
    pub async fn similar_artists(&self, _: &sqlx::SqlitePool) -> anyhow::Result<serde_json::Value> {
        Ok(json!([]))
    }
    pub async fn refresh(&self, _: &sqlx::SqlitePool) -> anyhow::Result<()> {
        Ok(())
    }
}
