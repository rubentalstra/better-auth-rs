//! Behavioral port of `rsa.test.ts`.
//!
//! Upstream's suite is entirely `vi.spyOn(crypto.subtle, ...)` mocks — it checks that the wrapper
//! forwards to Web Crypto with the right parameters (`RSA-OAEP`, `RSA-PSS` salt 32, `modulusLength`
//! 2048, exponent 65537, SHA-256) and never runs real crypto. There is no `subtle` to mock in Rust,
//! so each upstream case becomes the real round-trip it was approximating. One 2048-bit key pair is
//! generated and shared (keygen is expensive).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::OnceLock;

use super::*;

fn kp() -> &'static RsaKeyPair {
    static KP: OnceLock<RsaKeyPair> = OnceLock::new();
    KP.get_or_init(|| generate_key_pair(2048, ShaFamily::Sha256).unwrap())
}

// describe("generateKeyPair") — a 2048-bit RSA key pair is produced.
#[test]
fn generates_a_key_pair() {
    let ExportedKey::Der(spki) = kp().public.export(ExportKeyFormat::Spki).unwrap() else {
        panic!("expected DER");
    };
    assert!(!spki.is_empty());
}

// describe("encrypt") + describe("decrypt") — RSA-OAEP round-trips.
#[test]
fn oaep_encrypt_decrypt_round_trip() {
    let data = b"test data";
    let ct = kp().public.encrypt(data).unwrap();
    assert_ne!(ct.as_slice(), data, "ciphertext must differ from plaintext");
    let pt = kp().private.decrypt(&ct).unwrap();
    assert_eq!(pt, data);
}

// describe("sign") + describe("verify") — RSA-PSS round-trips; tampered data fails.
#[test]
fn pss_sign_verify_round_trip() {
    let data = b"test data";
    let sig = kp().private.sign(data).unwrap();
    assert!(kp().public.verify(&sig, data));
    assert!(!kp().public.verify(&sig, b"tampered data"));
}

// PSS is randomized: two signatures differ but both verify.
#[test]
fn pss_is_randomized() {
    let data = b"same message";
    let a = kp().private.sign(data).unwrap();
    let b = kp().private.sign(data).unwrap();
    assert_ne!(a, b);
    assert!(kp().public.verify(&a, data));
    assert!(kp().public.verify(&b, data));
}

// describe("exportKey") — pkcs8 (private) round-trips back to a working decrypt key.
#[test]
fn export_import_pkcs8_round_trip() {
    let ExportedKey::Der(der) = kp().private.export(ExportKeyFormat::Pkcs8).unwrap() else {
        panic!("expected DER");
    };
    let imported = import_private_pkcs8(&der).unwrap();
    let ct = kp().public.encrypt(b"roundtrip").unwrap();
    assert_eq!(imported.decrypt(&ct).unwrap(), b"roundtrip");
}

// spki (public) round-trips back to a working encrypt key.
#[test]
fn export_import_spki_round_trip() {
    let ExportedKey::Der(der) = kp().public.export(ExportKeyFormat::Spki).unwrap() else {
        panic!("expected DER");
    };
    let imported = import_public_spki(&der).unwrap();
    let ct = imported.encrypt(b"via spki").unwrap();
    assert_eq!(kp().private.decrypt(&ct).unwrap(), b"via spki");
}

// describe("importKey") — public JWK (`kty: RSA`, `n`, `e`) exports and re-imports to a usable key.
#[test]
fn public_jwk_export_import_round_trip() {
    let ExportedKey::Jwk(jwk) = kp().public.export(ExportKeyFormat::Jwk).unwrap() else {
        panic!("expected JWK");
    };
    assert_eq!(jwk["kty"], "RSA");
    assert!(jwk["n"].is_string());
    assert!(jwk["e"].is_string());

    let imported = import_public_jwk(&jwk).unwrap();
    let ct = imported.encrypt(b"via jwk").unwrap();
    assert_eq!(kp().private.decrypt(&ct).unwrap(), b"via jwk");
}

// A private JWK carries the private material (`kty`, `n`, `e`, `d`).
#[test]
fn private_jwk_has_components() {
    let ExportedKey::Jwk(jwk) = kp().private.export(ExportKeyFormat::Jwk).unwrap() else {
        panic!("expected JWK");
    };
    assert_eq!(jwk["kty"], "RSA");
    for field in ["n", "e", "d", "p", "q"] {
        assert!(jwk[field].is_string(), "private JWK missing `{field}`");
    }
}

// A public key rejects pkcs8 export; a private key rejects spki export.
#[test]
fn export_format_guards() {
    assert_eq!(
        kp().public.export(ExportKeyFormat::Pkcs8),
        Err(RsaError::UnsupportedFormat(ExportKeyFormat::Pkcs8))
    );
    assert_eq!(
        kp().private.export(ExportKeyFormat::Spki),
        Err(RsaError::UnsupportedFormat(ExportKeyFormat::Spki))
    );
}
