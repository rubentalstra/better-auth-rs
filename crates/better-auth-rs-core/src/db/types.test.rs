//! Behavior tests for the db field/value vocabulary and the `SecondaryStorage` default methods.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use super::*;

#[test]
fn db_value_from_and_accessors() {
    assert_eq!(DbValue::from("hi").as_str(), Some("hi"));
    assert_eq!(DbValue::from(String::from("hey")).as_str(), Some("hey"));
    assert_eq!(DbValue::from(7_i64).as_i64(), Some(7));
    assert_eq!(DbValue::from(true).as_bool(), Some(true));
    assert_eq!(DbValue::from(1.5_f64).as_f64(), Some(1.5));
    assert!(DbValue::from(1.5_f64).as_i64().is_none());
    assert!(DbValue::Null.is_null());
    // Option<T> -> Null / value
    assert!(DbValue::from(None::<String>).is_null());
    assert_eq!(DbValue::from(Some("x")).as_str(), Some("x"));
}

#[test]
fn default_value_resolves_static_and_generator() {
    let s = DefaultValue::Static(DbValue::from("fixed"));
    assert_eq!(s.resolve().as_str(), Some("fixed"));

    let g = DefaultValue::Generator(Arc::new(|| DbValue::from("gen")));
    assert_eq!(g.resolve().as_str(), Some("gen"));
}

#[test]
fn field_attribute_new_has_empty_config() {
    let f = DbFieldAttribute::new(DbFieldType::String);
    assert_eq!(f.field_type, DbFieldType::String);
    assert!(f.config.required.is_none());
    assert!(f.config.default_value.is_none());
    assert!(f.config.references.is_none());
}

#[test]
fn enum_field_type_carries_values() {
    let t = DbFieldType::Enum(vec!["a".to_owned(), "b".to_owned()]);
    assert_eq!(t, DbFieldType::Enum(vec!["a".to_owned(), "b".to_owned()]));
    assert_ne!(t, DbFieldType::String);
}

#[test]
fn referential_action_defaults_to_cascade() {
    assert_eq!(ReferentialAction::default(), ReferentialAction::Cascade);
}

#[test]
fn base_model_names() {
    assert_eq!(BaseModel::User.as_str(), "user");
    assert_eq!(
        BaseModel::ALL.map(BaseModel::as_str),
        ["user", "account", "session", "verification"]
    );
}

// A minimal in-memory SecondaryStorage to exercise the default trait methods.
#[derive(Default)]
struct MemStore {
    map: Mutex<HashMap<String, String>>,
}

#[async_trait::async_trait]
impl SecondaryStorage for MemStore {
    async fn get(&self, key: &str) -> Result<Option<String>, SecondaryStorageError> {
        Ok(self.map.lock().unwrap().get(key).cloned())
    }
    async fn set(
        &self,
        key: &str,
        value: &str,
        _ttl: Option<i64>,
    ) -> Result<(), SecondaryStorageError> {
        self.map
            .lock()
            .unwrap()
            .insert(key.to_owned(), value.to_owned());
        Ok(())
    }
    async fn delete(&self, key: &str) -> Result<(), SecondaryStorageError> {
        self.map.lock().unwrap().remove(key);
        Ok(())
    }
}

#[tokio::test]
async fn get_and_delete_default_reads_then_deletes() {
    let store = MemStore::default();
    store.set("k", "v", None).await.unwrap();
    assert_eq!(
        store.get_and_delete("k").await.unwrap(),
        Some("v".to_owned())
    );
    assert_eq!(store.get("k").await.unwrap(), None); // gone
    assert_eq!(store.get_and_delete("missing").await.unwrap(), None); // absent -> None
}

#[tokio::test]
async fn increment_default_is_unsupported() {
    let store = MemStore::default();
    assert_eq!(store.increment("k", 60).await.unwrap(), None);
}
