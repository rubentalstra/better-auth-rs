//! # better-auth-rs
//!
//! A faithful Rust port of [better-auth](https://github.com/better-auth/better-auth)
//! (`packages/better-auth`) — comprehensive, framework-agnostic authentication and
//! authorization.
//!
//! ## Features
//!
//! `axum` is enabled by default. Storage is chosen by depending on an adapter crate
//! ([`better-auth-rs-sqlx-adapter`] for PostgreSQL, [`better-auth-rs-memory-adapter`] for
//! tests/dev). In-package plugins are opt-in Cargo features named exactly like their upstream
//! better-auth counterparts (`two-factor`, `organization`, `jwt`, `oidc-provider`, ...).
//! Separate-package plugins (`api-key`, `passkey`, `sso`, ...) are their own `better-auth-rs-*`
//! crates.
//!
//! ```toml
//! better-auth-rs = { version = "0.1", features = ["axum", "organization", "two-factor", "jwt"] }
//! better-auth-rs-sqlx-adapter = "0.1"
//! ```
//!
//! ## Status
//!
//! Under active porting (see `.claude/phases/`). The upstream TypeScript source is vendored
//! read-only as co-located `.ts` siblings (next to each `.rs`) — the source of truth for intended
//! behavior; each module is translated 1:1 and proven by ported tests plus a TS-vs-Rust
//! differential harness.
//!
//! [`better-auth-rs-sqlx-adapter`]: https://docs.rs/better-auth-rs-sqlx-adapter
//! [`better-auth-rs-memory-adapter`]: https://docs.rs/better-auth-rs-memory-adapter

pub mod crypto;
pub mod db;

pub use better_auth_rs_core as core;

/// The better-auth upstream version this port currently tracks.
pub const UPSTREAM_VERSION: &str = better_auth_rs_core::UPSTREAM_VERSION;

/// The version of this crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
