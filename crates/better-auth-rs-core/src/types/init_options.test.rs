//! Behavior tests for the init-options sub-types.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use crate::db::schema::RateLimit;

#[test]
fn rate_limit_rule_defaults() {
    let r = BetterAuthRateLimitRule::default();
    assert_eq!(r.window, 10);
    assert_eq!(r.max, 100);
}

#[test]
fn enum_defaults() {
    assert!(matches!(
        GenerateIdConfig::default(),
        GenerateIdConfig::Default
    ));
    assert_eq!(BaseUrlProtocol::default(), BaseUrlProtocol::Auto);
    assert!(matches!(
        StoreIdentifierOption::default(),
        StoreIdentifierOption::Plain
    ));
    assert_eq!(
        RateLimitStorageKind::default(),
        RateLimitStorageKind::Memory
    );
}

#[test]
fn generate_id_fn_returns_value_or_none() {
    let f: GenerateIdFn = Arc::new(|input| Some(format!("{}-id", input.model)));
    assert_eq!(
        f(GenerateIdInput {
            model: "user".to_owned(),
            size: None
        }),
        Some("user-id".to_owned())
    );
    // `false` (let the DB generate) -> None
    let db_gen: GenerateIdFn = Arc::new(|_| None);
    assert_eq!(
        db_gen(GenerateIdInput {
            model: "x".to_owned(),
            size: Some(8)
        }),
        None
    );
}

struct MemRl;

#[async_trait::async_trait]
impl BetterAuthRateLimitStorage for MemRl {
    async fn get(&self, _key: &str) -> Result<Option<RateLimit>, RateLimitStorageError> {
        Ok(None)
    }
    async fn set(
        &self,
        _key: &str,
        _value: RateLimit,
        _update: bool,
    ) -> Result<(), RateLimitStorageError> {
        Ok(())
    }
}

#[tokio::test]
async fn rate_limit_storage_default_consume_is_none() {
    let storage = MemRl;
    assert!(
        storage
            .consume("k", BetterAuthRateLimitRule::default())
            .await
            .unwrap()
            .is_none()
    );
}
