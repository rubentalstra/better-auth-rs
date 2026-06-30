//! Upstream reference: db/schema/shared.ts
//!
//! `coreSchema` (`{ id, createdAt, updatedAt }`, timestamps defaulting to now) → a [`CoreFields`]
//! struct the entity structs flatten in — the Rust analog of `coreSchema.extend(...)`. zod →
//! `serde` + `time`; `.default(() => new Date())` → [`CoreFields::new`] stamps the timestamps.

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

/// The shared fields every core record carries: a string `id` and `createdAt`/`updatedAt`
/// timestamps. Entity structs include it via `#[serde(flatten)]`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoreFields {
    /// Primary key.
    pub id: String,
    /// Creation timestamp (RFC 3339).
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    /// Last-update timestamp (RFC 3339).
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

impl CoreFields {
    /// Build core fields for a new record: the given `id`, with both timestamps stamped to now
    /// (mirrors `coreSchema`'s `createdAt`/`updatedAt` defaults).
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        let now = OffsetDateTime::now_utc();
        Self {
            id: id.into(),
            created_at: now,
            updated_at: now,
        }
    }
}
