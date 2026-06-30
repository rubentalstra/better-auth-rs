//! Password hashing (port of `password.ts`) — scrypt via the audited `scrypt` crate.
//!
//! Parameters match upstream exactly: `N=16384 (log2=14)`, `r=16`, `p=1`, 64-byte derived key,
//! a 16-byte random salt, NFKC normalization, and the `"{saltHex}:{keyHex}"` storage format.
//!
//! Two parity-critical details mirrored from `@noble/hashes`:
//! - the salt fed to scrypt is the **hex string** of the 16 random bytes (its UTF-8 bytes, 32 of
//!   them), not the raw bytes;
//! - verification compares the derived key in **constant time** (upstream uses `===` on the hex,
//!   which is observably identical but timing-variable — we decode and compare bytes via `subtle`).

use subtle::ConstantTimeEq;
use unicode_normalization::UnicodeNormalization;

use crate::hex;

const LOG_N: u8 = 14; // N = 16384
const R: u32 = 16;
const P: u32 = 1;
const DK_LEN: usize = 64;
const SALT_LEN: usize = 16;

/// Errors from password hashing/verification.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PasswordError {
    /// The stored hash was not the expected `"{salt}:{key}"` form.
    #[error("Invalid password hash")]
    InvalidHash,
    /// scrypt configuration or output length was rejected (should be unreachable with our constants).
    #[error("scrypt error: {0}")]
    Scrypt(&'static str),
    /// The OS CSPRNG was unavailable while generating a salt.
    #[error("could not generate salt: {0}")]
    Random(&'static str),
}

fn derive_key(password: &str, salt: &[u8]) -> Result<[u8; DK_LEN], PasswordError> {
    let normalized: String = password.nfkc().collect();
    // scrypt 0.12 `Params::new` takes (log_n, r, p); the derived-key length is set by the output
    // buffer passed to `scrypt::scrypt` below (DK_LEN bytes).
    let params =
        scrypt::Params::new(LOG_N, R, P).map_err(|_| PasswordError::Scrypt("invalid params"))?;
    let mut out = [0u8; DK_LEN];
    scrypt::scrypt(normalized.as_bytes(), salt, &params, &mut out)
        .map_err(|_| PasswordError::Scrypt("invalid output length"))?;
    Ok(out)
}

/// Hash a password, returning `"{saltHex}:{keyHex}"`.
pub fn hash_password(password: &str) -> Result<String, PasswordError> {
    let mut salt = [0u8; SALT_LEN];
    getrandom::fill(&mut salt).map_err(|_| PasswordError::Random("OS CSPRNG unavailable"))?;
    let salt_hex = hex::encode(salt);
    // Salt fed to scrypt is the hex string's bytes, matching @noble/hashes.
    let key = derive_key(password, salt_hex.as_bytes())?;
    Ok(format!("{salt_hex}:{}", hex::encode(key)))
}

/// Verify `password` against a `"{saltHex}:{keyHex}"` hash. Comparison is constant-time.
pub fn verify_password(hash: &str, password: &str) -> Result<bool, PasswordError> {
    let (salt_hex, key_hex) = hash.split_once(':').ok_or(PasswordError::InvalidHash)?;
    if salt_hex.is_empty() || key_hex.is_empty() {
        return Err(PasswordError::InvalidHash);
    }
    let derived = derive_key(password, salt_hex.as_bytes())?;
    let Ok(expected) = hex::decode(key_hex) else {
        return Ok(false);
    };
    if expected.len() != derived.len() {
        return Ok(false);
    }
    Ok(derived.ct_eq(&expected).into())
}

#[cfg(test)]
#[path = "password.test.rs"]
mod password_tests;
