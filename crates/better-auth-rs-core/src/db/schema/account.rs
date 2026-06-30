//! The `account` model (port of `db/schema/account.ts` + the `account` table in `get-tables.ts`).

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::db::field::{DbFieldType, DefaultValue, FieldAttribute};

/// Base account record (port of `BaseAccount`). `password` lives only on the credential provider.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub id: String,
    pub provider_id: String,
    pub account_id: String,
    pub user_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id_token: Option<String>,
    #[serde(
        default,
        with = "time::serde::rfc3339::option",
        skip_serializing_if = "Option::is_none"
    )]
    pub access_token_expires_at: Option<OffsetDateTime>,
    #[serde(
        default,
        with = "time::serde::rfc3339::option",
        skip_serializing_if = "Option::is_none"
    )]
    pub refresh_token_expires_at: Option<OffsetDateTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

/// The `account` table's fields, in upstream order. Token/secret fields are `returned: false`.
pub fn fields() -> Vec<(String, FieldAttribute)> {
    vec![
        ("accountId".into(), FieldAttribute::new(DbFieldType::String)),
        (
            "providerId".into(),
            FieldAttribute::new(DbFieldType::String),
        ),
        (
            "userId".into(),
            FieldAttribute::new(DbFieldType::String)
                .references("user", "id")
                .indexed(),
        ),
        (
            "accessToken".into(),
            FieldAttribute::new(DbFieldType::String)
                .optional()
                .not_returned(),
        ),
        (
            "refreshToken".into(),
            FieldAttribute::new(DbFieldType::String)
                .optional()
                .not_returned(),
        ),
        (
            "idToken".into(),
            FieldAttribute::new(DbFieldType::String)
                .optional()
                .not_returned(),
        ),
        (
            "accessTokenExpiresAt".into(),
            FieldAttribute::new(DbFieldType::Date)
                .optional()
                .not_returned(),
        ),
        (
            "refreshTokenExpiresAt".into(),
            FieldAttribute::new(DbFieldType::Date)
                .optional()
                .not_returned(),
        ),
        (
            "scope".into(),
            FieldAttribute::new(DbFieldType::String).optional(),
        ),
        (
            "password".into(),
            FieldAttribute::new(DbFieldType::String)
                .optional()
                .not_returned(),
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
