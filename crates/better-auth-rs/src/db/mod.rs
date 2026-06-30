//! The default database layer (port of `packages/better-auth/src/db`).
//!
//! Built on [`better_auth_rs_core::db::DatabaseAdapter`]: the typed-entity ↔ `Row` [`mapping`], the
//! `with_hooks` create/update/delete lifecycle, and the internal (domain) adapter the auth routes
//! call. Ported across Phase 2.

pub mod internal_adapter;
pub mod mapping;
