//! Upstream reference: types/secret.ts
//!
//! `Map<number, string>` → [`BTreeMap<u32, String>`] (deterministic ordering; the lowest key is the
//! first/earliest version). Core is driver-light (no `secrecy` dep), so secret material is held as
//! plain `String` with a hand-written redacting [`Debug`] impl and a never-log convention.

use std::collections::BTreeMap;
use std::fmt;

/// Configuration for the rotating secret used by encryption/signing.
#[derive(Clone)]
pub struct SecretConfig {
    /// Map of version number → secret value.
    pub keys: BTreeMap<u32, String>,
    /// Version to use for new encryption (the first entry in the secrets array).
    pub current_version: u32,
    /// Legacy secret for bare-hex fallback (from `BETTER_AUTH_SECRET`).
    pub legacy_secret: Option<String>,
}

// Redacting Debug: never print secret material.
impl fmt::Debug for SecretConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SecretConfig")
            .field(
                "keys",
                &format_args!("<{} key(s) redacted>", self.keys.len()),
            )
            .field("current_version", &self.current_version)
            .field(
                "legacy_secret",
                &self.legacy_secret.as_ref().map(|_| "<redacted>"),
            )
            .finish()
    }
}
