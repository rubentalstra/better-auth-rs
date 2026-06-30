//! Upstream reference: db/adapter/utils.ts
//!
//! Adapter helpers: [`with_apply_default`] (apply a field's `defaultValue`/`onUpdate` when a value
//! is missing) and [`deepmerge`] (recursive merge of two JSON values). `value` is modeled as
//! `Option<DbValue>`: `None` is JS `undefined`, `Some(DbValue::Null)` is JS `null`.

use serde_json::Value;

use crate::db::types::{DbFieldAttribute, DbValue};

/// The adapter action driving [`with_apply_default`] (only `Create`/`Update` change the value).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplyAction {
    /// Inserting a new record.
    Create,
    /// Updating an existing record.
    Update,
    /// Reading a single record.
    FindOne,
    /// Reading many records.
    FindMany,
}

/// Apply a field's `onUpdate` (on update) or `defaultValue` (on create) when the incoming value is
/// missing, mirroring upstream `withApplyDefault`.
#[must_use]
pub fn with_apply_default(
    value: Option<DbValue>,
    field: &DbFieldAttribute,
    action: ApplyAction,
) -> Option<DbValue> {
    match action {
        ApplyAction::Update => {
            // Apply `onUpdate` only when the value is `undefined`.
            if value.is_none()
                && let Some(on_update) = &field.config.on_update
            {
                return Some(on_update());
            }
            value
        }
        ApplyAction::Create => {
            // Don't apply a default when the value is `null` but the field isn't strictly required.
            // (Upstream checks `required === true` literally, not the defaulted-true.)
            let required_strict = field.config.required == Some(true);
            let is_undefined = value.is_none();
            let is_null = matches!(value, Some(DbValue::Null));
            if (is_undefined || (required_strict && is_null))
                && let Some(default) = &field.config.default_value
            {
                return Some(default.resolve());
            }
            value
        }
        ApplyAction::FindOne | ApplyAction::FindMany => value,
    }
}

/// Recursively merge `source` into `target`: arrays concatenate, objects merge key-by-key
/// (recursing on shared keys), and any other value is overridden by `source`. (JS `undefined`
/// skipping has no JSON analog — absent keys simply aren't present in the source object.)
#[must_use]
pub fn deepmerge(target: Value, source: Value) -> Value {
    match (target, source) {
        (Value::Array(mut t), Value::Array(s)) => {
            t.extend(s);
            Value::Array(t)
        }
        (Value::Object(mut t), Value::Object(s)) => {
            for (key, value) in s {
                match t.remove(&key) {
                    Some(existing) => {
                        t.insert(key, deepmerge(existing, value));
                    }
                    None => {
                        t.insert(key, value);
                    }
                }
            }
            Value::Object(t)
        }
        (_, source) => source,
    }
}

#[cfg(test)]
#[path = "utils.test.rs"]
mod utils_tests;
