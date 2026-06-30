//! Behavior tests for the schema name-resolution helpers (model/field, default/db, plural).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeMap;

use super::*;
use crate::db::adapter::{get_default_field_name, get_field_name, get_model_name};
use crate::db::types::{BetterAuthDbSchema, DbFieldAttribute, DbFieldType, DbTableSchema};

fn schema() -> BetterAuthDbSchema {
    let mut fields = BTreeMap::new();
    fields.insert(
        "email".to_owned(),
        DbFieldAttribute::new(DbFieldType::String),
    );
    let mut display = DbFieldAttribute::new(DbFieldType::String);
    display.config.field_name = Some("display_name".to_owned());
    fields.insert("displayName".to_owned(), display);

    let mut s: BetterAuthDbSchema = BTreeMap::new();
    s.insert(
        "user".to_owned(),
        DbTableSchema {
            model_name: "app_user".to_owned(), // a customized model name
            fields,
            disable_migrations: None,
            order: None,
        },
    );
    s
}

#[test]
fn default_model_name_by_key_custom_and_plural() {
    let s = schema();
    assert_eq!(get_default_model_name(&s, false, "user").unwrap(), "user");
    // custom modelName resolves back to the schema key
    assert_eq!(
        get_default_model_name(&s, false, "app_user").unwrap(),
        "user"
    );
    // usePlural strips the trailing 's'
    assert_eq!(get_default_model_name(&s, true, "users").unwrap(), "user");
    assert!(get_default_model_name(&s, false, "nope").is_err());
}

#[test]
fn default_field_name_id_key_and_field_name_override() {
    let s = schema();
    assert_eq!(
        get_default_field_name(&s, false, "user", "id").unwrap(),
        "id"
    );
    assert_eq!(
        get_default_field_name(&s, false, "user", "_id").unwrap(),
        "id"
    );
    assert_eq!(
        get_default_field_name(&s, false, "user", "email").unwrap(),
        "email"
    );
    // a customized DB column name resolves back to its field key
    assert_eq!(
        get_default_field_name(&s, false, "user", "display_name").unwrap(),
        "displayName"
    );
    assert!(get_default_field_name(&s, false, "user", "nope").is_err());
}

#[test]
fn field_name_returns_db_column() {
    let s = schema();
    assert_eq!(
        get_field_name(&s, false, "user", "displayName").unwrap(),
        "display_name"
    );
    assert_eq!(get_field_name(&s, false, "user", "email").unwrap(), "email");
    // `id` has no override
    assert_eq!(get_field_name(&s, false, "user", "id").unwrap(), "id");
}

#[test]
fn model_name_custom_and_plural() {
    let s = schema();
    assert_eq!(get_model_name(&s, false, "user").unwrap(), "app_user");
    assert_eq!(get_model_name(&s, true, "user").unwrap(), "app_users");
}
