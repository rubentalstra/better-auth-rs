//! Password hashing (port of `crypto/password.ts`).
//!
//! Upstream re-exports `hashPassword`/`verifyPassword` from `@better-auth/utils/password` (scrypt).
//! We do the same — the implementation lives in [`better_auth_rs_utils::password`]. (argon2/bcrypt
//! are intentionally NOT provided: upstream's password layer is scrypt-only.)

pub use better_auth_rs_utils::password::{PasswordError, hash_password, verify_password};
