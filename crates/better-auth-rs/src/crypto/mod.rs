//! Cryptography (port of `crypto/index.ts` + siblings).
//!
//! - symmetric authenticated encryption: XChaCha20-Poly1305 with a managed (random, prepended)
//!   24-byte nonce; the key is `SHA-256(secret)`; output is hex. Versioned secrets wrap the
//!   ciphertext in the `$ba$<version>$…` rotation envelope.
//! - HMAC-SHA256 signing ([`make_signature`]) for signed cookies.
//! - submodules: [`buffer`] (constant-time compare), [`password`] (scrypt), [`random`] (CSPRNG
//!   strings), [`jwt`] (HS256 / gated JWE).
//!
//! Wire format matches better-auth byte-for-byte: a ciphertext produced by the TS server decrypts
//! here and vice-versa (same AEAD, same `SHA-256(secret)` key, same `nonce || ciphertext || tag`
//! layout, same hex encoding).

pub mod buffer;
pub mod jwt;
pub mod password;
pub mod random;

pub use buffer::constant_time_equal;
pub use password::{hash_password, verify_password};
pub use random::{generate_random_string, generate_random_string_with};

pub use better_auth_rs_core::secret::{Secret, SecretConfig, SecretSource};
use better_auth_rs_utils::base64::base64;
use better_auth_rs_utils::hash::sha256;
use better_auth_rs_utils::hex;
use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// XChaCha20-Poly1305 nonce length (bytes), prepended to the ciphertext (managed-nonce layout).
const XNONCE_LEN: usize = 24;
const ENVELOPE_PREFIX: &str = "$ba$";

/// Errors from the symmetric crypto layer.
#[derive(Debug, thiserror::Error)]
pub enum CryptoError {
    /// AEAD encryption failed (effectively unreachable for valid inputs).
    #[error("encryption failed")]
    Encrypt,
    /// AEAD decryption/authentication failed, or the ciphertext was truncated.
    #[error("decryption failed")]
    Decrypt,
    /// The hex ciphertext could not be decoded.
    #[error("invalid ciphertext encoding")]
    InvalidEncoding,
    /// Decrypted bytes were not valid UTF-8.
    #[error("decrypted payload is not valid UTF-8")]
    InvalidUtf8,
    /// The current/required secret version was absent from the key set.
    #[error("secret version {0} not found in keys")]
    SecretVersionNotFound(u32),
    /// A bare (pre-envelope) payload was seen but no legacy secret is configured.
    #[error(
        "cannot decrypt legacy bare-hex payload: no legacy secret available (set BETTER_AUTH_SECRET)"
    )]
    NoLegacySecret,
    /// The OS CSPRNG was unavailable while generating a nonce.
    #[error("random generation failed")]
    Random,
}

/// A parsed rotation envelope: `$ba$<version>$<ciphertext>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Envelope {
    /// The secret version the ciphertext was encrypted under.
    pub version: u32,
    /// The hex ciphertext (without the envelope prefix).
    pub ciphertext: String,
}

/// Parse a `$ba$<version>$<ciphertext>` envelope, or `None` if `data` is not one.
#[must_use]
pub fn parse_envelope(data: &str) -> Option<Envelope> {
    let rest = data.strip_prefix(ENVELOPE_PREFIX)?;
    let sep = rest.find('$')?;
    let version = parse_version(&rest[..sep])?;
    Some(Envelope {
        version,
        ciphertext: rest[sep + 1..].to_string(),
    })
}

/// Parse the envelope version the way upstream's `parseInt(s, 10)` + `>= 0` check does: skip leading
/// ASCII whitespace, allow a leading `+`, read leading digits, and ignore trailing non-digits
/// (`"12abc"` -> 12). A leading `-` (negative) or no digits yields `None` (upstream's `version < 0`
/// / `NaN` rejection). Absurd values above `u32::MAX` also yield `None` (no such secret version).
fn parse_version(s: &str) -> Option<u32> {
    let s = s.trim_start_matches(|c: char| c.is_ascii_whitespace());
    let s = s.strip_prefix('+').unwrap_or(s);
    let digits: String = s.chars().take_while(char::is_ascii_digit).collect();
    if digits.is_empty() {
        return None;
    }
    digits.parse::<u32>().ok()
}

/// Format a rotation envelope: `$ba$<version>$<ciphertext>`.
#[must_use]
pub fn format_envelope(version: u32, ciphertext: &str) -> String {
    format!("{ENVELOPE_PREFIX}{version}${ciphertext}")
}

#[allow(clippy::expect_used)]
fn cipher_for(secret: &str) -> XChaCha20Poly1305 {
    let key = sha256(secret);
    // SHA-256 always yields the exact 32-byte key size, so this cannot fail.
    XChaCha20Poly1305::new_from_slice(&key).expect("SHA-256 digest is a valid 32-byte key")
}

fn raw_encrypt(secret: &str, data: &str) -> Result<String, CryptoError> {
    let cipher = cipher_for(secret);
    let mut nonce_bytes = [0u8; XNONCE_LEN];
    getrandom::fill(&mut nonce_bytes).map_err(|_| CryptoError::Random)?;
    let nonce: XNonce = nonce_bytes.into();
    let ciphertext = cipher
        .encrypt(&nonce, data.as_bytes())
        .map_err(|_| CryptoError::Encrypt)?;
    // Managed-nonce layout: nonce || ciphertext || tag, hex-encoded.
    let mut out = Vec::with_capacity(XNONCE_LEN + ciphertext.len());
    out.extend_from_slice(&nonce_bytes);
    out.extend_from_slice(&ciphertext);
    Ok(hex::encode(out))
}

fn raw_decrypt(secret: &str, ciphertext_hex: &str) -> Result<String, CryptoError> {
    // Upstream decodes with @noble/ciphers `hexToBytes`, which is case-insensitive; our `hex::decode`
    // is lowercase-only, so normalize first to accept upper/mixed-case ciphertext like noble does.
    let bytes = hex::decode(&ciphertext_hex.to_ascii_lowercase())
        .map_err(|_| CryptoError::InvalidEncoding)?;
    if bytes.len() < XNONCE_LEN {
        return Err(CryptoError::Decrypt);
    }
    let (nonce, ciphertext) = bytes.split_at(XNONCE_LEN);
    let nonce = XNonce::try_from(nonce).map_err(|_| CryptoError::Decrypt)?;
    let plaintext = cipher_for(secret)
        .decrypt(&nonce, ciphertext)
        .map_err(|_| CryptoError::Decrypt)?;
    String::from_utf8(plaintext).map_err(|_| CryptoError::InvalidUtf8)
}

/// Encrypt `data` with the given secret. A [`SecretSource::Plain`] yields a bare hex ciphertext; a
/// [`SecretSource::Versioned`] wraps it in the `$ba$<version>$…` envelope under `current_version`.
pub fn symmetric_encrypt(key: &SecretSource, data: &str) -> Result<String, CryptoError> {
    match key {
        SecretSource::Plain(secret) => raw_encrypt(secret.expose(), data),
        SecretSource::Versioned(config) => {
            let secret = config
                .current()
                .ok_or(CryptoError::SecretVersionNotFound(config.current_version))?;
            let ciphertext = raw_encrypt(secret.expose(), data)?;
            Ok(format_envelope(config.current_version, &ciphertext))
        }
    }
}

/// Decrypt `data`. For [`SecretSource::Versioned`], an envelope selects the version's key; a bare
/// (pre-envelope) payload falls back to `legacy_secret`. Mirrors `symmetricDecrypt`.
pub fn symmetric_decrypt(key: &SecretSource, data: &str) -> Result<String, CryptoError> {
    match key {
        SecretSource::Plain(secret) => raw_decrypt(secret.expose(), data),
        SecretSource::Versioned(config) => {
            if let Some(envelope) = parse_envelope(data) {
                let secret = config
                    .keys
                    .get(&envelope.version)
                    .ok_or(CryptoError::SecretVersionNotFound(envelope.version))?;
                return raw_decrypt(secret.expose(), &envelope.ciphertext);
            }
            match &config.legacy_secret {
                Some(legacy) => raw_decrypt(legacy.expose(), data),
                None => Err(CryptoError::NoLegacySecret),
            }
        }
    }
}

/// HMAC-SHA256 signature of `value` under `secret`, base64-encoded (standard alphabet, padded) —
/// the cookie-signing primitive (port of `makeSignature`). The result is always 44 chars.
#[must_use]
#[allow(clippy::expect_used)]
pub fn make_signature(value: &str, secret: &str) -> String {
    // HMAC accepts a key of any length, so `new_from_slice` cannot fail here.
    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC accepts any key length");
    mac.update(value.as_bytes());
    base64::encode(mac.finalize().into_bytes(), true)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn envelope_round_trips() {
        let e = format_envelope(3, "deadbeef");
        assert_eq!(e, "$ba$3$deadbeef");
        let parsed = parse_envelope(&e).unwrap();
        assert_eq!(parsed.version, 3);
        assert_eq!(parsed.ciphertext, "deadbeef");
        // ciphertext may itself contain '$' (only the first separator splits version)
        assert_eq!(parse_envelope("$ba$0$a$b$c").unwrap().ciphertext, "a$b$c");
    }

    #[test]
    fn envelope_rejects_non_envelopes() {
        assert!(parse_envelope("plainhex").is_none());
        assert!(parse_envelope("$ba$").is_none());
        assert!(parse_envelope("$ba$notanumber$ct").is_none());
        assert!(parse_envelope("$ba$-1$ct").is_none());
    }

    #[test]
    fn plain_encrypt_decrypt_round_trip() {
        let key = SecretSource::from("my-secret-key");
        let ct = symmetric_encrypt(&key, "hello world").unwrap();
        // bare hex, no envelope
        assert!(!ct.starts_with(ENVELOPE_PREFIX));
        assert_eq!(symmetric_decrypt(&key, &ct).unwrap(), "hello world");
    }

    #[test]
    fn nonce_is_random_so_ciphertexts_differ() {
        let key = SecretSource::from("k");
        assert_ne!(
            symmetric_encrypt(&key, "same").unwrap(),
            symmetric_encrypt(&key, "same").unwrap()
        );
    }

    #[test]
    fn wrong_secret_fails_to_decrypt() {
        let ct = symmetric_encrypt(&SecretSource::from("right"), "data").unwrap();
        assert!(symmetric_decrypt(&SecretSource::from("wrong"), &ct).is_err());
    }

    fn versioned(current: u32, pairs: &[(u32, &str)], legacy: Option<&str>) -> SecretSource {
        let keys: BTreeMap<u32, Secret> =
            pairs.iter().map(|(v, s)| (*v, Secret::new(*s))).collect();
        SecretSource::Versioned(SecretConfig {
            current_version: current,
            keys,
            legacy_secret: legacy.map(Secret::new),
        })
    }

    #[test]
    fn versioned_wraps_envelope_and_round_trips() {
        let cfg = versioned(2, &[(1, "old"), (2, "new")], None);
        let ct = symmetric_encrypt(&cfg, "payload").unwrap();
        assert!(ct.starts_with("$ba$2$"));
        assert_eq!(symmetric_decrypt(&cfg, &ct).unwrap(), "payload");
    }

    #[test]
    fn versioned_decrypts_older_version() {
        // encrypt under a config whose current version is 1, then decrypt with a config that still
        // holds version 1 but has rotated current to 2.
        let v1 = versioned(1, &[(1, "old")], None);
        let ct = symmetric_encrypt(&v1, "legacy-data").unwrap();
        let rotated = versioned(2, &[(1, "old"), (2, "new")], None);
        assert_eq!(symmetric_decrypt(&rotated, &ct).unwrap(), "legacy-data");
    }

    #[test]
    fn versioned_legacy_bare_payload() {
        // a bare-hex payload encrypted with the plain secret decrypts via legacy_secret
        let bare = symmetric_encrypt(&SecretSource::from("legacy-key"), "old-cookie").unwrap();
        let cfg = versioned(1, &[(1, "current")], Some("legacy-key"));
        assert_eq!(symmetric_decrypt(&cfg, &bare).unwrap(), "old-cookie");

        let no_legacy = versioned(1, &[(1, "current")], None);
        assert!(matches!(
            symmetric_decrypt(&no_legacy, &bare),
            Err(CryptoError::NoLegacySecret)
        ));
    }

    #[test]
    fn signature_is_stable_44_char_base64() {
        let sig = make_signature("session-token-value", "signing-secret");
        assert_eq!(sig, make_signature("session-token-value", "signing-secret"));
        assert_eq!(sig.len(), 44);
        assert!(sig.ends_with('='));
        assert_ne!(
            sig,
            make_signature("session-token-value", "different-secret")
        );
    }
}
