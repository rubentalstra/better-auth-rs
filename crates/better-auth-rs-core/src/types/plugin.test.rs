//! Behavior tests for the `BetterAuthPlugin` trait surface.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeMap;
use std::sync::Arc;

use super::*;
use crate::error::RawError;

/// A plugin implementing only the required `id` — everything else uses the trait defaults.
struct Minimal;

impl BetterAuthPlugin for Minimal {
    fn id(&self) -> &str {
        "minimal"
    }
}

/// A plugin overriding a few capabilities.
struct Rich;

impl BetterAuthPlugin for Rich {
    fn id(&self) -> &str {
        "rich"
    }
    fn version(&self) -> Option<&str> {
        Some("1.2.3")
    }
    fn error_codes(&self) -> BTreeMap<&'static str, RawError> {
        let mut m = BTreeMap::new();
        m.insert(
            "OOPS",
            RawError {
                code: "OOPS",
                message: "Something went wrong",
            },
        );
        m
    }
}

#[test]
fn minimal_plugin_uses_defaults() {
    let p = Minimal;
    assert_eq!(p.id(), "minimal");
    assert!(p.version().is_none());
    assert!(p.endpoints().is_empty());
    assert!(p.middlewares().is_empty());
    assert!(p.hooks_before().is_empty());
    assert!(p.hooks_after().is_empty());
    assert!(p.schema().is_none());
    assert!(p.rate_limit().is_empty());
    assert!(p.error_codes().is_empty());
}

#[test]
fn rich_plugin_overrides_take_effect() {
    let p = Rich;
    assert_eq!(p.id(), "rich");
    assert_eq!(p.version(), Some("1.2.3"));
    let codes = p.error_codes();
    assert_eq!(
        codes.get("OOPS").map(|e| e.message),
        Some("Something went wrong")
    );
}

#[test]
fn plugins_are_usable_as_trait_objects() {
    let plugins: Vec<Arc<dyn BetterAuthPlugin>> = vec![Arc::new(Minimal), Arc::new(Rich)];
    let ids: Vec<&str> = plugins.iter().map(|p| p.id()).collect();
    assert_eq!(ids, vec!["minimal", "rich"]);
}

#[test]
fn init_result_default_is_empty() {
    let r = InitResult::default();
    assert!(r.context.is_none());
    assert!(r.options.is_none());
}
