//! Port of `hash.test.ts` (`createHash(...).digest`).
//!
//! Upstream returns an `ArrayBuffer` (raw) or an encoded `string`; here that's [`digest`]
//! (`Vec<u8>`) and [`digest_encoded`] (the [`Encoded`] union). The upstream "unsupported algorithm"
//! and "invalid input type" error cases are unrepresentable in Rust — `ShaFamily` is a closed enum
//! and input is a typed `impl AsRef<[u8]>`. The SHA-1 and standard-`base64` cases are added here:
//! upstream's `SHAFamily`/`EncodingFormat` types support them, though `hash.test.ts` itself only
//! exercises SHA-256/512 in raw + hex.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;

const INPUT: &str = "Hello, World!";

fn text(e: Encoded) -> String {
    e.as_text().expect("expected an encoded string").to_string()
}

// describe("SHA algorithms")
#[test]
fn sha256_raw() {
    assert_eq!(digest(ShaFamily::Sha256, INPUT).len(), 32);
}
#[test]
fn sha512_raw() {
    assert_eq!(digest(ShaFamily::Sha512, INPUT.as_bytes()).len(), 64);
}
#[test]
fn sha256_hex() {
    let h = text(digest_encoded(
        ShaFamily::Sha256,
        INPUT,
        EncodingFormat::Hex,
    ));
    // /^[a-f0-9]{64}$/
    assert_eq!(h.len(), 64);
    assert!(
        h.chars()
            .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c))
    );
}
#[test]
fn sha512_hex() {
    let h = text(digest_encoded(
        ShaFamily::Sha512,
        INPUT.as_bytes(),
        EncodingFormat::Hex,
    ));
    assert_eq!(h.len(), 128);
    assert!(
        h.chars()
            .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c))
    );
}
// The remaining two families in the `SHAFamily` union (not in upstream's test, added for coverage).
#[test]
fn sha1_and_sha384_raw() {
    assert_eq!(digest(ShaFamily::Sha1, INPUT).len(), 20);
    assert_eq!(digest(ShaFamily::Sha384, INPUT).len(), 48);
}

// describe("Input variations") — string / bytes / owned bytes all hash
#[test]
fn input_variations() {
    assert_eq!(digest(ShaFamily::Sha256, INPUT).len(), 32);
    assert_eq!(digest(ShaFamily::Sha256, INPUT.as_bytes()).len(), 32);
    assert_eq!(digest(ShaFamily::Sha256, vec![1u8, 2, 3]).len(), 32);
}

// Every `EncodingFormat` variant on the FIPS 180-2 "abc" vector.
#[test]
fn encoding_variants() {
    const ABC_HEX: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

    // "none" → raw bytes
    match digest_encoded(ShaFamily::Sha256, "abc", EncodingFormat::None) {
        Encoded::Raw(b) => assert_eq!(b.len(), 32),
        Encoded::Text(_) => panic!("`none` encoding must be raw"),
    }
    assert_eq!(
        text(digest_encoded(
            ShaFamily::Sha256,
            "abc",
            EncodingFormat::Hex
        )),
        ABC_HEX
    );
    // standard base64 of SHA-256("abc")
    assert_eq!(
        text(digest_encoded(
            ShaFamily::Sha256,
            "abc",
            EncodingFormat::Base64
        )),
        "ungWv48Bz+pBQUDeXa4iI7ADYaOWF3qctBD/YfIAFa0="
    );
    // URL-safe base64, padded and unpadded
    assert_eq!(
        text(digest_encoded(
            ShaFamily::Sha256,
            "abc",
            EncodingFormat::Base64Url
        )),
        "ungWv48Bz-pBQUDeXa4iI7ADYaOWF3qctBD_YfIAFa0="
    );
    assert_eq!(
        text(digest_encoded(
            ShaFamily::Sha256,
            "abc",
            EncodingFormat::Base64UrlNoPad
        )),
        "ungWv48Bz-pBQUDeXa4iI7ADYaOWF3qctBD_YfIAFa0"
    );
}

// Known vectors (stronger than upstream's shape-only checks): FIPS 180-2 "abc".
#[test]
fn sha256_known_vector() {
    assert_eq!(
        crate::hex::encode(sha256("abc")),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert_eq!(digest(ShaFamily::Sha384, "abc").len(), 48);
}
