//! `better-auth-rs-utils` — a faithful Rust port of [`@better-auth/utils`](https://github.com/better-auth/utils)
//! (v0.4.2). Low-level building blocks used across the port: encodings (`hex`, `base64`, `binary`),
//! hashing (`hash`), CSPRNG strings (`random`), and password hashing (`password`, scrypt).
//!
//! Implemented over audited RustCrypto crates (`sha2`, `scrypt`, `subtle`) and `getrandom` — no
//! hand-rolled cryptographic primitives. The upstream `.ts` sources are co-located read-only.
//!
//! `hmac`, `otp`, `base32`, `ecdsa`, and `rsa` are ported when their consumers land (later phases).

pub mod base64;
pub mod binary;
pub mod hash;
pub mod hex;
pub mod password;
pub mod random;
