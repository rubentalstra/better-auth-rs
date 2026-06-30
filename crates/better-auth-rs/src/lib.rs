//! # better-auth-rs
//!
//! A faithful 1:1 Rust port of better-auth (`packages/better-auth`). Upstream TypeScript is
//! vendored read-only as co-located `.ts` siblings (the spec); modules are ported bottom-up,
//! file-by-file — see `port/manifest.tsv` and `src/CLAUDE.md`.
//!
//! Upstream source: `index.ts` — the public re-export surface is wired here as modules land. This
//! crate depends on [`better_auth_rs_core`]; porting proceeds once core's surface exists. Port in
//! progress: no modules are wired yet.

pub use better_auth_rs_core as core;

/// The better-auth upstream version this port currently tracks (pinned in `port/UPSTREAM_PORTED`).
pub const UPSTREAM_VERSION: &str = better_auth_rs_core::UPSTREAM_VERSION;

/// The version of this crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
