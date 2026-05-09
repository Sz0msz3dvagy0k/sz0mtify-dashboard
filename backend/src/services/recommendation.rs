use crate::utils::matching::playlist_score;
use serde_json::json;
#[derive(Clone)]
pub struct RecommendationService;
impl RecommendationService {
    pub async fn rediscovery(&self, _: &sqlx::SqlitePool) -> anyhow::Result<serde_json::Value> {
        Ok(json!({"score_example":playlist_score(0.8,40.0,3.0,80.0,1.0,2.0,1.0,123)}))
    }
    pub async fn current_rotation(
        &self,
        _: &sqlx::SqlitePool,
    ) -> anyhow::Result<serde_json::Value> {
        Ok(json!([]))
    }
    pub async fn favorites(&self, _: &sqlx::SqlitePool) -> anyhow::Result<serde_json::Value> {
        Ok(json!([]))
    }
}
