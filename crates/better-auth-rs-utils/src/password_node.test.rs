//! Port of `password.node.test.ts` (the `node:crypto` scrypt suite + the noble↔node
//! cross-compatibility cases).
//!
//! In Rust `password_node` re-exports [`crate::password`], so "node" and "noble" are the same
//! implementation and the cross-compat cases hold by construction — we still exercise both module
//! paths to prove the produced hashes are mutually verifiable. The upstream `rejects.toThrow`
//! invalid-hash case becomes an `Err(PasswordError::InvalidHash)` assertion (Rust returns a typed
//! error instead of throwing).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::{hash_password, verify_password};

// it("hashPassword produces salt:hex format")
#[test]
fn produces_salt_colon_hex_format() {
    let hash = hash_password("mypassword").unwrap();
    let parts: Vec<&str> = hash.split(':').collect();
    assert_eq!(parts.len(), 2);
    // 16-byte salt → 32 hex chars; 64-byte key → 128 hex chars.
    assert_eq!(parts[0].len(), 32);
    assert!(parts[0].bytes().all(|b| b.is_ascii_hexdigit()));
    assert_eq!(parts[1].len(), 128);
    assert!(parts[1].bytes().all(|b| b.is_ascii_hexdigit()));
}

// it("verifyPassword returns true for correct password")
#[test]
fn verifies_correct_password() {
    let hash = hash_password("correcthorsebatterystaple").unwrap();
    assert!(verify_password(&hash, "correcthorsebatterystaple").unwrap());
}

// it("verifyPassword returns false for wrong password")
#[test]
fn rejects_wrong_password() {
    let hash = hash_password("correcthorsebatterystaple").unwrap();
    assert!(!verify_password(&hash, "wrongpassword").unwrap());
}

// it("throws on invalid hash format") — Rust returns a typed error rather than throwing.
#[test]
fn errors_on_invalid_hash_format() {
    assert_eq!(
        verify_password("invalidhash", "password"),
        Err(crate::password::PasswordError::InvalidHash)
    );
}

// it("each call produces a unique hash")
#[test]
fn each_call_is_unique() {
    let a = hash_password("samepassword").unwrap();
    let b = hash_password("samepassword").unwrap();
    assert_ne!(a, b);
}

// it("handles empty password")
#[test]
fn handles_empty_password() {
    let hash = hash_password("").unwrap();
    assert!(verify_password(&hash, "").unwrap());
    assert!(!verify_password(&hash, "notempty").unwrap());
}

// it("handles very long password")
#[test]
fn handles_very_long_password() {
    let long = "a".repeat(1000);
    let hash = hash_password(&long).unwrap();
    assert!(verify_password(&hash, &long).unwrap());
    assert!(!verify_password(&hash, &"a".repeat(999)).unwrap());
}

// it("normalizes unicode passwords (NFKC)") — ﬁ (U+FB01) normalizes to "fi".
#[test]
fn normalizes_unicode_nfkc() {
    let hash = hash_password("\u{FB01}").unwrap();
    assert!(verify_password(&hash, "fi").unwrap());
}

// it("returns false for tampered key in hash")
#[test]
fn rejects_tampered_key() {
    let hash = hash_password("password").unwrap();
    let (salt, key) = hash.split_once(':').unwrap();
    let tampered = format!("{salt}:{}", "0".repeat(key.len()));
    assert!(!verify_password(&tampered, "password").unwrap());
}

// it("returns false for tampered salt in hash")
#[test]
fn rejects_tampered_salt() {
    let hash = hash_password("password").unwrap();
    let (_, key) = hash.split_once(':').unwrap();
    let tampered = format!("{}:{key}", "0".repeat(32));
    assert!(!verify_password(&tampered, "password").unwrap());
}

// it("handles special characters in password")
#[test]
fn handles_special_characters() {
    let special = "p@$$w0rd!#%^&*()";
    let hash = hash_password(special).unwrap();
    assert!(verify_password(&hash, special).unwrap());
    assert!(!verify_password(&hash, "p@$$w0rd").unwrap());
}

// describe("cross-compatibility: noble and node:crypto produce identical keys")
// In Rust both paths are the same impl; assert hashes are mutually verifiable across module paths.
#[test]
fn cross_compat_noble_hash_verified_by_node() {
    // "noble" hash (crate::password) verified by "node" (super == password_node re-export).
    let hash = crate::password::hash_password("crossplatformtest").unwrap();
    assert!(verify_password(&hash, "crossplatformtest").unwrap());
}

#[test]
fn cross_compat_node_hash_verified_by_noble() {
    // "node" hash (super) verified by "noble" (crate::password).
    let hash = hash_password("crossplatformtest").unwrap();
    assert!(crate::password::verify_password(&hash, "crossplatformtest").unwrap());
}
