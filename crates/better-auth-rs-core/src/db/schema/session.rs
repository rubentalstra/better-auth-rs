//! The `session` model (port of `db/schema/session.ts` + the `session` table in `get-tables.ts`).

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::db::field::{DbFieldType, DefaultValue, FieldAttribute};

/// Base session record (port of `BaseSession`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub id: String,
    pub user_id: String,
    #[serde(with = "time::serde::rfc3339")]
    pub expires_at: OffsetDateTime,
    pub token: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ip_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_agent: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

/// The `session` table's fields, in upstream order.
pub fn fields() -> Vec<(String, FieldAttribute)> {
    vec![
        ("expiresAt".into(), FieldAttribute::new(DbFieldType::Date)),
        (
            "token".into(),
            FieldAttribute::new(DbFieldType::String).unique(),
        ),
        (
            "createdAt".into(),
            FieldAttribute::new(DbFieldType::Date).default(DefaultValue::Now),
        ),
        (
            "updatedAt".into(),
            FieldAttribute::new(DbFieldType::Date).default(DefaultValue::Now),
        ),
        (
            "ipAddress".into(),
            FieldAttribute::new(DbFieldType::String).optional(),
        ),
        (
            "userAgent".into(),
            FieldAttribute::new(DbFieldType::String).optional(),
        ),
        (
            "userId".into(),
            FieldAttribute::new(DbFieldType::String)
                .references("user", "id")
                .indexed(),
        ),
    ]
}
