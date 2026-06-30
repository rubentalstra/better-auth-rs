//! Interim aggregator for `error/`. The full port of `error/index.ts` (`BetterAuthError`,
//! `APIError`) is pending — that file's manifest row stays `todo` until then. For now this wires
//! `codes` and re-exports the error-code surface, mirroring `index.ts`'s
//! `export { type APIErrorCode, BASE_ERROR_CODES } from "./codes"`.

pub mod codes;

pub use codes::{ApiErrorCode, BaseErrorCode};
