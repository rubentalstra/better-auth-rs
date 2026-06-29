//! The `user` model (port of `db/schema/user.ts` + the `user` table in `db/get-tables.ts`).

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::db::field::{DbFieldType, DefaultValue, FieldAttribute};

/// Base user record (port of `BaseUser` from `userSchema`). Instances may carry additional
/// fields from options/plugins; those are handled dynamically via [`crate::db::field::Row`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub id: String,
    /// Stored lower-cased (upstream `userSchema` transforms email to lowercase).
    pub email: String,
    pub email_verified: bool,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

/// The `user` table's fields, in upstream order (id is the implicit primary key, added by the
/// adapter/migration layer). `updatedAt` also has an on-update trigger upstream, applied by the
/// adapter rather than described here.
pub fn fields() -> Vec<(String, FieldAttribute)> {
    vec![
        (
            "name".into(),
            FieldAttribute::new(DbFieldType::String).sortable(),
        ),
        (
            "email".into(),
            FieldAttribute::new(DbFieldType::String).unique().sortable(),
        ),
        (
            "emailVerified".into(),
            FieldAttribute::new(DbFieldType::Boolean)
                .not_input()
                .default(DefaultValue::Value(false.into())),
        ),
        (
            "image".into(),
            FieldAttribute::new(DbFieldType::String).optional(),
        ),
        (
            "createdAt".into(),
            FieldAttribute::new(DbFieldType::Date).default(DefaultValue::Now),
        ),
        (
            "updatedAt".into(),
            FieldAttribute::new(DbFieldType::Date).default(DefaultValue::Now),
        ),
    ]
}
