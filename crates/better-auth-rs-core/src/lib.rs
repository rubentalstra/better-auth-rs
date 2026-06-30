//! # better-auth-rs-core
//!
//! Framework-agnostic primitives for `better-auth-rs`, a security-first Rust reimplementation of
//! [`@better-auth/core`](https://github.com/better-auth/better-auth) — inspired by better-auth, not
//! a wire-compatible port. The better-auth TypeScript is co-located read-only as the design
//! reference; modules are built bottom-up — see `port/manifest.tsv` and this crate's `src/CLAUDE.md`.
//!
//! Upstream reference: `index.ts` (`export * from "./types"`) — the public re-export surface is wired
//! here as modules land. Port in progress.

pub mod db;
pub mod env;
pub mod error;
pub mod oauth2;
pub mod types;
pub mod utils;

/// The better-auth upstream version this port currently tracks (pinned in `port/UPSTREAM_PORTED`).
pub const UPSTREAM_VERSION: &str = "1.6.23";

/// The version of this crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
