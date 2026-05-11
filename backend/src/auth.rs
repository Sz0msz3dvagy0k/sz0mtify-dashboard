use std::{
    env,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{anyhow, Context};
use axum::{
    body::Body,
    extract::{Request, State},
    http::{header::HeaderName, StatusCode},
    middleware::Next,
    response::Response,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use ring::signature;
use serde::Deserialize;
use serde_json::Value;
use tracing::{info, warn};

const ACCESS_JWT_HEADER: HeaderName = HeaderName::from_static("cf-access-jwt-assertion");

#[derive(Clone)]
pub struct CloudflareAccessVerifier {
    audience: String,
    issuer: String,
    keys: Vec<JwkKey>,
}

#[derive(Clone)]
struct JwkKey {
    kid: String,
    n: Vec<u8>,
    e: Vec<u8>,
}

#[derive(Deserialize)]
struct JwksResponse {
    keys: Vec<JwksKey>,
}

#[derive(Deserialize)]
struct JwksKey {
    kid: String,
    kty: String,
    n: String,
    e: String,
    alg: Option<String>,
}

#[derive(Deserialize)]
struct JwtHeader {
    alg: String,
    kid: String,
}

#[derive(Deserialize)]
struct AccessClaims {
    aud: Value,
    exp: u64,
    iss: String,
    nbf: Option<u64>,
}

impl CloudflareAccessVerifier {
    pub async fn from_env() -> anyhow::Result<Option<Self>> {
        let Some(audience) = env_string("CLOUDFLARE_ACCESS_AUD") else {
            return Ok(None);
        };
        let Some(certs_url) = env_string("CLOUDFLARE_ACCESS_CERTS_URL") else {
            return Err(anyhow!(
                "CLOUDFLARE_ACCESS_AUD is set but CLOUDFLARE_ACCESS_CERTS_URL is missing"
            ));
        };

        let issuer = env_string("CLOUDFLARE_ACCESS_ISSUER")
            .unwrap_or_else(|| issuer_from_certs_url(&certs_url));
        let keys = fetch_keys(&certs_url).await?;

        info!(
            key_count = keys.len(),
            issuer, "Cloudflare Access JWT verification enabled"
        );

        Ok(Some(Self {
            audience,
            issuer,
            keys,
        }))
    }

    fn verify(&self, token: &str) -> anyhow::Result<()> {
        let parts = token.split('.').collect::<Vec<_>>();
        if parts.len() != 3 {
            return Err(anyhow!("invalid JWT format"));
        }

        let header_bytes = decode_jwt_part(parts[0]).context("invalid JWT header encoding")?;
        let header = serde_json::from_slice::<JwtHeader>(&header_bytes)
            .context("invalid JWT header JSON")?;
        if header.alg != "RS256" {
            return Err(anyhow!("unsupported JWT algorithm"));
        }

        let key = self
            .keys
            .iter()
            .find(|key| key.kid == header.kid)
            .ok_or_else(|| anyhow!("unknown JWT key id"))?;

        let signature_bytes =
            decode_jwt_part(parts[2]).context("invalid JWT signature encoding")?;
        let signing_input = format!("{}.{}", parts[0], parts[1]);
        signature::RsaPublicKeyComponents {
            n: &key.n,
            e: &key.e,
        }
        .verify(
            &signature::RSA_PKCS1_2048_8192_SHA256,
            signing_input.as_bytes(),
            &signature_bytes,
        )
        .map_err(|_| anyhow!("invalid JWT signature"))?;

        let claims_bytes = decode_jwt_part(parts[1]).context("invalid JWT claims encoding")?;
        let claims = serde_json::from_slice::<AccessClaims>(&claims_bytes)
            .context("invalid JWT claims JSON")?;
        validate_claims(&claims, &self.issuer, &self.audience)
    }
}

pub async fn require_cloudflare_access(
    State(verifier): State<Arc<CloudflareAccessVerifier>>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    if request.uri().path() == "/api/health" {
        return Ok(next.run(request).await);
    }

    let token = request
        .headers()
        .get(ACCESS_JWT_HEADER)
        .and_then(|header| header.to_str().ok());

    match token {
        Some(token) => match verifier.verify(token) {
            Ok(()) => Ok(next.run(request).await),
            Err(error) => {
                warn!(error = %error, "Cloudflare Access JWT verification failed");
                Err(StatusCode::UNAUTHORIZED)
            }
        },
        None => {
            warn!("missing Cloudflare Access JWT assertion header");
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}

async fn fetch_keys(certs_url: &str) -> anyhow::Result<Vec<JwkKey>> {
    let jwks = reqwest::Client::new()
        .get(certs_url)
        .send()
        .await
        .context("failed to fetch Cloudflare Access certs")?
        .error_for_status()
        .context("Cloudflare Access certs endpoint returned an HTTP error")?
        .json::<JwksResponse>()
        .await
        .context("failed to parse Cloudflare Access certs")?;

    let keys = jwks
        .keys
        .into_iter()
        .filter(|key| key.kty == "RSA" && key.alg.as_deref().is_none_or(|alg| alg == "RS256"))
        .map(|key| {
            Ok(JwkKey {
                kid: key.kid,
                n: decode_jwt_part(&key.n).context("invalid JWK modulus")?,
                e: decode_jwt_part(&key.e).context("invalid JWK exponent")?,
            })
        })
        .collect::<anyhow::Result<Vec<_>>>()?;

    if keys.is_empty() {
        return Err(anyhow!("Cloudflare Access certs did not contain RSA keys"));
    }

    Ok(keys)
}

fn validate_claims(claims: &AccessClaims, issuer: &str, audience: &str) -> anyhow::Result<()> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system time is before UNIX epoch")?
        .as_secs();

    if normalize_issuer(&claims.iss) != normalize_issuer(issuer) {
        return Err(anyhow!("invalid issuer"));
    }
    if !audience_matches(&claims.aud, audience) {
        return Err(anyhow!("invalid audience"));
    }
    if claims.exp <= now {
        return Err(anyhow!("JWT has expired"));
    }
    if let Some(nbf) = claims.nbf {
        if nbf > now {
            return Err(anyhow!("JWT is not valid yet"));
        }
    }

    Ok(())
}

fn audience_matches(value: &Value, expected: &str) -> bool {
    match value {
        Value::String(audience) => audience == expected,
        Value::Array(audiences) => audiences
            .iter()
            .any(|audience| audience.as_str() == Some(expected)),
        _ => false,
    }
}

fn decode_jwt_part(value: &str) -> anyhow::Result<Vec<u8>> {
    Ok(URL_SAFE_NO_PAD.decode(value)?)
}

fn env_string(key: &str) -> Option<String> {
    env::var(key)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn issuer_from_certs_url(certs_url: &str) -> String {
    certs_url
        .split("/cdn-cgi/access/certs")
        .next()
        .unwrap_or(certs_url)
        .trim_end_matches('/')
        .to_string()
}

fn normalize_issuer(value: &str) -> &str {
    value.trim_end_matches('/')
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn audience_can_be_string_or_array() {
        assert!(audience_matches(&json!("aud-1"), "aud-1"));
        assert!(audience_matches(&json!(["aud-0", "aud-1"]), "aud-1"));
        assert!(!audience_matches(&json!(["aud-0"]), "aud-1"));
    }

    #[test]
    fn issuer_is_derived_from_certs_url() {
        assert_eq!(
            issuer_from_certs_url("https://team.cloudflareaccess.com/cdn-cgi/access/certs"),
            "https://team.cloudflareaccess.com"
        );
    }
}
