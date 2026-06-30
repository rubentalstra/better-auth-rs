//! HMAC signing / verification (port of `hmac.ts`).
//!
//! Upstream wraps Web Crypto HMAC. Its `importKey` step is a JS `CryptoKey` ceremony with no Rust
//! analog — here the key is simply raw bytes (`&str`'s UTF-8, or a byte slice). [`HmacBuilder::sign`]
//! returns the [`Encoded`] union (raw for `None`, else text); [`HmacBuilder::verify`] is
//! constant-time (RustCrypto's `Mac::verify_slice`), matching `subtle.verify`.
//!
//! Fidelity quirk (preserved bug-for-bug): like `hmac.ts`, the `Base64`, `Base64Url`, and
//! `Base64UrlNoPad` encodings all use the **URL-safe** alphabet (only padding differs) — unlike
//! `hash.ts`, where `Base64` is the standard alphabet.

use hmac::{Hmac, KeyInit, Mac};
use sha1::Sha1;
use sha2::{Sha256, Sha384, Sha512};

use crate::base64::{base64, base64_url};
use crate::hex;
pub use crate::types::{Encoded, EncodingFormat, ShaFamily};

// HMAC accepts a key of any length, so `new_from_slice` is infallible here.
macro_rules! hmac_sign {
    ($hash:ty, $key:expr, $data:expr) => {{
        let Ok(mut mac) = Hmac::<$hash>::new_from_slice($key) else {
            unreachable!("HMAC accepts keys of any length")
        };
        mac.update($data);
        mac.finalize().into_bytes().to_vec()
    }};
}

macro_rules! hmac_verify {
    ($hash:ty, $key:expr, $data:expr, $sig:expr) => {{
        let Ok(mut mac) = Hmac::<$hash>::new_from_slice($key) else {
            unreachable!("HMAC accepts keys of any length")
        };
        mac.update($data);
        mac.verify_slice($sig).is_ok()
    }};
}

fn sign_raw(algorithm: ShaFamily, key: &[u8], data: &[u8]) -> Vec<u8> {
    match algorithm {
        ShaFamily::Sha1 => hmac_sign!(Sha1, key, data),
        ShaFamily::Sha256 => hmac_sign!(Sha256, key, data),
        ShaFamily::Sha384 => hmac_sign!(Sha384, key, data),
        ShaFamily::Sha512 => hmac_sign!(Sha512, key, data),
    }
}

fn verify_raw(algorithm: ShaFamily, key: &[u8], data: &[u8], sig: &[u8]) -> bool {
    match algorithm {
        ShaFamily::Sha1 => hmac_verify!(Sha1, key, data, sig),
        ShaFamily::Sha256 => hmac_verify!(Sha256, key, data, sig),
        ShaFamily::Sha384 => hmac_verify!(Sha384, key, data, sig),
        ShaFamily::Sha512 => hmac_verify!(Sha512, key, data, sig),
    }
}

/// An HMAC configured with an algorithm and output encoding — the analogue of upstream's
/// `createHMAC(algorithm, encoding)`.
#[derive(Debug, Clone, Copy)]
pub struct HmacBuilder {
    algorithm: ShaFamily,
    encoding: EncodingFormat,
}

/// Build an HMAC. Upstream defaults are `algorithm = "SHA-256"`, `encoding = "none"`; Rust has no
/// default args, so pass them explicitly (e.g. `create_hmac(ShaFamily::Sha256, EncodingFormat::None)`).
#[must_use]
pub fn create_hmac(algorithm: ShaFamily, encoding: EncodingFormat) -> HmacBuilder {
    HmacBuilder {
        algorithm,
        encoding,
    }
}

impl HmacBuilder {
    /// Sign `data` with `key`, returning the MAC in the configured encoding.
    #[must_use]
    pub fn sign(&self, key: impl AsRef<[u8]>, data: impl AsRef<[u8]>) -> Encoded {
        let raw = sign_raw(self.algorithm, key.as_ref(), data.as_ref());
        match self.encoding {
            EncodingFormat::None => Encoded::Raw(raw),
            EncodingFormat::Hex => Encoded::Text(hex::encode(raw)),
            // `hmac.ts` uses URL-safe base64 for all three base64 variants; only padding differs.
            EncodingFormat::Base64 | EncodingFormat::Base64Url => {
                Encoded::Text(base64_url::encode(raw, true))
            }
            EncodingFormat::Base64UrlNoPad => Encoded::Text(base64_url::encode(raw, false)),
        }
    }

    /// Verify that `signature` is a valid MAC of `data` under `key`. The signature must be in the
    /// configured encoding (the same shape [`sign`](Self::sign) produces). Comparison is
    /// constant-time; a malformed/mis-encoded signature returns `false`.
    #[must_use]
    pub fn verify(
        &self,
        key: impl AsRef<[u8]>,
        data: impl AsRef<[u8]>,
        signature: &Encoded,
    ) -> bool {
        let sig_bytes = match (self.encoding, signature) {
            (EncodingFormat::None, Encoded::Raw(b)) => b.clone(),
            (EncodingFormat::Hex, Encoded::Text(s)) => match hex::decode(s) {
                Ok(b) => b,
                Err(_) => return false,
            },
            (
                EncodingFormat::Base64 | EncodingFormat::Base64Url | EncodingFormat::Base64UrlNoPad,
                Encoded::Text(s),
            ) => match base64::decode(s) {
                Ok(b) => b,
                Err(_) => return false,
            },
            // Encoding/shape mismatch (e.g. a `Text` signature for a `None` HMAC).
            _ => return false,
        };
        verify_raw(self.algorithm, key.as_ref(), data.as_ref(), &sig_bytes)
    }
}

#[cfg(test)]
#[path = "hmac.test.rs"]
mod hmac_tests;
