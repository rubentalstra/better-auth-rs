//! Port of `hmac.test.ts` (`createHMAC` import/sign/verify).
//!
//! Upstream's first case introspects the Web Crypto `CryptoKey` (`algorithm.name === "HMAC"`,
//! `hash.name === "SHA-256"`); that `CryptoKey` ceremony has no Rust analog (the key is raw bytes),
//! so it becomes the behavioral assertion that an HMAC-SHA-256 MAC is 32 bytes. Added beyond
//! upstream: RFC 4231 / RFC 2202 known-answer vectors and encoded-signature round-trips.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;

const ALGORITHM: ShaFamily = ShaFamily::Sha256;
const TEST_KEY: &str = "super-secret-key";
const TEST_DATA: &str = "Hello, HMAC!";

fn raw_len(e: &Encoded) -> usize {
    e.as_raw().expect("default encoding is `none` → raw").len()
}

// it("imports a key for HMAC") — adapted: an HMAC-SHA-256 MAC is 32 bytes (proves HMAC + SHA-256).
#[test]
fn imports_a_key_for_hmac() {
    let sig = create_hmac(ALGORITHM, EncodingFormat::None).sign(TEST_KEY, TEST_DATA);
    assert_eq!(raw_len(&sig), 32);
}

// it("signs data using HMAC")
#[test]
fn signs_data_using_hmac() {
    let sig = create_hmac(ShaFamily::Sha256, EncodingFormat::None).sign(TEST_KEY, TEST_DATA);
    assert!(raw_len(&sig) > 0);
    assert!(matches!(sig, Encoded::Raw(_)));
}

// it("verifies HMAC signature")
#[test]
fn verifies_hmac_signature() {
    let hmac = create_hmac(ShaFamily::Sha256, EncodingFormat::None);
    let sig = hmac.sign(TEST_KEY, TEST_DATA);
    assert!(hmac.verify(TEST_KEY, TEST_DATA, &sig));
}

// it("fails verification for modified data")
#[test]
fn fails_verification_for_modified_data() {
    let hmac = create_hmac(ALGORITHM, EncodingFormat::None);
    let sig = hmac.sign(TEST_KEY, TEST_DATA);
    assert!(!hmac.verify(TEST_KEY, "Modified data", &sig));
}

// it("fails verification for a different key")
#[test]
fn fails_verification_for_a_different_key() {
    let hmac = create_hmac(ALGORITHM, EncodingFormat::None);
    let sig = hmac.sign(TEST_KEY, TEST_DATA);
    assert!(!hmac.verify("different-secret-key", TEST_DATA, &sig));
}

// Encoded signatures round-trip (hex + the three base64 variants, which upstream maps to URL-safe).
#[test]
fn encoded_round_trips() {
    for encoding in [
        EncodingFormat::Hex,
        EncodingFormat::Base64,
        EncodingFormat::Base64Url,
        EncodingFormat::Base64UrlNoPad,
    ] {
        let hmac = create_hmac(ShaFamily::Sha256, encoding);
        let sig = hmac.sign(TEST_KEY, TEST_DATA);
        assert!(sig.as_text().is_some(), "{encoding:?} must encode to text");
        assert!(
            hmac.verify(TEST_KEY, TEST_DATA, &sig),
            "{encoding:?} verify"
        );
        assert!(
            !hmac.verify(TEST_KEY, "tampered", &sig),
            "{encoding:?} negative"
        );
    }
}

// RFC 4231 Test Case 1 (HMAC-SHA-256): key = 0x0b×20, data = "Hi There".
#[test]
fn rfc4231_sha256_vector() {
    let key = vec![0x0bu8; 20];
    let sig = create_hmac(ShaFamily::Sha256, EncodingFormat::Hex).sign(&key, "Hi There");
    assert_eq!(
        sig.as_text().unwrap(),
        "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7"
    );
}

// RFC 2202 Test Case 1 (HMAC-SHA-1): key = 0x0b×20, data = "Hi There". (otp relies on SHA-1.)
#[test]
fn rfc2202_sha1_vector() {
    let key = vec![0x0bu8; 20];
    let sig = create_hmac(ShaFamily::Sha1, EncodingFormat::Hex).sign(&key, "Hi There");
    assert_eq!(
        sig.as_text().unwrap(),
        "b617318655057264e28bc0b6fb378c8ef146be00"
    );
}
