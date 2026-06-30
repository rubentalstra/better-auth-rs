//! Tests for `base32.rs`. Upstream `@better-auth/utils` ships **no** `base32.test.ts`, so these are
//! Rust-authored, anchored on the canonical RFC 4648 §10 test vectors (base32 and base32hex), plus
//! padding, round-trip, and error coverage. The behavior under test is still a 1:1 port of
//! `base32.ts`.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;

// RFC 4648 §10 base32 vectors.
const STD_VECTORS: &[(&str, &str)] = &[
    ("", ""),
    ("f", "MY======"),
    ("fo", "MZXQ===="),
    ("foo", "MZXW6==="),
    ("foob", "MZXW6YQ="),
    ("fooba", "MZXW6YTB"),
    ("foobar", "MZXW6YTBOI======"),
];

// RFC 4648 §10 base32hex vectors.
const HEX_VECTORS: &[(&str, &str)] = &[
    ("", ""),
    ("f", "CO======"),
    ("fo", "CPNG===="),
    ("foo", "CPNMU==="),
    ("foob", "CPNMUOG="),
    ("fooba", "CPNMUOJ1"),
    ("foobar", "CPNMUOJ1E8======"),
];

#[test]
fn standard_rfc4648_vectors() {
    for (input, expected) in STD_VECTORS {
        assert_eq!(base32::encode(input, true), *expected, "encode {input:?}");
        assert_eq!(
            base32::decode(expected).unwrap(),
            input.as_bytes(),
            "decode {expected:?}"
        );
    }
}

#[test]
fn hex_rfc4648_vectors() {
    for (input, expected) in HEX_VECTORS {
        assert_eq!(
            base32hex::encode(input, true),
            *expected,
            "hex encode {input:?}"
        );
        assert_eq!(
            base32hex::decode(expected).unwrap(),
            input.as_bytes(),
            "hex decode {expected:?}"
        );
    }
}

#[test]
fn unpadded_matches_padding_stripped() {
    for (input, padded) in STD_VECTORS {
        let nopad = base32::encode(input, false);
        assert_eq!(nopad, padded.trim_end_matches('='));
        // Decoding tolerates the absence of padding (it stops at `=`, which simply isn't present).
        assert_eq!(base32::decode(&nopad).unwrap(), input.as_bytes());
    }
}

#[test]
fn round_trip_bytes() {
    let data: Vec<u8> = (0u8..=255).collect();
    let encoded = base32::encode(&data, true);
    assert_eq!(base32::decode(&encoded).unwrap(), data);
    let encoded_hex = base32hex::encode(&data, false);
    assert_eq!(base32hex::decode(&encoded_hex).unwrap(), data);
}

#[test]
fn rejects_invalid_characters() {
    // `0` and `1` are not in the standard base32 alphabet.
    assert_eq!(
        base32::decode("MY0====="),
        Err(Base32Error::InvalidCharacter('0'))
    );
    // lowercase is not accepted (alphabet is uppercase).
    assert_eq!(
        base32::decode("my======"),
        Err(Base32Error::InvalidCharacter('m'))
    );
    // `W` is outside the base32hex alphabet (`0–9A–V`).
    assert_eq!(
        base32hex::decode("CW======"),
        Err(Base32Error::InvalidCharacter('W'))
    );
}

// The shape `otp::url` relies on: a secret string encoded with the standard alphabet, no padding.
#[test]
fn otp_secret_shape() {
    let secret = "1234567890";
    let encoded = base32::encode(secret, false);
    assert!(!encoded.contains('='));
    assert_eq!(base32::decode(&encoded).unwrap(), secret.as_bytes());
}
