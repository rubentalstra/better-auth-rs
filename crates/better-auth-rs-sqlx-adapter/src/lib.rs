//! # better-auth-rs-sqlx-adapter
//!
//! PostgreSQL `DatabaseAdapter` backed by SQLx — the port of better-auth's Postgres storage path.
//!
//! **Port reset.** This crate depends on `better_auth_rs_core::db` (the `DatabaseAdapter` trait and
//! value model), which is being re-ported from scratch. It is re-implemented in the adapter phase
//! once core's `db` surface lands — see `port/manifest.tsv`. No items are wired yet.
