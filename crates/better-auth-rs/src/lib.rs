//! # better-auth-rs
//!
//! A faithful Rust port of [better-auth](https://github.com/better-auth/better-auth)
//! (`packages/better-auth`) — comprehensive, framework-agnostic authentication and
//! authorization.
//!
//! ## Features
//!
//! `axum` and `sqlx-postgres` are enabled by default. Plugins are opt-in Cargo
//! features named exactly like their upstream better-auth counterparts
//! (`two-factor`, `organization`, `api-key`, `jwt`, `oidc-provider`, ...).
//!
//! ```toml
//! better-auth-rs = { version = "0.1", features = ["axum", "sqlx-postgres", "organization", "two-factor", "jwt"] }
//! ```
//!
//! ## Status
//!
//! Under active porting (see `.claude/phases/`). The upstream TypeScript source is
//! vendored read-only under `reference/better-auth/` as the source of truth for
//! intended behavior; each module is translated 1:1 and proven by ported tests plus
//! a TS-vs-Rust differential harness.

pub use better_auth_rs_core as core;

/// The better-auth upstream version this port currently tracks.
pub const UPSTREAM_VERSION: &str = better_auth_rs_core::UPSTREAM_VERSION;

/// The version of this crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
