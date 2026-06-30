//! # better-auth-rs
//!
//! A security-first Rust reimplementation of better-auth (`packages/better-auth`) — inspired by
//! better-auth, not a wire-compatible port. The better-auth TypeScript is co-located read-only as
//! the design reference; modules are built bottom-up — see `port/manifest.tsv` and `src/CLAUDE.md`.
//!
//! Upstream reference: `index.ts` — the public re-export surface is wired here as modules land. This
//! crate depends on [`better_auth_rs_core`]; work proceeds once core's surface exists.

pub use better_auth_rs_core as core;

/// The better-auth upstream version this port currently tracks (pinned in `port/UPSTREAM_PORTED`).
pub const UPSTREAM_VERSION: &str = better_auth_rs_core::UPSTREAM_VERSION;

/// The version of this crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
