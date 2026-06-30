//! Upstream reference: db/schema/verification.ts
//!
//! `verificationSchema` (extends `coreSchema`) → the [`Verification`] record: an identifier→value
//! token with an expiry (used for email verification, password reset, OTPs, …). zod → `serde`.

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use super::shared::CoreFields;

/// A verification record (`verificationSchema`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Verification {
    /// `id` / `createdAt` / `updatedAt`.
    #[serde(flatten)]
    pub core: CoreFields,
    /// The stored value (e.g. a token or code).
    pub value: String,
    /// When the verification expires (RFC 3339).
    #[serde(with = "time::serde::rfc3339")]
    pub expires_at: OffsetDateTime,
    /// The identifier the value is keyed by (e.g. an email or a scoped key).
    pub identifier: String,
}

#[cfg(test)]
#[path = "verification.test.rs"]
mod verification_tests;
