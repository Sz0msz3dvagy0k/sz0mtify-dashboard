use serde_json::Value;
#[derive(Clone)]
pub struct SettingsService;

const ALLOWED_SETTING_KEYS: &[&str] = &[
    "subsonic_base_url",
    "subsonic_url",
    "subsonic_username",
    "subsonic_password",
    "subsonic_api_version",
    "lastfm_api_key",
    "stream_transcode_mode",
    "stream_transcode_quality",
];

const REDACTED_VALUE: &str = "********";

impl SettingsService {
    pub async fn get_all(&self, pool: &sqlx::SqlitePool) -> anyhow::Result<Vec<(String, String)>> {
        let settings = sqlx::query_as::<_, (String, String)>("SELECT key,value FROM settings")
            .fetch_all(pool)
            .await?;

        Ok(settings
            .into_iter()
            .map(|(key, value)| {
                if is_secret_key(&key) {
                    (key, REDACTED_VALUE.to_string())
                } else {
                    (key, normalize_setting_value(value))
                }
            })
            .collect())
    }

    pub async fn save(&self, pool: &sqlx::SqlitePool, payload: Value) -> anyhow::Result<()> {
        if let Some(map) = payload.as_object() {
            for (k, v) in map {
                if !is_allowed_setting_key(k) {
                    anyhow::bail!("unsupported setting key: {k}");
                }
                if is_secret_key(k) && v.as_str() == Some(REDACTED_VALUE) {
                    continue;
                }
                sqlx::query("INSERT INTO settings(key,value) VALUES(?,?) ON CONFLICT(key) DO UPDATE SET value=excluded.value").bind(k).bind(v.to_string()).execute(pool).await?;
            }
        }
        Ok(())
    }

    pub async fn get_value(
        &self,
        pool: &sqlx::SqlitePool,
        key: &str,
    ) -> anyhow::Result<Option<String>> {
        setting_value(pool, key).await
    }
}

fn is_allowed_setting_key(key: &str) -> bool {
    ALLOWED_SETTING_KEYS.contains(&key)
}

fn is_secret_key(key: &str) -> bool {
    let lower = key.to_ascii_lowercase();
    lower.contains("password")
        || lower.contains("token")
        || lower.contains("secret")
        || lower.contains("api_key")
}

async fn setting_value(pool: &sqlx::SqlitePool, key: &str) -> anyhow::Result<Option<String>> {
    let value: Option<(String,)> = sqlx::query_as("SELECT value FROM settings WHERE key = ?")
        .bind(key)
        .fetch_optional(pool)
        .await?;

    if let Some((value,)) = value {
        let parsed = normalize_setting_value(value);
        let parsed = parsed.trim().to_string();
        if !parsed.is_empty() {
            return Ok(Some(parsed));
        }
    }

    Ok(None)
}

fn normalize_setting_value(value: String) -> String {
    serde_json::from_str::<String>(&value).unwrap_or(value)
}
