//! Port of `hash.test.ts` (`createHash(...).digest`).
//!
//! Upstream returns an `ArrayBuffer` (raw) or an encoded `string`; here that's `Vec<u8>` /
//! `String`. The upstream "unsupported algorithm" and "invalid input type" error cases are
//! unrepresentable in Rust — `ShaFamily` is a closed enum and input is a typed `impl AsRef<[u8]>`.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;

const INPUT: &str = "Hello, World!";

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
    let h = digest_encoded(ShaFamily::Sha256, INPUT, Encoding::Hex);
    // /^[a-f0-9]{64}$/
    assert_eq!(h.len(), 64);
    assert!(
        h.chars()
            .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c))
    );
}
#[test]
fn sha512_hex() {
    let h = digest_encoded(ShaFamily::Sha512, INPUT.as_bytes(), Encoding::Hex);
    assert_eq!(h.len(), 128);
    assert!(
        h.chars()
            .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c))
    );
}

// describe("Input variations") — string / bytes / owned bytes all hash
#[test]
fn input_variations() {
    assert_eq!(digest(ShaFamily::Sha256, INPUT).len(), 32);
    assert_eq!(digest(ShaFamily::Sha256, INPUT.as_bytes()).len(), 32);
    assert_eq!(digest(ShaFamily::Sha256, vec![1u8, 2, 3]).len(), 32);
}

// Known vectors (stronger than upstream's shape-only checks): FIPS 180-2 "abc".
#[test]
fn sha256_known_vector() {
    assert_eq!(
        crate::hex::encode(sha256("abc")),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert_eq!(
        digest_encoded(ShaFamily::Sha256, "abc", Encoding::Hex),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert_eq!(digest(ShaFamily::Sha384, "abc").len(), 48);
}
