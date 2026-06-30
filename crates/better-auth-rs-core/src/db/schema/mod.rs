//! Rust-only module aggregator for `db/schema/` (no upstream `index.ts`). Wires the entity record
//! structs and re-exports them.

pub mod account;
pub mod rate_limit;
pub mod session;
pub mod shared;
pub mod user;
pub mod verification;

pub use account::Account;
pub use rate_limit::RateLimit;
pub use session::Session;
pub use shared::CoreFields;
pub use user::User;
pub use verification::Verification;
