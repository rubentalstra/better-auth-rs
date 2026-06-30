//! Upstream reference: db/schema/rate-limit.ts
//!
//! `rateLimitSchema` → the [`RateLimit`] record. Unlike the other entities it does **not** extend
//! `coreSchema` (no `id`/timestamps). The two `z.number()` fields are integer counters/timestamps,
//! so they map to `i64` rather than a float.

use serde::{Deserialize, Serialize};

/// A rate-limit record (`rateLimitSchema`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RateLimit {
    /// The key the limit is tracked under.
    pub key: String,
    /// The number of requests made in the window.
    pub count: i64,
    /// The time of the last request, in milliseconds since the Unix epoch.
    pub last_request: i64,
}

#[cfg(test)]
#[path = "rate_limit.test.rs"]
mod rate_limit_tests;
