//! # better-auth-rs-memory-adapter
//!
//! In-memory `DatabaseAdapter` for tests and development (port of `@better-auth/memory-adapter`).
//! The upstream `.ts` is co-located read-only as the spec.
//!
//! **Port reset.** This crate depends on `better_auth_rs_core::db` (the `DatabaseAdapter` trait and
//! value model), which is being re-ported from scratch. It is re-implemented in the adapter phase
//! once core's `db` surface lands — see `port/manifest.tsv`. No items are wired yet.
