//! SHA-2 hashing (port of `hash.ts`).
//!
//! Upstream's `createHash(algorithm, encoding?)` wraps Web Crypto `digest`. Here we expose the raw
//! digest plus an encoded variant over the audited `sha2` crate. SHA-1 is intentionally omitted
//! (better-auth never hashes with it); add it alongside its first consumer if ever needed.

use sha2::{Digest, Sha256, Sha384, Sha512};

use crate::{base64::base64_url, hex};

/// Supported SHA-2 families.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShaFamily {
    /// SHA-256 (32-byte digest).
    Sha256,
    /// SHA-384 (48-byte digest).
    Sha384,
    /// SHA-512 (64-byte digest).
    Sha512,
}

/// Output encoding for [`digest_encoded`], mirroring upstream's `EncodingFormat`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Encoding {
    /// Lowercase hex.
    Hex,
    /// URL-safe base64 with padding.
    Base64Url,
    /// URL-safe base64 without padding.
    Base64UrlNoPad,
}

/// Compute the raw digest of `data` under `algorithm`.
#[must_use]
pub fn digest(algorithm: ShaFamily, data: impl AsRef<[u8]>) -> Vec<u8> {
    let data = data.as_ref();
    match algorithm {
        ShaFamily::Sha256 => Sha256::digest(data).to_vec(),
        ShaFamily::Sha384 => Sha384::digest(data).to_vec(),
        ShaFamily::Sha512 => Sha512::digest(data).to_vec(),
    }
}

/// Compute the digest of `data` and encode it.
#[must_use]
pub fn digest_encoded(algorithm: ShaFamily, data: impl AsRef<[u8]>, encoding: Encoding) -> String {
    let raw = digest(algorithm, data);
    match encoding {
        Encoding::Hex => hex::encode(raw),
        Encoding::Base64Url => base64_url::encode(raw, true),
        Encoding::Base64UrlNoPad => base64_url::encode(raw, false),
    }
}

/// SHA-256 digest as a fixed 32-byte array — the common case (e.g. deriving a symmetric key from a
/// secret in the crypto layer).
#[must_use]
pub fn sha256(data: impl AsRef<[u8]>) -> [u8; 32] {
    Sha256::digest(data.as_ref()).into()
}

#[cfg(test)]
#[path = "hash.test.rs"]
mod hash_tests;
