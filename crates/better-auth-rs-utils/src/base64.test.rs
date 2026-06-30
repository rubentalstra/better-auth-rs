//! Port of `base64.test.ts` (+ retained known-vector / round-trip checks).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use crate::binary;

const PLAIN: &str = "Hello, World!";
const B64: &str = "SGVsbG8sIFdvcmxkIQ==";
const B64_URL: &str = "SGVsbG8sIFdvcmxkIQ";

// describe("encode")
#[test]
fn encodes_with_padding() {
    assert_eq!(base64::encode(PLAIN, true), B64);
}
#[test]
fn encodes_without_padding() {
    assert_eq!(base64::encode(PLAIN, false), B64.trim_end_matches('='));
}
#[test]
fn encodes_url_safe() {
    assert_eq!(base64_url::encode(PLAIN, false), B64_URL);
}
#[test]
fn encodes_bytes() {
    assert_eq!(base64::encode(PLAIN.as_bytes(), true), B64);
}

// describe("decode")
#[test]
fn decodes_base64() {
    let bytes = base64::decode(B64).unwrap();
    assert_eq!(binary::decode(&bytes, binary::BinaryEncoding::Utf8), PLAIN);
}
#[test]
fn decodes_url_safe() {
    let bytes = base64::decode(B64_URL).unwrap();
    assert_eq!(binary::decode(&bytes, binary::BinaryEncoding::Utf8), PLAIN);
}

// RFC 4648 vectors + url-safe alphabet + round-trip + invalid char (retained, stronger).
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
