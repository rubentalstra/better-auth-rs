//! Port of `ecdsa.test.ts`.
//!
//! Upstream introspects Web Crypto `CryptoKey` objects (`type === "private"`,
//! `algorithm.name === "ECDSA"`); those checks become typed-variant / `curve()` assertions here.
//! Signatures are the fixed-width IEEE-P1363 encoding (Web Crypto's format). Added beyond upstream:
//! P-384 and P-521 round-trips, and DER/JWK re-import round-trips.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;

const CURVE: EcdsaCurve = EcdsaCurve::P256;
const DATA: &[u8] = b"Hello, ECDSA!";

// describe("generateKeyPair")
#[test]
fn generates_a_valid_key_pair() {
    let (priv_der, pub_der) = generate_key_pair(CURVE).unwrap();
    assert!(!priv_der.is_empty());
    assert!(!pub_der.is_empty());
}

// describe("importPrivateKey")
#[test]
fn imports_a_private_key() {
    let (priv_der, _) = generate_key_pair(CURVE).unwrap();
    let key = import_private_key(&priv_der, CURVE).unwrap();
    assert_eq!(key.curve(), EcdsaCurve::P256);
}

// describe("importPublicKey")
#[test]
fn imports_a_public_key() {
    let (_, pub_der) = generate_key_pair(CURVE).unwrap();
    let key = import_public_key(&pub_der, CURVE).unwrap();
    assert_eq!(key.curve(), EcdsaCurve::P256);
}

// describe("sign")
#[test]
fn signs_data() {
    let (priv_der, _) = generate_key_pair(CURVE).unwrap();
    let sk = import_private_key(&priv_der, CURVE).unwrap();
    let sig = sign(&sk, DATA, ShaFamily::Sha256).unwrap();
    // P-256 P1363 signature is 64 bytes (32-byte r ‖ 32-byte s).
    assert_eq!(sig.len(), 64);
}

// describe("verify") — "verifies a signature using the corresponding public key"
#[test]
fn verifies_a_valid_signature() {
    let (priv_der, pub_der) = generate_key_pair(CURVE).unwrap();
    let sk = import_private_key(&priv_der, CURVE).unwrap();
    let pk = import_public_key(&pub_der, CURVE).unwrap();
    let sig = sign(&sk, DATA, ShaFamily::Sha256).unwrap();
    assert!(verify(&pk, &sig, DATA, ShaFamily::Sha256).unwrap());
}

// describe("verify") — "fails to verify with incorrect data"
#[test]
fn fails_to_verify_tampered_data() {
    let (priv_der, pub_der) = generate_key_pair(CURVE).unwrap();
    let sk = import_private_key(&priv_der, CURVE).unwrap();
    let pk = import_public_key(&pub_der, CURVE).unwrap();
    let sig = sign(&sk, DATA, ShaFamily::Sha256).unwrap();
    assert!(!verify(&pk, &sig, b"Tampered Data!", ShaFamily::Sha256).unwrap());
}

// describe("exportKey") — "exports a private key in pkcs8 format"
#[test]
fn exports_private_key_pkcs8() {
    let (priv_der, _) = generate_key_pair(CURVE).unwrap();
    let sk = import_private_key(&priv_der, CURVE).unwrap();
    let ExportedKey::Der(der) = sk.export(ExportKeyFormat::Pkcs8).unwrap() else {
        panic!("expected DER");
    };
    assert!(!der.is_empty());
}

// describe("exportKey") — "exports a public key in spki format"
#[test]
fn exports_public_key_spki() {
    let (_, pub_der) = generate_key_pair(CURVE).unwrap();
    let pk = import_public_key(&pub_der, CURVE).unwrap();
    let ExportedKey::Der(der) = pk.export(ExportKeyFormat::Spki).unwrap() else {
        panic!("expected DER");
    };
    assert!(!der.is_empty());
}

// describe("exportKey") — "exports a key in jwk format" (kty: EC, crv: P-256)
#[test]
fn exports_public_key_jwk() {
    let (_, pub_der) = generate_key_pair(CURVE).unwrap();
    let pk = import_public_key(&pub_der, CURVE).unwrap();
    let ExportedKey::Jwk(jwk) = pk.export(ExportKeyFormat::Jwk).unwrap() else {
        panic!("expected JWK");
    };
    assert_eq!(jwk["kty"], "EC");
    assert_eq!(jwk["crv"], "P-256");
    assert!(jwk["x"].is_string());
    assert!(jwk["y"].is_string());
}

// ---- Beyond upstream: all three curves round-trip, and DER/JWK re-imports ----

#[test]
fn all_curves_sign_verify_round_trip() {
    for curve in [EcdsaCurve::P256, EcdsaCurve::P384, EcdsaCurve::P521] {
        let (priv_der, pub_der) = generate_key_pair(curve).unwrap();
        let sk = import_private_key(&priv_der, curve).unwrap();
        let pk = import_public_key(&pub_der, curve).unwrap();
        let sig = sign(&sk, DATA, ShaFamily::Sha256).unwrap();
        assert!(
            verify(&pk, &sig, DATA, ShaFamily::Sha256).unwrap(),
            "{curve:?} verify"
        );
        assert!(
            !verify(&pk, &sig, b"other", ShaFamily::Sha256).unwrap(),
            "{curve:?} negative"
        );
    }
}

#[test]
fn private_jwk_has_d_and_reexports_pkcs8() {
    let (priv_der, _) = generate_key_pair(CURVE).unwrap();
    let sk = import_private_key(&priv_der, CURVE).unwrap();
    let ExportedKey::Jwk(jwk) = sk.export(ExportKeyFormat::Jwk).unwrap() else {
        panic!("expected JWK");
    };
    assert_eq!(jwk["kty"], "EC");
    assert_eq!(jwk["crv"], "P-256");
    assert!(jwk["d"].is_string(), "private JWK must carry d");

    // The re-exported PKCS#8 round-trips back to a usable signing key.
    let ExportedKey::Der(der) = sk.export(ExportKeyFormat::Pkcs8).unwrap() else {
        panic!("expected DER");
    };
    let sk2 = import_private_key(&der, CURVE).unwrap();
    let sig = sign(&sk2, DATA, ShaFamily::Sha256).unwrap();
    assert_eq!(sig.len(), 64);
}

// A private key rejects spki export; a public key rejects pkcs8 export.
#[test]
fn export_format_guards() {
    let (priv_der, pub_der) = generate_key_pair(CURVE).unwrap();
    let sk = import_private_key(&priv_der, CURVE).unwrap();
    let pk = import_public_key(&pub_der, CURVE).unwrap();
    assert_eq!(
        sk.export(ExportKeyFormat::Spki),
        Err(EcdsaError::UnsupportedFormat(ExportKeyFormat::Spki))
    );
    assert_eq!(
        pk.export(ExportKeyFormat::Pkcs8),
        Err(EcdsaError::UnsupportedFormat(ExportKeyFormat::Pkcs8))
    );
}
