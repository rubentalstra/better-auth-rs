//! Port of `hex.test.ts`. Upstream `hex.decode` returns the decoded bytes as a UTF-8 string; our
//! `decode` returns the bytes, so the string-oriented cases convert via `String::from_utf8`.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;

// describe("encode")
#[test]
fn encode_string_to_hex() {
    // Buffer.from("Hello, World!").toString("hex")
    assert_eq!(encode("Hello, World!"), "48656c6c6f2c20576f726c6421");
}
#[test]
fn encode_bytes_to_hex() {
    // new Uint8Array([72,101,108,108,111]) -> "Hello"
    assert_eq!(encode([72u8, 101, 108, 108, 111]), "48656c6c6f");
}

// describe("decode")
#[test]
fn decode_hex_to_original() {
    let bytes = decode("48656c6c6f2c20576f726c6421").unwrap();
    assert_eq!(String::from_utf8(bytes).unwrap(), "Hello, World!");
}
#[test]
fn decode_throws_for_odd_length() {
    assert!(decode("123").is_err());
}
#[test]
fn decode_throws_for_non_hex() {
    assert!(decode("zzzz").is_err());
}

// describe("round-trip tests")
#[test]
fn round_trip_string() {
    let input = "Hello, Hex!";
    let bytes = decode(&encode(input)).unwrap();
    assert_eq!(String::from_utf8(bytes).unwrap(), input);
}
#[test]
fn round_trip_empty() {
    let bytes = decode(&encode("")).unwrap();
    assert_eq!(String::from_utf8(bytes).unwrap(), "");
}

// Rust-specific: upstream's regex accepts only lowercase [0-9a-f]; uppercase is rejected.
#[test]
fn uppercase_rejected() {
    assert!(decode("DEAD").is_err());
}
