//! Upstream reference: db/schema/account.ts
//!
//! `accountSchema` (extends `coreSchema`) → the [`Account`] record. zod → `serde`; `.nullish()` →
//! `Option`; nullable date fields use `time::serde::rfc3339::option`. `password` is only set for the
//! credential provider.

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use super::shared::CoreFields;

/// An account record (`accountSchema`) — a link between a user and an auth provider (social or the
/// credential provider).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    /// `id` / `createdAt` / `updatedAt`.
    #[serde(flatten)]
    pub core: CoreFields,
    /// The provider id (e.g. `"google"`, `"credential"`).
    pub provider_id: String,
    /// The account id at the provider.
    pub account_id: String,
    /// The owning user's id.
    pub user_id: String,
    /// OAuth access token, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub access_token: Option<String>,
    /// OAuth refresh token, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
    /// OIDC id token, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id_token: Option<String>,
    /// When the access token expires (RFC 3339), if known.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "time::serde::rfc3339::option"
    )]
    pub access_token_expires_at: Option<OffsetDateTime>,
    /// When the refresh token expires (RFC 3339), if known.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "time::serde::rfc3339::option"
    )]
    pub refresh_token_expires_at: Option<OffsetDateTime>,
    /// The scopes the user authorized.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    /// The hashed password — only stored for the credential provider.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
}

#[cfg(test)]
#[path = "account.test.rs"]
mod account_tests;
