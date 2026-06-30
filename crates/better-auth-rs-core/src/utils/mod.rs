//! Rust-only module aggregator for `utils/` (no upstream `utils/index.ts`). Wires the utility
//! modules ported so far.

pub(crate) mod error_codes;
pub mod id;

pub use id::generate_id;
