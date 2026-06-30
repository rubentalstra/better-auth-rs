//! Upstream reference: db/schema/session.ts
//!
//! `sessionSchema` (extends `coreSchema`) → the [`Session`] record. zod → `serde`; `z.coerce.string`
//! for `userId` is a boundary coercion (the field is a `String` here); `.nullish()` → `Option`.

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use super::shared::CoreFields;

/// A session record (`sessionSchema`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    /// `id` / `createdAt` / `updatedAt`.
    #[serde(flatten)]
    pub core: CoreFields,
    /// The owning user's id.
    pub user_id: String,
    /// When the session expires (RFC 3339).
    #[serde(with = "time::serde::rfc3339")]
    pub expires_at: OffsetDateTime,
    /// The session token.
    pub token: String,
    /// Originating IP address, if recorded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ip_address: Option<String>,
    /// Originating user agent, if recorded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_agent: Option<String>,
}

#[cfg(test)]
#[path = "session.test.rs"]
mod session_tests;
