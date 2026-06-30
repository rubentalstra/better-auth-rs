//! Behavior tests for the `Verification` record.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;

#[test]
fn round_trips_camel_case() {
    let vf: Verification = serde_json::from_value(serde_json::json!({
        "id": "v1",
        "createdAt": "2024-01-02T03:04:05Z",
        "updatedAt": "2024-01-02T03:04:05Z",
        "value": "code-123",
        "expiresAt": "2024-02-03T04:05:06Z",
        "identifier": "a@b.com"
    }))
    .unwrap();
    assert_eq!(vf.value, "code-123");
    assert_eq!(vf.identifier, "a@b.com");
    assert_eq!(vf.core.id, "v1");

    let v = serde_json::to_value(&vf).unwrap();
    assert!(v.get("expiresAt").is_some());
    let back: Verification = serde_json::from_value(v).unwrap();
    assert_eq!(vf, back);
}
