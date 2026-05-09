use serde_json::json;
#[derive(Clone)]
pub struct AnalyticsService;
impl AnalyticsService {
    pub async fn overview(&self, p: &sqlx::SqlitePool) -> anyhow::Result<serde_json::Value> {
        let (tracks,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM tracks")
            .fetch_one(p)
            .await?;
        Ok(json!({"total_tracks":tracks}))
    }
    pub async fn tracks(&self, p: &sqlx::SqlitePool) -> anyhow::Result<serde_json::Value> {
        Ok(json!(
            sqlx::query_as::<_, (i64, String)>(
                "SELECT id,title FROM tracks ORDER BY id DESC LIMIT 200"
            )
            .fetch_all(p)
            .await?
        ))
    }
    pub async fn albums(&self, p: &sqlx::SqlitePool) -> anyhow::Result<serde_json::Value> {
        Ok(json!(
            sqlx::query_as::<_, (i64, String)>("SELECT id,title FROM albums LIMIT 200")
                .fetch_all(p)
                .await?
        ))
    }
    pub async fn album_by_id(
        &self,
        p: &sqlx::SqlitePool,
        id: i64,
    ) -> anyhow::Result<serde_json::Value> {
        Ok(json!(
            sqlx::query_as::<_, (i64, String)>("SELECT id,title FROM albums WHERE id=?")
                .bind(id)
                .fetch_optional(p)
                .await?
        ))
    }
    pub async fn artists(&self, p: &sqlx::SqlitePool) -> anyhow::Result<serde_json::Value> {
        Ok(json!(
            sqlx::query_as::<_, (i64, String)>("SELECT id,name FROM artists LIMIT 200")
                .fetch_all(p)
                .await?
        ))
    }
    pub async fn artist_by_id(
        &self,
        p: &sqlx::SqlitePool,
        id: i64,
    ) -> anyhow::Result<serde_json::Value> {
        Ok(json!(
            sqlx::query_as::<_, (i64, String)>("SELECT id,name FROM artists WHERE id=?")
                .bind(id)
                .fetch_optional(p)
                .await?
        ))
    }
    pub async fn genres(&self, p: &sqlx::SqlitePool) -> anyhow::Result<serde_json::Value> {
        Ok(json!(
            sqlx::query_as::<_, (i64, String)>("SELECT id,name FROM genres")
                .fetch_all(p)
                .await?
        ))
    }
    pub async fn audio_quality(&self, _: &sqlx::SqlitePool) -> anyhow::Result<serde_json::Value> {
        Ok(json!({"message":"placeholder"}))
    }
    pub async fn storage(&self, _: &sqlx::SqlitePool) -> anyhow::Result<serde_json::Value> {
        Ok(json!({"message":"placeholder"}))
    }
    pub async fn metadata_health(&self, _: &sqlx::SqlitePool) -> anyhow::Result<serde_json::Value> {
        Ok(json!({"health_score":82}))
    }
    pub async fn listening(&self, _: &sqlx::SqlitePool) -> anyhow::Result<serde_json::Value> {
        Ok(json!({"total_plays":0}))
    }
    pub async fn timeline(&self, _: &sqlx::SqlitePool) -> anyhow::Result<serde_json::Value> {
        Ok(json!([]))
    }
    pub async fn search(&self, p: &sqlx::SqlitePool, q: &str) -> anyhow::Result<serde_json::Value> {
        let t = format!("%{}%", q);
        Ok(json!(
            sqlx::query_as::<_, (i64, String)>(
                "SELECT id,title FROM tracks WHERE title LIKE ? LIMIT 30"
            )
            .bind(t)
            .fetch_all(p)
            .await?
        ))
    }
}
