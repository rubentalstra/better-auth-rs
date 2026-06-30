//! Behavior tests for the `Account` record (notably its nullable date fields).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;

#[test]
fn optional_dates_and_camel_case() {
    let a: Account = serde_json::from_value(serde_json::json!({
        "id": "a1",
        "createdAt": "2024-01-02T03:04:05Z",
        "updatedAt": "2024-01-02T03:04:05Z",
        "providerId": "google",
        "accountId": "acc",
        "userId": "u1",
        "accessTokenExpiresAt": "2024-05-06T07:08:09Z"
        // refreshTokenExpiresAt / accessToken / ... omitted -> None
    }))
    .unwrap();
    assert_eq!(a.provider_id, "google");
    assert_eq!(a.account_id, "acc");
    assert!(a.access_token_expires_at.is_some());
    assert!(a.refresh_token_expires_at.is_none());
    assert!(a.access_token.is_none());
    assert!(a.password.is_none());

    let v = serde_json::to_value(&a).unwrap();
    assert!(v.get("accessTokenExpiresAt").is_some());
    assert!(v.get("refreshTokenExpiresAt").is_none()); // None omitted
    assert!(v.get("accessToken").is_none());
    assert!(v.get("provider_id").is_none());

    let back: Account = serde_json::from_value(v).unwrap();
    assert_eq!(a, back);
}
