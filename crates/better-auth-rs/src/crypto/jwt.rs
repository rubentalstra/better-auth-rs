//! JWT signing/verification (port of `crypto/jwt.ts`).
//!
//! - [`sign_jwt`]/[`verify_jwt`] — HS256 JWS over the cookie-cache payload (pure Rust, via
//!   `jsonwebtoken`). Always available.
//! - `symmetric_encode_jwt`/`symmetric_decode_jwt` — JWE `dir`/`A256CBC-HS512` with HKDF-SHA256 key
//!   derivation (josekit), used by the encrypted cookie-cache strategy. Gated behind the `jwe`
//!   feature; HS256 is the default cookie-cache strategy here.

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

/// JWE `dir` / `A256CBC-HS512` encrypted JWTs for the encrypted cookie-cache strategy (port of
/// `symmetricEncodeJWT`/`symmetricDecodeJWT`). Behind the `jwe` feature.
#[cfg(feature = "jwe")]
mod jwe {
    use std::time::{Duration, SystemTime};

    use better_auth_rs_utils::base64::base64_url;
    use better_auth_rs_utils::hash::sha256;
    use hkdf::Hkdf;
    use josekit::jwe::{Dir, JweHeader};
    use josekit::jwt::{self, JwtPayload};
    use serde_json::Value;
    use sha2::Sha256;

    use super::super::SecretSource;
    use super::super::random::generate_random_string;

    // "BetterAuth.js Generated Encryption Key" (HKDF info).
    const INFO: &[u8] = b"BetterAuth.js Generated Encryption Key";
    const ENC: &str = "A256CBC-HS512";
    const CLOCK_TOLERANCE_SECS: u64 = 15;

    /// Errors from the JWE cookie-cache codec.
    #[derive(Debug, thiserror::Error)]
    pub enum JweError {
        /// The JOSE layer rejected the key, header, or ciphertext.
        #[error("jose error: {0}")]
        Jose(#[from] josekit::JoseError),
        /// `symmetric_encode_jwt` was given a payload that is not a JSON object.
        #[error("jwe payload must be a JSON object")]
        PayloadNotObject,
        /// HKDF expansion failed (unreachable for our 64-byte output).
        #[error("hkdf expand failed")]
        Hkdf,
        /// The current secret version was absent from the key set.
        #[error("secret version not found in keys")]
        SecretVersionNotFound,
    }

    fn derive_encryption_secret(secret: &str, salt: &str) -> Result<[u8; 64], JweError> {
        let hk = Hkdf::<Sha256>::new(Some(salt.as_bytes()), secret.as_bytes());
        let mut okm = [0u8; 64];
        hk.expand(INFO, &mut okm).map_err(|_| JweError::Hkdf)?;
        Ok(okm)
    }

    fn jwk_thumbprint(key: &[u8]) -> String {
        // RFC 7638 thumbprint of an `oct` JWK: SHA-256 over the canonical {"k","kty"} JSON,
        // base64url (no pad) — matching jose's `calculateJwkThumbprint`.
        let k = base64_url::encode(key, false);
        let json = format!("{{\"k\":\"{k}\",\"kty\":\"oct\"}}");
        base64_url::encode(sha256(json.as_bytes()), false)
    }

    fn current_secret(secret: &SecretSource) -> Result<String, JweError> {
        match secret {
            SecretSource::Plain(s) => Ok(s.expose().to_string()),
            SecretSource::Versioned(c) => c
                .current()
                .map(|s| s.expose().to_string())
                .ok_or(JweError::SecretVersionNotFound),
        }
    }

    fn all_secrets(secret: &SecretSource) -> Vec<String> {
        match secret {
            SecretSource::Plain(s) => vec![s.expose().to_string()],
            SecretSource::Versioned(c) => {
                let mut out: Vec<String> =
                    c.keys.values().map(|s| s.expose().to_string()).collect();
                if let Some(legacy) = &c.legacy_secret {
                    let legacy = legacy.expose().to_string();
                    if !out.contains(&legacy) {
                        out.push(legacy);
                    }
                }
                out
            }
        }
    }

    /// Encrypt `payload` (a JSON object) as a JWE (`dir` / `A256CBC-HS512`), keyed by an
    /// HKDF-SHA256 derivation of the current secret + `salt`. Sets `iat`, `exp = now + expires_in`,
    /// a random `jti`, and `kid` = the key's JWK thumbprint. Port of `symmetricEncodeJWT`.
    pub fn symmetric_encode_jwt(
        payload: &Value,
        secret: &SecretSource,
        salt: &str,
        expires_in: i64,
    ) -> Result<String, JweError> {
        let enc_secret = derive_encryption_secret(&current_secret(secret)?, salt)?;
        let kid = jwk_thumbprint(&enc_secret);

        let obj = payload.as_object().ok_or(JweError::PayloadNotObject)?;
        let mut jp = JwtPayload::new();
        for (k, v) in obj {
            jp.set_claim(k, Some(v.clone()))?;
        }
        let now = SystemTime::now();
        jp.set_issued_at(&now);
        // exp = now + expires_in; a negative expires_in yields a past expiry (matches upstream).
        let exp = if expires_in >= 0 {
            now + Duration::from_secs(expires_in as u64)
        } else {
            now - Duration::from_secs(expires_in.unsigned_abs())
        };
        jp.set_expires_at(&exp);
        let jti = generate_random_string(36);
        jp.set_jwt_id(&jti);

        let mut header = JweHeader::new();
        header.set_token_type("JWT");
        header.set_content_encryption(ENC);
        header.set_key_id(&kid);

        let encrypter = Dir.encrypter_from_bytes(enc_secret)?;
        Ok(jwt::encode_with_encrypter(&jp, &header, &encrypter)?)
    }

    /// Decrypt a JWE produced by [`symmetric_encode_jwt`], returning its claims or `None` on any
    /// failure (no matching key, bad ciphertext, expired beyond a 15s skew). Port of
    /// `symmetricDecodeJWT`.
    #[must_use]
    pub fn symmetric_decode_jwt(token: &str, secret: &SecretSource, salt: &str) -> Option<Value> {
        if token.is_empty() {
            return None;
        }
        let header = jwt::decode_header(token).ok()?;
        let kid = header
            .claim("kid")
            .and_then(|v| v.as_str())
            .map(str::to_string);

        let secrets = all_secrets(secret);
        let keys: Vec<[u8; 64]> = match &kid {
            // kid present: use the single key whose thumbprint matches.
            Some(kid) => secrets
                .iter()
                .filter_map(|s| derive_encryption_secret(s, salt).ok())
                .find(|k| &jwk_thumbprint(k) == kid)
                .into_iter()
                .collect(),
            // kid absent: try each secret in turn.
            None => secrets
                .iter()
                .filter_map(|s| derive_encryption_secret(s, salt).ok())
                .collect(),
        };

        for key in keys {
            let Ok(decrypter) = Dir.decrypter_from_bytes(key) else {
                continue;
            };
            let Ok((payload, _header)) = jwt::decode_with_decrypter(token, &decrypter) else {
                continue;
            };
            if let Some(exp) = payload.expires_at()
                && SystemTime::now() > exp + Duration::from_secs(CLOCK_TOLERANCE_SECS)
            {
                return None;
            }
            return serde_json::from_str(&payload.to_string()).ok();
        }
        None
    }
}

#[cfg(feature = "jwe")]
pub use jwe::{JweError, symmetric_decode_jwt, symmetric_encode_jwt};

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

#[cfg(all(test, feature = "jwe"))]
#[allow(clippy::unwrap_used)]
mod jwe_tests {
    use super::{symmetric_decode_jwt, symmetric_encode_jwt};
    use better_auth_rs_core::secret::{Secret, SecretConfig, SecretSource};
    use serde_json::json;
    use std::collections::BTreeMap;

    #[test]
    fn jwe_round_trip() {
        let secret = SecretSource::from("super-secret");
        let token = symmetric_encode_jwt(
            &json!({ "userId": "u1", "role": "admin" }),
            &secret,
            "salt",
            3600,
        )
        .unwrap();
        // it is an encrypted compact JWE (five dot-separated parts), not the cleartext
        assert_eq!(token.matches('.').count(), 4);
        let claims = symmetric_decode_jwt(&token, &secret, "salt").unwrap();
        assert_eq!(claims["userId"], json!("u1"));
        assert_eq!(claims["role"], json!("admin"));
    }

    #[test]
    fn wrong_salt_or_secret_fails() {
        let secret = SecretSource::from("s");
        let token = symmetric_encode_jwt(&json!({ "a": 1 }), &secret, "salt-a", 3600).unwrap();
        assert!(symmetric_decode_jwt(&token, &secret, "salt-b").is_none());
        assert!(symmetric_decode_jwt(&token, &SecretSource::from("other"), "salt-a").is_none());
    }

    #[test]
    fn tampered_and_empty_fail() {
        let secret = SecretSource::from("s");
        let mut token = symmetric_encode_jwt(&json!({ "a": 1 }), &secret, "salt", 3600).unwrap();
        token.push('x');
        assert!(symmetric_decode_jwt(&token, &secret, "salt").is_none());
        assert!(symmetric_decode_jwt("", &secret, "salt").is_none());
    }

    #[test]
    fn expired_token_fails() {
        let secret = SecretSource::from("s");
        // exp well past the 15s tolerance
        let token = symmetric_encode_jwt(&json!({ "a": 1 }), &secret, "salt", -60).unwrap();
        assert!(symmetric_decode_jwt(&token, &secret, "salt").is_none());
    }

    #[test]
    fn versioned_kid_resolves_after_rotation() {
        let v1 = SecretSource::Versioned(SecretConfig {
            current_version: 1,
            keys: BTreeMap::from([(1, Secret::new("k1"))]),
            legacy_secret: None,
        });
        let token = symmetric_encode_jwt(&json!({ "a": 1 }), &v1, "salt", 3600).unwrap();
        // rotate: current is now 2, but version 1's key is retained for decryption
        let rotated = SecretSource::Versioned(SecretConfig {
            current_version: 2,
            keys: BTreeMap::from([(1, Secret::new("k1")), (2, Secret::new("k2"))]),
            legacy_secret: None,
        });
        assert_eq!(
            symmetric_decode_jwt(&token, &rotated, "salt").unwrap()["a"],
            json!(1)
        );
    }
}
