//! Upstream reference: db/schema/user.ts
//!
//! `userSchema` (extends `coreSchema`) → the [`User`] record. zod → `serde`; `.nullish()` →
//! `Option`; `.default(false)` → `#[serde(default)]`; the `email` `.transform(toLowerCase)` is
//! applied at creation by [`User::new`] (records read back from storage are already normalized).

use serde::{Deserialize, Serialize};

use super::shared::CoreFields;

/// A user record (`userSchema`). Additional fields from options/plugins are not part of this base
/// struct (the upstream `InferDBFields*` generics are compile-time only and have no runtime analog).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    /// `id` / `createdAt` / `updatedAt`.
    #[serde(flatten)]
    pub core: CoreFields,
    /// Email address (stored lowercased).
    pub email: String,
    /// Whether the email has been verified.
    #[serde(default)]
    pub email_verified: bool,
    /// Display name.
    pub name: String,
    /// Optional avatar URL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
}

impl User {
    /// Create a new user record: `email` is normalized to lowercase, `email_verified` defaults to
    /// `false`, and timestamps are stamped to now — mirroring `userSchema`'s transform/defaults.
    #[must_use]
    pub fn new(id: impl Into<String>, email: impl AsRef<str>, name: impl Into<String>) -> Self {
        Self {
            core: CoreFields::new(id),
            email: email.as_ref().to_lowercase(),
            email_verified: false,
            name: name.into(),
            image: None,
        }
    }
}

#[cfg(test)]
#[path = "user.test.rs"]
mod user_tests;
