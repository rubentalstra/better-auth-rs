//! Behavior tests for the `Session` record.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;

fn sample() -> serde_json::Value {
    serde_json::json!({
        "id": "s1",
        "createdAt": "2024-01-02T03:04:05Z",
        "updatedAt": "2024-01-02T03:04:05Z",
        "userId": "u1",
        "expiresAt": "2024-02-03T04:05:06Z",
        "token": "tok"
        // ipAddress / userAgent omitted -> None
    })
}

#[test]
fn deserializes_camel_case_and_handles_nullish() {
    let s: Session = serde_json::from_value(sample()).unwrap();
    assert_eq!(s.user_id, "u1");
    assert_eq!(s.token, "tok");
    assert!(s.ip_address.is_none());
    assert!(s.user_agent.is_none());
    assert_eq!(s.core.id, "s1");
}

#[test]
fn serializes_camel_case_omits_none_and_round_trips() {
    let s: Session = serde_json::from_value(sample()).unwrap();
    let v = serde_json::to_value(&s).unwrap();
    assert_eq!(v["userId"], "u1");
    assert!(v.get("expiresAt").is_some());
    assert!(v.get("user_id").is_none());
    assert!(v.get("ipAddress").is_none());
    let back: Session = serde_json::from_value(v).unwrap();
    assert_eq!(s, back);
}
