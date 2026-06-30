//! # better-auth-rs-core
//!
//! Framework-agnostic primitives for [`better-auth-rs`], a faithful Rust port of
//! [better-auth](https://github.com/better-auth/better-auth) (`@better-auth/core`).
//!
//! This crate will hold the parts of better-auth that do not depend on any web
//! framework or database driver: the data model (`user`/`session`/`account`/
//! `verification`), the `DatabaseAdapter` trait, the OAuth2 protocol layer, the
//! social-provider registry, error codes, and the plugin/context type system.
//!
//! Modules are filled in during **Phase 1+** of the port (see `.claude/phases/`).
//! The upstream TypeScript source lives read-only under `reference/better-auth/`
//! and is the source of truth for intended behavior.

pub mod db;

/// The better-auth upstream version this port currently tracks.
pub const UPSTREAM_VERSION: &str = "1.6.22";

/// The version of this crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
