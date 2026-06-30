//! The `verification` model (port of `db/schema/verification.ts` + its `get-tables.ts` table).

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::db::types::{DbFieldType, DefaultValue, FieldAttribute};

/// Base verification record (port of `BaseVerification`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Verification {
    pub id: String,
    pub identifier: String,
    pub value: String,
    #[serde(with = "time::serde::rfc3339")]
    pub expires_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

/// The `verification` table's fields, in upstream order.
pub fn fields() -> Vec<(String, FieldAttribute)> {
    vec![
        (
            "identifier".into(),
            FieldAttribute::new(DbFieldType::String).indexed(),
        ),
        ("value".into(), FieldAttribute::new(DbFieldType::String)),
        ("expiresAt".into(), FieldAttribute::new(DbFieldType::Date)),
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
