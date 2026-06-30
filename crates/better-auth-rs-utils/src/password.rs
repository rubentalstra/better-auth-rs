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
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    // Port of password.test.ts.

    #[test]
    fn hashes_in_salt_colon_key_format() {
        let hash = hash_password("mySecurePassword123!").unwrap();
        assert!(!hash.is_empty());
        assert_eq!(hash.split(':').count(), 2);
    }

    #[test]
    fn verifies_correct_and_rejects_incorrect() {
        let hash = hash_password("correctPassword123!").unwrap();
        assert!(verify_password(&hash, "correctPassword123!").unwrap());
        assert!(!verify_password(&hash, "wrongPassword456!").unwrap());
    }

    #[test]
    fn same_password_yields_different_hashes() {
        let h1 = hash_password("samePassword123!").unwrap();
        let h2 = hash_password("samePassword123!").unwrap();
        assert_ne!(h1, h2, "random salt should differ");
    }

    #[test]
    fn handles_long_passwords() {
        let pw = "a".repeat(1000);
        let hash = hash_password(&pw).unwrap();
        assert!(verify_password(&hash, &pw).unwrap());
    }

    #[test]
    fn is_case_sensitive() {
        let pw = "CaseSensitivePassword123!";
        let hash = hash_password(pw).unwrap();
        assert!(!verify_password(&hash, &pw.to_lowercase()).unwrap());
        assert!(!verify_password(&hash, &pw.to_uppercase()).unwrap());
    }

    #[test]
    fn handles_unicode() {
        let pw = "пароль123!";
        let hash = hash_password(pw).unwrap();
        assert!(verify_password(&hash, pw).unwrap());
    }

    #[test]
    fn rejects_malformed_hash() {
        assert!(matches!(
            verify_password("no-colon-here", "x"),
            Err(PasswordError::InvalidHash)
        ));
        assert!(matches!(
            verify_password(":missingsalt", "x"),
            Err(PasswordError::InvalidHash)
        ));
    }

    // Backward-compatibility with @better-auth/utils + @noble/hashes: these `salt:key` vectors were
    // produced by the reference scrypt (N=16384, r=16, p=1, dkLen=64; salt = the hex string's bytes;
    // NFKC) and MUST verify byte-for-byte against our implementation.
    const V_EXISTING: &str = "a1b2c3d4e5f60718293a4b5c6d7e8f90:765544082a079ea1373c9bf8154c17a023b860aa8620ce3e4c9c6a7749800c1c1e26311229e3cc4149a2b442f56b0847215f9b8008c7e4401ee8b9afe5dd3533";
    const V_EMPTY: &str = "a1b2c3d4e5f60718293a4b5c6d7e8f90:c9f3e1542c44f19dd81a719d71f0dc1234a301f05619a9594f47c60955241d08347331e8d637cbd170a5e8de965ccb89e56f42f1f4ad6a34b6121a72af88d589";
    const V_UNICODE: &str = "a1b2c3d4e5f60718293a4b5c6d7e8f90:e59b899cc3037a69278c0f9ff2dd16016b38a317cf16616a9a00ec6203e5740c05b46e743750189bf26ace8edf631fd49ad0e1472f44bf3c85bb6a3568ca4cfc";

    #[test]
    fn verifies_legacy_noble_vector() {
        assert!(verify_password(V_EXISTING, "ExistingUser123!").unwrap());
        assert!(!verify_password(V_EXISTING, "WrongPassword!").unwrap());
    }

    #[test]
    fn verifies_legacy_empty_password_vector() {
        assert!(verify_password(V_EMPTY, "").unwrap());
        assert!(!verify_password(V_EMPTY, "x").unwrap());
    }

    #[test]
    fn verifies_legacy_unicode_vector() {
        assert!(verify_password(V_UNICODE, "비밀번호🔑密码🔒パスワード").unwrap());
    }
}
