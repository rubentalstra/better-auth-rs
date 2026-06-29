//! Database adapters implementing [`better_auth_rs_core::db::DatabaseAdapter`].
//!
//! `memory` (behind `memory-adapter`) is the in-process backend for tests/dev. The
//! `sqlx-postgres` backend lands next.

#[cfg(feature = "memory-adapter")]
pub mod memory;
