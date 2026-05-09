use serde_json::json;

#[derive(Clone)]
pub struct SyncService;
impl SyncService {
    pub fn new() -> Self {
        Self
    }
    pub async fn sync_subsonic(&self, pool: &sqlx::SqlitePool) -> anyhow::Result<()> {
        sqlx::query("INSERT OR REPLACE INTO sync_state (id,source,last_sync_at,status) VALUES (1,'subsonic',datetime('now'),'ok')").execute(pool).await?;
        Ok(())
    }
    pub async fn sync_lastfm(&self, pool: &sqlx::SqlitePool) -> anyhow::Result<()> {
        sqlx::query("INSERT OR REPLACE INTO sync_state (id,source,last_sync_at,status) VALUES (2,'lastfm',datetime('now'),'ok')").execute(pool).await?;
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
}
