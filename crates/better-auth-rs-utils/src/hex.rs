//! Lowercase hexadecimal encoding (port of `hex.ts`).
//!
//! Hand-ported rather than pulling the `hex` crate: the upstream is itself hand-written, the logic
//! is trivial, and it keeps the module name free of a same-named extern crate. Encoding only — not a
//! cryptographic primitive.

const HEX: &[u8; 16] = b"0123456789abcdef";

/// Encode bytes as a lowercase hex string.
#[must_use]
pub fn encode(data: impl AsRef<[u8]>) -> String {
    let data = data.as_ref();
    let mut out = String::with_capacity(data.len() * 2);
    for &b in data {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    out
}

/// Decode a lowercase hex string into bytes.
///
/// Mirrors upstream's validation: even length and `[0-9a-f]` only (uppercase is rejected).
pub fn decode(data: &str) -> Result<Vec<u8>, HexError> {
    let bytes = data.as_bytes();
    if !bytes.len().is_multiple_of(2) {
        return Err(HexError::InvalidString);
    }
    let nibble = |c: u8| -> Result<u8, HexError> {
        match c {
            b'0'..=b'9' => Ok(c - b'0'),
            b'a'..=b'f' => Ok(c - b'a' + 10),
            _ => Err(HexError::InvalidString),
        }
    };
    let mut out = Vec::with_capacity(bytes.len() / 2);
    for pair in bytes.chunks_exact(2) {
        out.push((nibble(pair[0])? << 4) | nibble(pair[1])?);
    }
    Ok(out)
}

/// Error returned when [`decode`] is given a malformed hex string.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum HexError {
    /// The string had odd length or contained a non-`[0-9a-f]` character.
    #[error("Invalid hexadecimal string")]
    InvalidString,
}

#[cfg(test)]
#[path = "hex.test.rs"]
mod hex_tests;
