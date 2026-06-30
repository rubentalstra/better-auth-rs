//! Database adapters implementing [`better_auth_rs_core::db::DatabaseAdapter`].
//!
//! `memory` (behind `memory-adapter`) is the in-process backend for tests/dev.
//! `sqlx_postgres` (behind `sqlx-postgres`) is the PostgreSQL backend.

#[cfg(feature = "memory-adapter")]
pub mod memory;

#[cfg(feature = "sqlx-postgres")]
pub mod sqlx_postgres;

/// Generate an opaque text id, used when `create` is called without one (better-auth ids are
/// opaque strings). Shared by the adapters that need to mint ids.
#[cfg(any(feature = "memory-adapter", feature = "sqlx-postgres"))]
pub(crate) fn generate_id() -> String {
    uuid::Uuid::new_v4().simple().to_string()
}
