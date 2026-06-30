//! # better-auth-rs-core
//!
//! Framework-agnostic primitives for `better-auth-rs`, a faithful 1:1 Rust port of
//! [`@better-auth/core`](https://github.com/better-auth/better-auth). The upstream TypeScript is
//! vendored read-only as co-located `.ts` siblings (the spec); modules are ported bottom-up,
//! file-by-file — see `port/manifest.tsv` and this crate's `src/CLAUDE.md`.
//!
//! Upstream source: `index.ts` (`export * from "./types"`) — the public re-export surface is wired
//! here as modules land. Port in progress: no modules are wired yet.

/// The better-auth upstream version this port currently tracks (pinned in `port/UPSTREAM_PORTED`).
pub const UPSTREAM_VERSION: &str = "1.6.23";

/// The version of this crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
