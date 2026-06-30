//! SHA hashing (port of `hash.ts`).
//!
//! Upstream's `createHash(algorithm, encoding?).digest(input)` wraps Web Crypto `digest`, returning
//! an `ArrayBuffer` (encoding `"none"`) or an encoded `string`. Here [`digest`] returns the raw
//! bytes and [`digest_encoded`] returns the [`Encoded`] union (raw for `None`, else text), over the
//! audited `sha1`/`sha2` crates. SHA-1 is included for parity (HOTP/TOTP use it) but is never used
//! to hash secrets or passwords.
//!
//! Fidelity note: per `hash.ts`, the `"base64"` encoding uses the **standard** alphabet — unlike
//! `hmac.ts`, which (quirk) uses URL-safe base64 for that same name.

use sha1::Sha1;
use sha2::{Digest, Sha256, Sha384, Sha512};

use crate::base64::{base64, base64_url};
use crate::hex;
pub use crate::types::{Encoded, EncodingFormat, ShaFamily};

/// Compute the raw digest of `data` under `algorithm`.
#[must_use]
pub fn digest(algorithm: ShaFamily, data: impl AsRef<[u8]>) -> Vec<u8> {
    let data = data.as_ref();
    match algorithm {
        ShaFamily::Sha1 => Sha1::digest(data).to_vec(),
        ShaFamily::Sha256 => Sha256::digest(data).to_vec(),
        ShaFamily::Sha384 => Sha384::digest(data).to_vec(),
        ShaFamily::Sha512 => Sha512::digest(data).to_vec(),
    }
}

/// Compute the digest of `data` and encode it, mirroring `createHash(algorithm, encoding).digest`.
#[must_use]
pub fn digest_encoded(
    algorithm: ShaFamily,
    data: impl AsRef<[u8]>,
    encoding: EncodingFormat,
) -> Encoded {
    let raw = digest(algorithm, data);
    match encoding {
        EncodingFormat::None => Encoded::Raw(raw),
        EncodingFormat::Hex => Encoded::Text(hex::encode(raw)),
        // `hash.ts` uses the standard alphabet for `"base64"`.
        EncodingFormat::Base64 => Encoded::Text(base64::encode(raw, true)),
        EncodingFormat::Base64Url => Encoded::Text(base64_url::encode(raw, true)),
        EncodingFormat::Base64UrlNoPad => Encoded::Text(base64_url::encode(raw, false)),
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
