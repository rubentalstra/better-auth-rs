//! Base32 / Base32hex (port of `base32.ts`, itself inspired by oslo).
//!
//! Hand-ported to match upstream byte-for-byte (and to avoid a same-named extern crate), mirroring
//! [`crate::base64`]. RFC 4648 §6 (`base32`, alphabet `A–Z2–7`) and §7 (`base32hex`, alphabet
//! `0–9A–V`). Encoding only — not a cryptographic primitive. Used by [`crate::otp`] to render the
//! `otpauth://` secret.

const STANDARD: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
const HEX: &[u8; 32] = b"0123456789ABCDEFGHIJKLMNOPQRSTUV";

fn encode_with(data: &[u8], alphabet: &[u8; 32], padding: bool) -> String {
    let mut out = String::with_capacity(data.len().div_ceil(5) * 8);
    // 5-bit packing; the buffer never holds more than 12 bits, so `u32` is ample.
    let mut buffer: u32 = 0;
    let mut shift: u32 = 0;
    for &byte in data {
        buffer = (buffer << 8) | u32::from(byte);
        shift += 8;
        while shift >= 5 {
            shift -= 5;
            out.push(alphabet[((buffer >> shift) & 0x1f) as usize] as char);
        }
    }
    if shift > 0 {
        out.push(alphabet[((buffer << (5 - shift)) & 0x1f) as usize] as char);
    }
    if padding {
        let pad = (8 - (out.len() % 8)) % 8;
        for _ in 0..pad {
            out.push('=');
        }
    }
    out
}

fn decode_with(data: &str, alphabet: &[u8; 32]) -> Result<Vec<u8>, Base32Error> {
    let mut reverse = [-1i16; 256];
    for (i, &c) in alphabet.iter().enumerate() {
        reverse[c as usize] = i as i16;
    }
    let mut out = Vec::with_capacity(data.len() / 8 * 5);
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
            return Err(Base32Error::InvalidCharacter(ch));
        }
        buffer = (buffer << 5) | value as u32;
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            out.push(((buffer >> bits) & 0xff) as u8);
        }
    }
    Ok(out)
}

/// Standard base32 (RFC 4648 §6, `A–Z2–7` alphabet).
#[allow(clippy::module_inception)]
pub mod base32 {
    use super::{Base32Error, STANDARD, decode_with, encode_with};

    /// Encode bytes as standard base32. `padding` controls trailing `=`.
    #[must_use]
    pub fn encode(data: impl AsRef<[u8]>, padding: bool) -> String {
        encode_with(data.as_ref(), STANDARD, padding)
    }

    /// Decode a standard base32 string (stops at the first `=`).
    pub fn decode(data: &str) -> Result<Vec<u8>, Base32Error> {
        decode_with(data, STANDARD)
    }
}

/// Extended-hex base32 (RFC 4648 §7, `0–9A–V` alphabet).
pub mod base32hex {
    use super::{Base32Error, HEX, decode_with, encode_with};

    /// Encode bytes as base32hex. `padding` controls trailing `=`.
    #[must_use]
    pub fn encode(data: impl AsRef<[u8]>, padding: bool) -> String {
        encode_with(data.as_ref(), HEX, padding)
    }

    /// Decode a base32hex string (stops at the first `=`).
    pub fn decode(data: &str) -> Result<Vec<u8>, Base32Error> {
        decode_with(data, HEX)
    }
}

/// Error returned when decoding encounters a character outside the active alphabet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Base32Error {
    /// A non-alphabet, non-padding character was encountered (upstream throws
    /// `Invalid Base32 character: {0}`).
    #[error("Invalid Base32 character: {0}")]
    InvalidCharacter(char),
}

#[cfg(test)]
#[path = "base32.test.rs"]
mod base32_tests;
