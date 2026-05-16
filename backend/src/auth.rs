use std::collections::HashMap;
use std::{
    env,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::{anyhow, Context};
use axum::{
    body::Body,
    extract::{Request, State},
    http::{header, HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::RngCore;
use ring::digest;
use serde::Serialize;
use serde_json::json;
use tokio::sync::Mutex;
use tracing::warn;

#[derive(Clone)]
pub struct AppAuth {
    username: String,
    password_hash: [u8; 32],
    pool: sqlx::SqlitePool,
    stream_tokens: Arc<Mutex<HashMap<String, Session>>>,
    session_ttl: Duration,
    stream_token_ttl: Duration,
}

#[derive(Clone)]
struct Session {
    username: String,
    expires_at: u64,
}

#[derive(Clone, Serialize)]
pub struct AuthSession {
    pub username: String,
    pub token: String,
    pub expires_at: u64,
}

#[derive(Clone, Serialize)]
pub struct StreamToken {
    pub token: String,
    pub expires_at: u64,
}

#[derive(Clone, Serialize)]
pub struct ActiveSession {
    pub session_id: String,
    pub username: String,
    pub created_at: u64,
    pub last_seen_at: u64,
    pub expires_at: u64,
    pub current: bool,
}

#[derive(Clone)]
pub struct AuthUser {
    pub username: String,
}

impl AppAuth {
    pub fn from_env(pool: sqlx::SqlitePool) -> anyhow::Result<Self> {
        let username = env_string("APP_USERNAME").unwrap_or_else(|| "admin".to_string());
        let password_hash = match env_string("APP_PASSWORD_SHA256") {
            Some(value) => parse_password_hash(&value)?,
            None => {
                let password = env_string("APP_PASSWORD")
                    .ok_or_else(|| anyhow!("APP_PASSWORD or APP_PASSWORD_SHA256 must be set"))?;
                hash_password(&password)
            }
        };
        let session_ttl = Duration::from_secs(
            env::var("APP_SESSION_TTL_HOURS")
                .ok()
                .and_then(|value| value.parse::<u64>().ok())
                .filter(|hours| *hours > 0)
                .unwrap_or(24 * 30)
                * 3600,
        );
        let stream_token_ttl = Duration::from_secs(
            env::var("APP_STREAM_TOKEN_TTL_SECONDS")
                .ok()
                .and_then(|value| value.parse::<u64>().ok())
                .filter(|seconds| *seconds > 0)
                .unwrap_or(300),
        );

        Ok(Self {
            username,
            password_hash,
            pool,
            stream_tokens: Arc::new(Mutex::new(HashMap::new())),
            session_ttl,
            stream_token_ttl,
        })
    }

    pub async fn login(
        &self,
        username: &str,
        password: &str,
    ) -> anyhow::Result<Option<AuthSession>> {
        if username != self.username || !password_matches(&self.password_hash, password) {
            warn!(username, "failed login attempt");
            return Ok(None);
        }

        let token = generate_token();
        let token_hash = token_hash(&token);
        let created_at = now_unix();
        let expires_at = created_at + self.session_ttl.as_secs();

        sqlx::query(
            "INSERT INTO app_sessions(token_hash, username, created_at, last_seen_at, expires_at)
             VALUES(?, ?, ?, ?, ?)",
        )
        .bind(&token_hash)
        .bind(&self.username)
        .bind(created_at as i64)
        .bind(created_at as i64)
        .bind(expires_at as i64)
        .execute(&self.pool)
        .await?;

        Ok(Some(AuthSession {
            username: self.username.clone(),
            token,
            expires_at,
        }))
    }

    pub async fn logout(&self, token: &str) -> anyhow::Result<()> {
        sqlx::query("DELETE FROM app_sessions WHERE token_hash = ?")
            .bind(token_hash(token))
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn authenticate(&self, token: &str) -> Option<AuthUser> {
        let now = now_unix();
        if let Err(error) = self.cleanup_expired_sessions(now).await {
            warn!(error = %error, "failed to clean up expired sessions");
        }

        let row = match sqlx::query_as::<_, (String, i64)>(
            "SELECT username, expires_at FROM app_sessions WHERE token_hash = ?",
        )
        .bind(token_hash(token))
        .fetch_optional(&self.pool)
        .await
        {
            Ok(row) => row,
            Err(error) => {
                warn!(error = %error, "failed to authenticate session");
                return None;
            }
        };

        let Some((username, expires_at)) = row else {
            return None;
        };

        if expires_at <= now as i64 {
            if let Err(error) = self.logout(token).await {
                warn!(error = %error, "failed to remove expired session");
            }
            return None;
        }

        if let Err(error) =
            sqlx::query("UPDATE app_sessions SET last_seen_at = ? WHERE token_hash = ?")
                .bind(now as i64)
                .bind(token_hash(token))
                .execute(&self.pool)
                .await
        {
            warn!(error = %error, "failed to update session last seen time");
        }

        Some(AuthUser { username })
    }

    pub async fn active_sessions(
        &self,
        current_token: Option<&str>,
    ) -> anyhow::Result<Vec<ActiveSession>> {
        let now = now_unix();
        self.cleanup_expired_sessions(now).await?;
        let current_hash = current_token.map(token_hash);
        let rows = sqlx::query_as::<_, (String, String, i64, i64, i64)>(
            "SELECT token_hash, username, created_at, last_seen_at, expires_at
             FROM app_sessions
             WHERE expires_at > ?
             ORDER BY last_seen_at DESC, created_at DESC",
        )
        .bind(now as i64)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(
                |(hash, username, created_at, last_seen_at, expires_at)| ActiveSession {
                    session_id: hash.chars().take(12).collect(),
                    current: current_hash.as_deref() == Some(hash.as_str()),
                    username,
                    created_at: created_at.max(0) as u64,
                    last_seen_at: last_seen_at.max(0) as u64,
                    expires_at: expires_at.max(0) as u64,
                },
            )
            .collect())
    }

    pub async fn issue_stream_token(&self, username: &str) -> StreamToken {
        let token = generate_token();
        let expires_at = now_unix() + self.stream_token_ttl.as_secs();
        self.stream_tokens.lock().await.insert(
            token.clone(),
            Session {
                username: username.to_string(),
                expires_at,
            },
        );

        StreamToken { token, expires_at }
    }

    pub async fn authenticate_stream_token(&self, token: &str) -> Option<AuthUser> {
        let now = now_unix();
        let mut tokens = self.stream_tokens.lock().await;
        tokens.retain(|_, session| session.expires_at > now);
        tokens.get(token).map(|session| AuthUser {
            username: session.username.clone(),
        })
    }

    async fn cleanup_expired_sessions(&self, now: u64) -> anyhow::Result<()> {
        sqlx::query("DELETE FROM app_sessions WHERE expires_at <= ?")
            .bind(now as i64)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

pub async fn require_app_session(
    State(auth): State<AppAuth>,
    mut request: Request<Body>,
    next: Next,
) -> Response {
    let path = request.uri().path();
    if path == "/api/health" || path == "/api/auth/login" {
        return next.run(request).await;
    }

    let user = if let Some(token) = bearer_token(request.headers()) {
        auth.authenticate(token).await
    } else if is_stream_path(path) {
        match stream_token_query(request.uri().query()) {
            Some(token) => auth.authenticate_stream_token(&token).await,
            None => None,
        }
    } else {
        None
    };

    let Some(user) = user else {
        return unauthorized_response();
    };

    request.extensions_mut().insert(user);
    next.run(request).await
}

pub fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn is_stream_path(path: &str) -> bool {
    path.starts_with("/api/tracks/") && path.ends_with("/stream")
}

fn stream_token_query(query: Option<&str>) -> Option<String> {
    query?
        .split('&')
        .filter_map(|pair| pair.split_once('='))
        .find_map(|(key, value)| {
            (key == "stream_token" && !value.is_empty()).then(|| value.to_string())
        })
}

fn unauthorized_response() -> Response {
    (
        StatusCode::UNAUTHORIZED,
        Json(json!({"ok": false, "error": "unauthorized"})),
    )
        .into_response()
}

fn hash_password(password: &str) -> [u8; 32] {
    let digest = digest::digest(&digest::SHA256, password.as_bytes());
    let mut hash = [0_u8; 32];
    hash.copy_from_slice(digest.as_ref());
    hash
}

fn password_matches(expected_hash: &[u8; 32], password: &str) -> bool {
    let submitted_hash = hash_password(password);
    expected_hash
        .iter()
        .zip(submitted_hash.iter())
        .fold(0_u8, |diff, (left, right)| diff | (left ^ right))
        == 0
}

fn parse_password_hash(value: &str) -> anyhow::Result<[u8; 32]> {
    let bytes = hex::decode(value).context("APP_PASSWORD_SHA256 must be hex encoded")?;
    if bytes.len() != 32 {
        return Err(anyhow!("APP_PASSWORD_SHA256 must be a SHA-256 hash"));
    }
    let mut hash = [0_u8; 32];
    hash.copy_from_slice(&bytes);
    Ok(hash)
}

fn generate_token() -> String {
    let mut bytes = [0_u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

fn token_hash(token: &str) -> String {
    let digest = digest::digest(&digest::SHA256, token.as_bytes());
    hex::encode(digest.as_ref())
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn env_string(key: &str) -> Option<String> {
    env::var(key)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn test_auth() -> AppAuth {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::query(
            "CREATE TABLE app_sessions (
                token_hash TEXT PRIMARY KEY,
                username TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                last_seen_at INTEGER NOT NULL,
                expires_at INTEGER NOT NULL
            )",
        )
        .execute(&pool)
        .await
        .unwrap();

        AppAuth {
            username: "admin".to_string(),
            password_hash: hash_password("secret"),
            pool,
            stream_tokens: Arc::new(Mutex::new(HashMap::new())),
            session_ttl: Duration::from_secs(60),
            stream_token_ttl: Duration::from_secs(60),
        }
    }

    #[tokio::test]
    async fn login_issues_and_revokes_session_token() {
        let auth = test_auth().await;

        assert!(auth.login("admin", "wrong").await.unwrap().is_none());

        let session = auth.login("admin", "secret").await.unwrap().unwrap();
        assert_eq!(
            auth.authenticate(&session.token).await.unwrap().username,
            "admin"
        );

        auth.logout(&session.token).await.unwrap();
        assert!(auth.authenticate(&session.token).await.is_none());
    }

    #[tokio::test]
    async fn active_sessions_marks_current_session() {
        let auth = test_auth().await;

        let session = auth.login("admin", "secret").await.unwrap().unwrap();
        let sessions = auth.active_sessions(Some(&session.token)).await.unwrap();

        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].username, "admin");
        assert!(sessions[0].current);
    }

    #[tokio::test]
    async fn stream_tokens_are_separate_from_session_tokens() {
        let auth = test_auth().await;

        let session = auth.login("admin", "secret").await.unwrap().unwrap();
        assert!(auth
            .authenticate_stream_token(&session.token)
            .await
            .is_none());

        let stream_token = auth.issue_stream_token("admin").await;
        assert_eq!(
            auth.authenticate_stream_token(&stream_token.token)
                .await
                .unwrap()
                .username,
            "admin"
        );
    }

    #[test]
    fn password_hash_can_be_configured_as_sha256_hex() {
        let hash =
            parse_password_hash("2bb80d537b1da3e38bd30361aa855686bde0eacd7162fef6a25fe97bf527a25b")
                .unwrap();

        assert!(password_matches(&hash, "secret"));
        assert!(!password_matches(&hash, "not-secret"));
    }
}
