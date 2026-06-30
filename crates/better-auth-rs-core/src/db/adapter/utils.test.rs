//! Behavior tests for `with_apply_default` and `deepmerge`.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::Arc;

use serde_json::json;

use super::*;
use crate::db::types::{DbFieldAttribute, DbFieldType, DbValue, DefaultValue};

#[test]
fn create_applies_default_value_when_missing() {
    let mut f = DbFieldAttribute::new(DbFieldType::String);
    f.config.default_value = Some(DefaultValue::Static(DbValue::from("d")));
    // undefined (None) -> default applied
    assert_eq!(
        with_apply_default(None, &f, ApplyAction::Create),
        Some(DbValue::from("d"))
    );
    // a provided value is kept
    assert_eq!(
        with_apply_default(Some(DbValue::from("x")), &f, ApplyAction::Create),
        Some(DbValue::from("x"))
    );
}

#[test]
fn create_applies_default_for_null_only_when_strictly_required() {
    let mut f = DbFieldAttribute::new(DbFieldType::String);
    f.config.default_value = Some(DefaultValue::Static(DbValue::from("d")));
    // null + not strictly required -> null kept (no default)
    assert_eq!(
        with_apply_default(Some(DbValue::Null), &f, ApplyAction::Create),
        Some(DbValue::Null)
    );
    // null + strictly required -> default applied
    f.config.required = Some(true);
    assert_eq!(
        with_apply_default(Some(DbValue::Null), &f, ApplyAction::Create),
        Some(DbValue::from("d"))
    );
}

#[test]
fn update_applies_on_update_when_missing() {
    let mut f = DbFieldAttribute::new(DbFieldType::Date);
    let on_update: Arc<dyn Fn() -> DbValue + Send + Sync> = Arc::new(|| DbValue::from("updated"));
    f.config.on_update = Some(on_update);
    assert_eq!(
        with_apply_default(None, &f, ApplyAction::Update),
        Some(DbValue::from("updated"))
    );
    // a provided value is kept
    assert_eq!(
        with_apply_default(Some(DbValue::from("x")), &f, ApplyAction::Update),
        Some(DbValue::from("x"))
    );
    // find actions never change the value
    assert_eq!(with_apply_default(None, &f, ApplyAction::FindOne), None);
}

#[test]
fn deepmerge_concatenates_arrays_merges_objects_overrides_primitives() {
    // arrays concatenate
    assert_eq!(deepmerge(json!([1, 2]), json!([3])), json!([1, 2, 3]));
    // objects merge recursively
    assert_eq!(
        deepmerge(
            json!({"a": {"x": 1}, "b": 2}),
            json!({"a": {"y": 3}, "c": 4})
        ),
        json!({"a": {"x": 1, "y": 3}, "b": 2, "c": 4})
    );
    // primitives: source overrides
    assert_eq!(deepmerge(json!(1), json!("two")), json!("two"));
}
