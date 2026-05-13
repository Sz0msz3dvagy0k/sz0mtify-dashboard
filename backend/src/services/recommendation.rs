use crate::utils::matching::playlist_score;
use serde_json::json;

#[derive(Clone)]
pub struct RecommendationService;

impl RecommendationService {
    pub async fn rediscovery(&self, p: &sqlx::SqlitePool) -> anyhow::Result<serde_json::Value> {
        let rows = sqlx::query_as::<_, (i64, String, Option<String>, Option<i64>, i64, Option<String>)>(
            "SELECT id, title, last_played_at, album_id, play_count, genre FROM tracks WHERE COALESCE(play_count, 0) > 0 ORDER BY COALESCE(last_played_at,'1970-01-01') ASC, play_count DESC LIMIT 50",
        )
        .fetch_all(p)
        .await?;

        Ok(json!({
            "tracks": rows,
            "score_example": playlist_score(0.8, 40.0, 3.0, 80.0, 1.0, 2.0, 1.0, 123)
        }))
    }

    pub async fn current_rotation(
        &self,
        p: &sqlx::SqlitePool,
    ) -> anyhow::Result<serde_json::Value> {
        let rows = sqlx::query_as::<_, (i64, String, Option<String>, Option<i64>, Option<i64>)>(
            "SELECT id, title, genre, album_id, play_count FROM tracks ORDER BY COALESCE(last_played_at,'1970-01-01') DESC, COALESCE(play_count,0) DESC LIMIT 50",
        )
        .fetch_all(p)
        .await?;
        Ok(json!(rows))
    }

    #[allow(dead_code)]
    pub async fn favorites(&self, p: &sqlx::SqlitePool) -> anyhow::Result<serde_json::Value> {
        let rows = sqlx::query_as::<_, (i64, String, Option<i64>, Option<i64>)>(
            "SELECT id, title, rating, play_count FROM tracks WHERE COALESCE(starred,0)=1 OR COALESCE(rating,0)>=4 ORDER BY COALESCE(rating,0) DESC, COALESCE(play_count,0) DESC LIMIT 100",
        )
        .fetch_all(p)
        .await?;
        Ok(json!(rows))
    }
}
