//! The `rateLimit` model (port of `db/schema/rate-limit.ts` + its `get-tables.ts` table).
//!
//! Unlike the other models, `rateLimit` is a plain object (no `id`/timestamps) and the table is
//! only created when `rateLimit.storage = "database"`.

use serde::{Deserialize, Serialize};

use crate::db::field::{DbFieldType, DefaultValue, FieldAttribute};

/// Base rate-limit record (port of `BaseRateLimit`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RateLimit {
    pub key: String,
    /// Number of requests made.
    pub count: i64,
    /// Last request time in milliseconds since the Unix epoch.
    pub last_request: i64,
}

/// The `rateLimit` table's fields, in upstream order.
pub fn fields() -> Vec<(String, FieldAttribute)> {
    vec![
        (
            "key".into(),
            FieldAttribute::new(DbFieldType::String).unique(),
        ),
        ("count".into(), FieldAttribute::new(DbFieldType::Number)),
        (
            "lastRequest".into(),
            // upstream `defaultValue: () => Date.now()` — unix millis on this bigint column.
            FieldAttribute::new(DbFieldType::Number)
                .bigint()
                .default(DefaultValue::Now),
        ),
    ]
}
