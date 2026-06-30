//! Base64 / Base64URL (port of `base64.ts`, itself inspired by oslo).
//!
//! Hand-ported to match upstream byte-for-byte (and to avoid the same-named extern crate). Encoding
//! only — not a cryptographic primitive. `decode` auto-detects the URL-safe alphabet from the
//! presence of `-`/`_`, exactly as upstream does.

const STANDARD: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const URL_SAFE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

fn encode_with(data: &[u8], alphabet: &[u8; 64], padding: bool) -> String {
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    let mut buffer: u32 = 0;
    let mut shift: u32 = 0;
    for &byte in data {
        buffer = (buffer << 8) | u32::from(byte);
        shift += 8;
        while shift >= 6 {
            shift -= 6;
            out.push(alphabet[((buffer >> shift) & 0x3f) as usize] as char);
        }
    }
    if shift > 0 {
        out.push(alphabet[((buffer << (6 - shift)) & 0x3f) as usize] as char);
    }
    if padding {
        let pad = (4 - (out.len() % 4)) % 4;
        for _ in 0..pad {
            out.push('=');
        }
    }
    out
}

fn decode_with(data: &str, alphabet: &[u8; 64]) -> Result<Vec<u8>, Base64Error> {
    let mut reverse = [-1i16; 256];
    for (i, &c) in alphabet.iter().enumerate() {
        reverse[c as usize] = i as i16;
    }
    let mut out = Vec::with_capacity(data.len() / 4 * 3);
    let mut buffer: u32 = 0;
    let mut bits: u32 = 0;
    for ch in data.chars() {
        if ch == '=' {
            break;
        }
        let code = ch as u32;
        let value = if code < 256 {
            reverse[code as usize]
        } else {
            -1
        };
        if value < 0 {
            return Err(Base64Error::InvalidCharacter(ch));
        }
        buffer = (buffer << 6) | value as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push(((buffer >> bits) & 0xff) as u8);
        }
    }
    Ok(out)
}

/// Standard base64 (`+`/`/` alphabet).
#[allow(clippy::module_inception)]
pub mod base64 {
    use super::{Base64Error, STANDARD, URL_SAFE, decode_with, encode_with};

    /// Encode bytes as standard base64. `padding` controls trailing `=`.
    #[must_use]
    pub fn encode(data: impl AsRef<[u8]>, padding: bool) -> String {
        encode_with(data.as_ref(), STANDARD, padding)
    }

    /// Decode a base64 string, auto-detecting the URL-safe alphabet from `-`/`_`.
    pub fn decode(data: &str) -> Result<Vec<u8>, Base64Error> {
        let url_safe = data.contains('-') || data.contains('_');
        decode_with(data, if url_safe { URL_SAFE } else { STANDARD })
    }
}

/// URL-safe base64 (`-`/`_` alphabet).
pub mod base64_url {
    use super::{Base64Error, STANDARD, URL_SAFE, decode_with, encode_with};

    /// Encode bytes as URL-safe base64. `padding` controls trailing `=`.
    #[must_use]
    pub fn encode(data: impl AsRef<[u8]>, padding: bool) -> String {
        encode_with(data.as_ref(), URL_SAFE, padding)
    }

    /// Decode a base64 string, auto-detecting the URL-safe alphabet from `-`/`_`.
    pub fn decode(data: &str) -> Result<Vec<u8>, Base64Error> {
        let url_safe = data.contains('-') || data.contains('_');
        decode_with(data, if url_safe { URL_SAFE } else { STANDARD })
    }
}

/// Error returned when decoding encounters a character outside the active alphabet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Base64Error {
    /// A non-alphabet, non-padding character was encountered.
    #[error("Invalid Base64 character: {0}")]
    InvalidCharacter(char),
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn standard_known_vectors() {
        assert_eq!(base64::encode("", true), "");
        assert_eq!(base64::encode("f", true), "Zg==");
        assert_eq!(base64::encode("fo", true), "Zm8=");
        assert_eq!(base64::encode("foo", true), "Zm9v");
        assert_eq!(base64::encode("foobar", true), "Zm9vYmFy");
        assert_eq!(base64::encode("foo", false), "Zm9v");
        assert_eq!(base64::encode("fo", false), "Zm8");
    }

    #[test]
    fn url_safe_uses_dash_underscore() {
        // 0xfb 0xff -> standard "+/8=", url-safe "-_8="
        assert_eq!(base64::encode([0xfb, 0xff], true), "+/8=");
        assert_eq!(base64_url::encode([0xfb, 0xff], true), "-_8=");
        assert_eq!(base64_url::encode([0xfb, 0xff], false), "-_8");
    }

    #[test]
    fn round_trips_and_autodetects() {
        let bytes = [0u8, 1, 2, 0xfb, 0xff, 42];
        assert_eq!(base64::decode(&base64::encode(bytes, true)).unwrap(), bytes);
        // decode auto-detects url-safe alphabet from '-'/'_'
        assert_eq!(
            base64::decode(&base64_url::encode(bytes, false)).unwrap(),
            bytes
        );
    }

    #[test]
    fn rejects_invalid_char() {
        assert!(matches!(
            base64::decode("Zm9v*"),
            Err(Base64Error::InvalidCharacter('*'))
        ));
    }
}
