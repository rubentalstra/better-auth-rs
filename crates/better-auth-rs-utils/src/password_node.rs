//! Native-`node:crypto` scrypt password hashing (port of `password.node.ts`).
//!
//! `password.node.ts` exists upstream so Node can use the built-in `node:crypto.scrypt` rather than
//! `@noble/hashes` (`password.ts`). Both sides use **identical** parameters (`N=16384`, `r=16`,
//! `p=1`, `dkLen=64`), the same hex-string salt, NFKC normalization, and the `"{salt}:{key}"`
//! storage format, so their outputs are byte-for-byte interchangeable (upstream even asserts this
//! cross-compatibility in its tests).
//!
//! In Rust there is no Node-vs-noble split: a single audited `scrypt` implementation backs both.
//! This module therefore re-exports [`crate::password`], preserving the 1:1 file mapping with
//! `password.node.ts` while keeping a single source of truth.

pub use crate::password::{PasswordError, hash_password, verify_password};

#[cfg(test)]
#[path = "password_node.test.rs"]
mod password_node_tests;
