//! Core data model: the four base tables plus the optional rate-limit table.
//!
//! Each submodule provides a base record struct and a `fields()` function describing its table
//! (port of `db/schema/*.ts` + the table definitions in `db/get-tables.ts`).

pub mod account;
pub mod rate_limit;
pub mod session;
pub mod user;
pub mod verification;

pub use account::Account;
pub use rate_limit::RateLimit;
pub use session::Session;
pub use user::User;
pub use verification::Verification;

use crate::db::types::{BetterAuthDbSchema, TableSchema};

/// The four core tables every better-auth instance has, with default model names, in upstream
/// order (`user`=1, `session`=2, `account`=3, `verification`=4).
///
/// Plugin tables, the optional `rateLimit` table, and options-driven model/field-name
/// customization layer on top of this in the later port of `get-tables.ts`.
pub fn core_tables() -> BetterAuthDbSchema {
    let mut schema = BetterAuthDbSchema::new();
    for (name, order, fields) in [
        ("user", 1, user::fields()),
        ("session", 2, session::fields()),
        ("account", 3, account::fields()),
        ("verification", 4, verification::fields()),
    ] {
        let mut table = TableSchema::new(name, fields);
        table.order = Some(order);
        schema.insert(name.to_string(), table);
    }
    schema
}
