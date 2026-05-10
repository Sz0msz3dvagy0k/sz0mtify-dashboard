use anyhow::anyhow;
use chrono::{Duration, Utc};
use reqwest::Client;
use serde_json::Value;
use tracing::debug;

use crate::services::sync::setting_or_env;
use crate::utils::matching::normalize_name;

#[derive(Clone)]
pub struct LastfmClient {
    client: Client,
}

impl LastfmClient {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    pub async fn artist_get_info(
        &self,
        pool: &sqlx::SqlitePool,
        artist: &str,
    ) -> anyhow::Result<Value> {
        self.call_cached(
            pool,
            "artist.getinfo",
            vec![("artist".to_string(), artist.to_string())],
            7,
        )
        .await
    }

    pub async fn artist_get_top_albums(
        &self,
        pool: &sqlx::SqlitePool,
        artist: &str,
        limit: i64,
    ) -> anyhow::Result<Value> {
        self.call_cached(
            pool,
            "artist.gettopalbums",
            vec![
                ("artist".to_string(), artist.to_string()),
                ("limit".to_string(), limit.to_string()),
            ],
            7,
        )
        .await
    }

    pub async fn artist_get_top_tracks(
        &self,
        pool: &sqlx::SqlitePool,
        artist: &str,
        limit: i64,
    ) -> anyhow::Result<Value> {
        self.call_cached(
            pool,
            "artist.gettoptracks",
            vec![
                ("artist".to_string(), artist.to_string()),
                ("limit".to_string(), limit.to_string()),
            ],
            7,
        )
        .await
    }

    pub async fn artist_get_similar(
        &self,
        pool: &sqlx::SqlitePool,
        artist: &str,
        limit: i64,
    ) -> anyhow::Result<Value> {
        self.call_cached(
            pool,
            "artist.getsimilar",
            vec![
                ("artist".to_string(), artist.to_string()),
                ("limit".to_string(), limit.to_string()),
            ],
            14,
        )
        .await
    }

    async fn call_cached(
        &self,
        pool: &sqlx::SqlitePool,
        method: &str,
        params: Vec<(String, String)>,
        ttl_days: i64,
    ) -> anyhow::Result<Value> {
        let cache_key = cache_key(method, &params);
        if let Some((response_json,)) = sqlx::query_as::<_, (String,)>(
            "SELECT response_json
             FROM api_cache
             WHERE provider='lastfm' AND cache_key=? AND expires_at > datetime('now')
             LIMIT 1",
        )
        .bind(&cache_key)
        .fetch_optional(pool)
        .await?
        {
            debug!(cache_key, "Last.fm cache hit");
            return Ok(serde_json::from_str(&response_json)?);
        }

        debug!(cache_key, "Last.fm cache miss");
        let api_key = setting_or_env(pool, "lastfm_api_key", "LASTFM_API_KEY")
            .await?
            .ok_or_else(|| {
                anyhow!("missing Last.fm API key (settings.lastfm_api_key or LASTFM_API_KEY)")
            })?;
        let mut query = vec![
            ("method".to_string(), method.to_string()),
            ("api_key".to_string(), api_key),
            ("format".to_string(), "json".to_string()),
        ];
        query.extend(params.clone());
        let response = self
            .client
            .get("https://ws.audioscrobbler.com/2.0/")
            .query(&query)
            .send()
            .await?
            .error_for_status()?
            .json::<Value>()
            .await?;

        if let Some(code) = response.get("error").and_then(|value| value.as_i64()) {
            if code != 6 {
                let message = response["message"]
                    .as_str()
                    .unwrap_or("Last.fm returned an error without a message");
                return Err(anyhow!("Last.fm API error {code}: {message}"));
            }
        }

        let response_json = serde_json::to_string(&response)?;
        let expires_at = (Utc::now() + Duration::days(ttl_days))
            .format("%Y-%m-%d %H:%M:%S")
            .to_string();
        sqlx::query("INSERT INTO api_cache(provider,cache_key,response_json,expires_at) VALUES('lastfm',?,?,?) ON CONFLICT(cache_key) DO UPDATE SET response_json=excluded.response_json, expires_at=excluded.expires_at, created_at=datetime('now')")
            .bind(&cache_key)
            .bind(response_json)
            .bind(expires_at)
            .execute(pool)
            .await?;

        Ok(response)
    }
}

pub fn cache_key(method: &str, params: &[(String, String)]) -> String {
    let mut normalized_params = params
        .iter()
        .map(|(key, value)| (key.to_lowercase(), normalize_name(value)))
        .collect::<Vec<_>>();
    normalized_params.sort_by(|left, right| left.0.cmp(&right.0).then(left.1.cmp(&right.1)));
    let params = normalized_params
        .into_iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join("&");
    format!("lastfm:{}?{}", method.to_lowercase(), params)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lastfm_cache_key_is_stable() {
        let left = cache_key(
            "artist.getTopAlbums",
            &[
                ("limit".to_string(), "10".to_string()),
                ("artist".to_string(), "Ariana Grande".to_string()),
            ],
        );
        let right = cache_key(
            "ARTIST.GETTOPALBUMS",
            &[
                ("artist".to_string(), "ariana grande".to_string()),
                ("limit".to_string(), "10".to_string()),
            ],
        );
        assert_eq!(left, right);
    }
}
