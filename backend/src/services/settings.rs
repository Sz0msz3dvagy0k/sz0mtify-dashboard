use serde_json::Value;
#[derive(Clone)]
pub struct SettingsService;
impl SettingsService {
    pub async fn get_all(&self, pool: &sqlx::SqlitePool) -> anyhow::Result<Vec<(String, String)>> {
        Ok(sqlx::query_as("SELECT key,value FROM settings")
            .fetch_all(pool)
            .await?)
    }
    pub async fn save(&self, pool: &sqlx::SqlitePool, payload: Value) -> anyhow::Result<()> {
        if let Some(map) = payload.as_object() {
            for (k, v) in map {
                sqlx::query("INSERT INTO settings(key,value) VALUES(?,?) ON CONFLICT(key) DO UPDATE SET value=excluded.value").bind(k).bind(v.to_string()).execute(pool).await?;
            }
        }
        Ok(())
    }
}
