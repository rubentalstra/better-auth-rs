//! # better-auth-rs-test-utils
//!
//! Backend-agnostic `DatabaseAdapter` conformance battery (port of `@better-auth/test-utils`'s
//! adapter suites). The upstream `.ts` suites are co-located read-only as the spec.
//!
//! **Port reset.** This crate depends on `better_auth_rs_core::db` (the `DatabaseAdapter` trait and
//! value model), which is being re-ported from scratch. The conformance harness is re-ported in
//! the adapter phase once core's `db` surface lands — see `port/manifest.tsv`. No items are wired
//! yet.
