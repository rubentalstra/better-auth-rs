//! Tests for `binary.rs`. Upstream `@better-auth/utils` ships **no** `binary.test.ts`, so these are
//! Rust-authored — covering the UTF-8 round-trip plus the UTF-16 (LE, BOM) and ISO-8859-1 decode
//! paths and the lossy `TextDecoder` behavior.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;

#[test]
fn encode_is_utf8() {
    assert_eq!(encode("AbC"), &[0x41, 0x62, 0x43]);
    // Multi-byte: "é" is U+00E9 → 0xC3 0xA9 in UTF-8.
    assert_eq!(encode("é"), &[0xC3, 0xA9]);
}

#[test]
fn decode_utf8_round_trip() {
    let s = "Hello, 世界 🌍";
    assert_eq!(decode(encode(s), BinaryEncoding::Utf8), s);
    // Default encoding is UTF-8.
    assert_eq!(decode(encode(s), BinaryEncoding::default()), s);
}

#[test]
fn decode_utf8_is_lossy() {
    // 0x80 is an invalid lone continuation byte → U+FFFD (TextDecoder is non-fatal).
    assert_eq!(decode(&[0x80], BinaryEncoding::Utf8), "\u{FFFD}");
}

#[test]
fn decode_utf16_le_with_bom() {
    // BOM (FF FE) + "Hi" in UTF-16LE.
    let bytes = [0xFF, 0xFE, 0x48, 0x00, 0x69, 0x00];
    assert_eq!(decode(&bytes, BinaryEncoding::Utf16), "Hi");
    // Without a BOM, the same units decode identically.
    assert_eq!(decode(&bytes[2..], BinaryEncoding::Utf16), "Hi");
}

#[test]
fn decode_iso_8859_1() {
    // In latin1 every byte is its own code point: 0x41 → "A", 0xE9 → "é", 0xFF → "ÿ".
    assert_eq!(
        decode(&[0x41, 0xE9, 0xFF], BinaryEncoding::Iso8859_1),
        "A\u{E9}\u{FF}"
    );
}
