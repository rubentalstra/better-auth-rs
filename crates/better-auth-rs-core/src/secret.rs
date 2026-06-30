//! Secret material and the versioned secret config used for encryption/signing key rotation
//! (port of `@better-auth/core` `SecretConfig`).

use std::collections::BTreeMap;
use std::fmt;

/// A secret string that redacts itself in `Debug`/logs. Expose the raw value only at the crypto
/// boundary via [`Secret::expose`].
#[derive(Clone, PartialEq, Eq)]
pub struct Secret(String);

impl Secret {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
    /// The raw secret. Use only where the bytes are actually needed (hashing/signing/encryption).
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Secret(***)")
    }
}

impl From<&str> for Secret {
    fn from(s: &str) -> Self {
        Self(s.to_owned())
    }
}
impl From<String> for Secret {
    fn from(s: String) -> Self {
        Self(s)
    }
}

/// Versioned secret configuration for key rotation (port of `SecretConfig`).
///
/// New ciphertexts are tagged with `current_version` (via the `$ba$<version>$…` envelope); older
/// versions stay in `keys` for decryption-only. `legacy_secret` decrypts pre-envelope bare payloads.
#[derive(Clone, Debug)]
pub struct SecretConfig {
    pub current_version: u32,
    pub keys: BTreeMap<u32, Secret>,
    pub legacy_secret: Option<Secret>,
}

impl SecretConfig {
    /// The secret for the current version, if present.
    pub fn current(&self) -> Option<&Secret> {
        self.keys.get(&self.current_version)
    }
}

/// Either a bare secret (no rotation envelope) or a versioned config — the Rust form of
/// better-auth's `string | SecretConfig` secret argument.
#[derive(Clone, Debug)]
pub enum SecretSource {
    /// A single secret; ciphertexts are bare (no `$ba$` envelope).
    Plain(Secret),
    /// Versioned secrets; ciphertexts carry the `$ba$<version>$` envelope.
    Versioned(SecretConfig),
}

impl From<&str> for SecretSource {
    fn from(s: &str) -> Self {
        Self::Plain(Secret::new(s))
    }
}
impl From<String> for SecretSource {
    fn from(s: String) -> Self {
        Self::Plain(Secret::new(s))
    }
}
impl From<SecretConfig> for SecretSource {
    fn from(c: SecretConfig) -> Self {
        Self::Versioned(c)
    }
}
