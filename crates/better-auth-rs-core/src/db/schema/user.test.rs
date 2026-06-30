//! Behavior tests for the `User` record.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;

#[test]
fn new_lowercases_email_and_applies_defaults() {
    let u = User::new("u1", "Foo@Example.COM", "Foo Bar");
    assert_eq!(u.email, "foo@example.com");
    assert!(!u.email_verified);
    assert_eq!(u.core.id, "u1");
    assert_eq!(u.name, "Foo Bar");
    assert!(u.image.is_none());
    assert_eq!(u.core.created_at, u.core.updated_at);
}

#[test]
fn serializes_to_camel_case_and_omits_none_image() {
    let u = User::new("u1", "a@b.com", "Name");
    let v = serde_json::to_value(&u).unwrap();
    // camelCase keys, no snake_case leakage.
    assert_eq!(v["id"], "u1");
    assert_eq!(v["email"], "a@b.com");
    assert_eq!(v["emailVerified"], false);
    assert_eq!(v["name"], "Name");
    assert!(v.get("createdAt").is_some());
    assert!(v.get("updatedAt").is_some());
    assert!(v.get("email_verified").is_none());
    // `image` is None -> omitted.
    assert!(v.get("image").is_none());
}

#[test]
fn deserializes_camel_case_with_default_email_verified() {
    // `emailVerified` missing -> defaults to false; `image` present.
    let json = serde_json::json!({
        "id": "u1",
        "createdAt": "2024-01-02T03:04:05Z",
        "updatedAt": "2024-01-02T03:04:05Z",
        "email": "a@b.com",
        "name": "N",
        "image": "https://x/y.png"
    });
    let u: User = serde_json::from_value(json).unwrap();
    assert!(!u.email_verified);
    assert_eq!(u.image.as_deref(), Some("https://x/y.png"));
    assert_eq!(u.core.id, "u1");
}

#[test]
fn round_trips() {
    let u = User::new("u1", "a@b.com", "N");
    let back: User = serde_json::from_value(serde_json::to_value(&u).unwrap()).unwrap();
    assert_eq!(u, back);
}
