//! Behavior tests for the `RateLimit` record.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;

#[test]
fn integer_fields_and_camel_case() {
    let r: RateLimit = serde_json::from_value(serde_json::json!({
        "key": "ip:1.2.3.4",
        "count": 5,
        "lastRequest": 1_704_164_645_000_i64
    }))
    .unwrap();
    assert_eq!(r.count, 5);
    assert_eq!(r.last_request, 1_704_164_645_000);

    let v = serde_json::to_value(&r).unwrap();
    assert_eq!(v["key"], "ip:1.2.3.4");
    assert_eq!(v["count"], 5);
    assert_eq!(v["lastRequest"], 1_704_164_645_000_i64);
    assert!(v.get("last_request").is_none());

    let back: RateLimit = serde_json::from_value(v).unwrap();
    assert_eq!(r, back);
}
