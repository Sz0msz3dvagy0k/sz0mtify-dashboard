use std::{
    collections::HashMap,
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
    sessions: Arc<Mutex<HashMap<String, Session>>>,
    session_ttl: Duration,
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

#[derive(Clone)]
pub struct AuthUser {
    pub username: String,
}

impl AppAuth {
    pub fn from_env() -> anyhow::Result<Self> {
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

        Ok(Self {
            username,
            password_hash,
            sessions: Arc::new(Mutex::new(HashMap::new())),
            session_ttl,
        })
    }

    pub async fn login(&self, username: &str, password: &str) -> Option<AuthSession> {
        if username != self.username || !password_matches(&self.password_hash, password) {
            warn!(username, "failed login attempt");
            return None;
        }

        let token = generate_token();
        let expires_at = now_unix() + self.session_ttl.as_secs();
        self.sessions.lock().await.insert(
            token.clone(),
            Session {
                username: self.username.clone(),
                expires_at,
            },
        );

        Some(AuthSession {
            username: self.username.clone(),
            token,
            expires_at,
        })
    }

    pub async fn logout(&self, token: &str) {
        self.sessions.lock().await.remove(token);
    }

    pub async fn authenticate(&self, token: &str) -> Option<AuthUser> {
        let now = now_unix();
        let mut sessions = self.sessions.lock().await;
        sessions.retain(|_, session| session.expires_at > now);
        sessions.get(token).map(|session| AuthUser {
            username: session.username.clone(),
        })
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

    let Some(token) = request_token(&request) else {
        return unauthorized_response();
    };

    let Some(user) = auth.authenticate(&token).await else {
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

fn request_token(request: &Request<Body>) -> Option<String> {
    bearer_token(request.headers())
        .map(str::to_string)
        .or_else(|| access_token_query(request.uri().query()))
}

fn access_token_query(query: Option<&str>) -> Option<String> {
    query?
        .split('&')
        .filter_map(|pair| pair.split_once('='))
        .find_map(|(key, value)| {
            (key == "access_token" && !value.is_empty()).then(|| value.to_string())
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

    #[tokio::test]
    async fn login_issues_and_revokes_session_token() {
        let auth = AppAuth {
            username: "admin".to_string(),
            password_hash: hash_password("secret"),
            sessions: Arc::new(Mutex::new(HashMap::new())),
            session_ttl: Duration::from_secs(60),
        };

        assert!(auth.login("admin", "wrong").await.is_none());

        let session = auth.login("admin", "secret").await.unwrap();
        assert_eq!(
            auth.authenticate(&session.token).await.unwrap().username,
            "admin"
        );

        auth.logout(&session.token).await;
        assert!(auth.authenticate(&session.token).await.is_none());
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
