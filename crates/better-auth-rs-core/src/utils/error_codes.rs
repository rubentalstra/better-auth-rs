//! Upstream reference: utils/error-codes.ts
//!
//! Almost the entire file is compile-time TypeScript with no runtime analog:
//! `UpperLetter`/`SpecialCharacter`/`IsValidUpperSnakeCase`/`InvalidKeyError`/`ValidateErrorCodes`
//! are type-level key validators, and `defineErrorCodes` is a runtime object-builder that turns a
//! `{KEY: message}` map into a `{KEY: { code, message, toString }}` record. Rust models an
//! error-code *set* as a closed enum instead (see [`crate::error::codes::BaseErrorCode`]), so there
//! is no `define_error_codes` function — the enum *is* its realization. The one type that carries
//! over is [`RawError`], the `{ code, message }` value shape, re-exported via `crate::error` (next
//! to its consumer, `APIError::from`).

/// A raw error entry: a machine-readable `code` and a human-readable `message` (upstream
/// `RawError<K> = { readonly code: K; message: string }`). Error codes are static, so both fields
/// are `&'static str`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RawError {
    /// The machine-readable error code.
    pub code: &'static str,
    /// The human-readable message.
    pub message: &'static str,
}
