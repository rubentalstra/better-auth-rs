//! `better-auth-rs-utils` — a faithful Rust port of [`@better-auth/utils`](https://github.com/better-auth/utils)
//! (v0.4.2). Low-level building blocks used across the port: encodings (`hex`, `base64`, `base32`,
//! `binary`), hashing (`hash`), MAC (`hmac`), one-time passwords (`otp`), CSPRNG strings (`random`),
//! password hashing (`password`/`password_node`, scrypt), and asymmetric crypto (`ecdsa`, `rsa`).
//!
//! Implemented over audited RustCrypto crates (`sha1`/`sha2`, `hmac`, `scrypt`, `subtle`, the
//! `p256`/`p384`/`p521` + `ecdsa` and `rsa` stacks) and `getrandom` — no hand-rolled cryptographic
//! primitives. The upstream `.ts` sources are co-located read-only.
//!
//! The heavy asymmetric modules ([`ecdsa`], [`rsa`]) sit behind the on-by-default `ecdsa`/`rsa`
//! Cargo features so downstreams that don't need them (e.g. the main crate, which has its own
//! JWT/JOSE stack) can opt out. (`index.ts`'s `getWebcryptoSubtle` has no Rust analog — RustCrypto
//! is used directly rather than the Web Crypto `SubtleCrypto` handle.)

pub mod base32;
pub mod base64;
pub mod binary;
#[cfg(feature = "ecdsa")]
pub mod ecdsa;
pub mod hash;
pub mod hex;
pub mod hmac;
pub mod otp;
pub mod password;
pub mod password_node;
pub mod random;
#[cfg(feature = "rsa")]
pub mod rsa;
pub mod types;

// Only `rsa` needs an explicit RNG adapter; `ecdsa` keygen is getrandom-backed via `Generate`.
#[cfg(feature = "rsa")]
pub(crate) mod rng;
