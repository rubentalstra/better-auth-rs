//! Behavior tests for the init-options sub-types.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use crate::db::schema::{RateLimit, User};

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

#[test]
fn options_default_is_empty() {
    let o = BetterAuthOptions::default();
    assert!(o.plugins.is_empty());
    assert!(o.social_providers.is_empty());
    assert!(o.secret.is_none());
    assert!(o.email_and_password.is_none());
    assert!(o.database_hooks.is_none());
}

#[test]
fn options_debug_redacts_secrets() {
    let o = BetterAuthOptions {
        secret: Some("top-secret-value".to_owned()),
        secrets: Some(vec![SecretEntry {
            version: 1,
            value: "rotation-key".to_owned(),
        }]),
        ..BetterAuthOptions::default()
    };
    let rendered = format!("{o:?}");
    assert!(!rendered.contains("top-secret-value"));
    assert!(!rendered.contains("rotation-key"));
    assert!(rendered.contains("<redacted>"));
}

#[tokio::test]
async fn send_email_callback_can_be_built_and_invoked() {
    let called = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let flag = called.clone();
    let cb: SendUserEmailFn = Arc::new(move |data: UserUrlToken, _ctx| {
        let flag = flag.clone();
        Box::pin(async move {
            assert_eq!(data.url, "https://verify.example");
            flag.store(true, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        })
    });

    let data = UserUrlToken {
        user: User::new("u1", "e@x.com", "Tester"),
        url: "https://verify.example".to_owned(),
        token: "tok".to_owned(),
    };
    // No request context (e.g. during a server-side `auth.api` call).
    cb(data, None).await.unwrap();
    assert!(called.load(std::sync::atomic::Ordering::SeqCst));
}

#[test]
fn database_hooks_default_is_all_none() {
    let h = DatabaseHooks::default();
    assert!(h.user.is_none());
    assert!(h.session.is_none());
    assert!(h.account.is_none());
    assert!(h.verification.is_none());
}

#[test]
fn custom_rate_limit_rule_distinguishes_rule_from_exemption() {
    let mut rules = std::collections::BTreeMap::new();
    rules.insert(
        "/sign-in".to_owned(),
        CustomRateLimitRule::Rule(BetterAuthRateLimitRule { window: 60, max: 5 }),
    );
    rules.insert("/get-session".to_owned(), CustomRateLimitRule::Disabled);
    let opts = BetterAuthRateLimitOptions {
        custom_rules: Some(rules),
        ..BetterAuthRateLimitOptions::default()
    };
    let cr = opts.custom_rules.unwrap();
    // `false` (exempt) is distinct from a rule...
    assert!(matches!(
        cr.get("/get-session"),
        Some(CustomRateLimitRule::Disabled)
    ));
    assert!(matches!(
        cr.get("/sign-in"),
        Some(CustomRateLimitRule::Rule(r)) if r.max == 5 && r.window == 60
    ));
    // ...and both are distinct from "no entry" (which falls back to the default rule).
    assert!(!cr.contains_key("/other"));
}
