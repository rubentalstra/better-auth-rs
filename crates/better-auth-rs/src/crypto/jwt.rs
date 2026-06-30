//! JWT signing/verification (port of `crypto/jwt.ts`).
//!
//! - [`sign_jwt`]/[`verify_jwt`] — HS256 JWS over the cookie-cache payload (pure Rust, via
//!   `jsonwebtoken`). Always available.
//! - `symmetric_encode_jwt`/`symmetric_decode_jwt` — JWE `dir`/`A256CBC-HS512` with HKDF-SHA256 key
//!   derivation, used by the encrypted cookie-cache strategy. Gated behind the `jwe` feature (lands
//!   with the josekit-backed implementation); HS256 is the default cookie-cache strategy here.

use std::time::{SystemTime, UNIX_EPOCH};

use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde_json::{Map, Value, json};

/// Errors from JWT signing. (Verification returns `None` on any failure, matching upstream.)
#[derive(Debug, thiserror::Error)]
pub enum JwtError {
    /// The JOSE library rejected the claims or key while signing.
    #[error("jwt encode failed: {0}")]
    Encode(#[from] jsonwebtoken::errors::Error),
    /// `sign_jwt` was given a payload that is not a JSON object.
    #[error("jwt payload must be a JSON object")]
    PayloadNotObject,
}

fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Sign `payload` (a JSON object) as an HS256 JWT, setting `iat` now and `exp` at `now + expires_in`
/// seconds. Mirrors upstream `signJWT` (default `expires_in` upstream is 3600).
pub fn sign_jwt(payload: &Value, secret: &str, expires_in: i64) -> Result<String, JwtError> {
    let mut claims: Map<String, Value> = payload
        .as_object()
        .cloned()
        .ok_or(JwtError::PayloadNotObject)?;
    let now = unix_now();
    claims.insert("iat".to_string(), json!(now));
    claims.insert("exp".to_string(), json!(now + expires_in));
    Ok(encode(
        &Header::new(Algorithm::HS256),
        &Value::Object(claims),
        &EncodingKey::from_secret(secret.as_bytes()),
    )?)
}

/// Verify an HS256 JWT and return its claims, or `None` on any failure (bad signature, expired,
/// malformed). Mirrors upstream `verifyJWT` (validates `exp` if present, no required claims, no
/// clock skew — matching jose's `jwtVerify` defaults).
#[must_use]
pub fn verify_jwt(token: &str, secret: &str) -> Option<Value> {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.required_spec_claims.clear();
    validation.leeway = 0;
    validation.validate_exp = true;
    validation.validate_nbf = false;
    validation.validate_aud = false;
    decode::<Value>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )
    .ok()
    .map(|data| data.claims)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn sign_and_verify_round_trip() {
        let payload = json!({ "sub": "user-123", "role": "admin" });
        let token = sign_jwt(&payload, "topsecret", 3600).unwrap();
        let claims = verify_jwt(&token, "topsecret").unwrap();
        assert_eq!(claims["sub"], json!("user-123"));
        assert_eq!(claims["role"], json!("admin"));
        assert!(claims.get("iat").is_some());
        assert!(claims.get("exp").is_some());
    }

    #[test]
    fn wrong_secret_fails() {
        let token = sign_jwt(&json!({ "sub": "x" }), "secret-a", 3600).unwrap();
        assert!(verify_jwt(&token, "secret-b").is_none());
    }

    #[test]
    fn tampered_token_fails() {
        let mut token = sign_jwt(&json!({ "sub": "x" }), "secret", 3600).unwrap();
        token.push('x');
        assert!(verify_jwt(&token, "secret").is_none());
    }

    #[test]
    fn expired_token_fails() {
        let token = sign_jwt(&json!({ "sub": "x" }), "secret", -10).unwrap();
        assert!(verify_jwt(&token, "secret").is_none());
    }

    #[test]
    fn non_object_payload_errors() {
        assert!(matches!(
            sign_jwt(&json!("a string"), "secret", 3600),
            Err(JwtError::PayloadNotObject)
        ));
    }
}
