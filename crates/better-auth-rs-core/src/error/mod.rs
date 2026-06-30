//! Upstream reference: error/index.ts
//!
//! `BetterAuthError` (a JS `Error` subclass — its blanked `stack` is a JS quirk with no analog) and
//! `APIError` (extends `better-call`'s `APIError`). Per "don't reinvent the wheel", `APIError` is
//! built on the `http` crate: `status` is an [`http::StatusCode`] (better-call's status name and
//! numeric `statusCode` collapse into one) and `headers` is an [`http::HeaderMap`]. Re-exports the
//! error-code surface from [`codes`] (mirroring `export { APIErrorCode, BASE_ERROR_CODES }`) and the
//! [`RawError`] value type that [`APIError::from`] consumes.

pub mod codes;

pub use codes::{ApiErrorCode, BaseErrorCode};

pub use crate::utils::error_codes::RawError;

use http::{HeaderMap, StatusCode};

/// Port of `BetterAuthError` (extends JS `Error`). The fixed `name` and blanked `stack` are JS
/// internals with no Rust analog.
#[derive(Debug, thiserror::Error)]
#[error("{message}")]
pub struct BetterAuthError {
    /// The error message.
    pub message: String,
    /// Optional underlying cause (`options.cause`).
    #[source]
    pub cause: Option<Box<dyn std::error::Error + Send + Sync + 'static>>,
}

impl BetterAuthError {
    /// Create an error with a message (no cause).
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            cause: None,
        }
    }

    /// Create an error with a message and an underlying cause.
    #[must_use]
    pub fn with_cause(
        message: impl Into<String>,
        cause: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self {
            message: message.into(),
            cause: Some(Box::new(cause)),
        }
    }
}

/// The body of an [`APIError`] — `{ message?, code? } & Record<string, any>`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ApiErrorBody {
    /// `code`
    pub code: Option<String>,
    /// `message`
    pub message: Option<String>,
    /// Any additional body fields (the `& Record<string, any>` part).
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// Port of `APIError` (extends better-call's `APIError`), built on `http` types.
#[derive(Debug, Clone)]
pub struct APIError {
    /// The HTTP status (better-call's status name + numeric `statusCode`, unified).
    pub status: StatusCode,
    /// The response body sent to the client.
    pub body: ApiErrorBody,
    /// Response headers.
    pub headers: HeaderMap,
}

impl APIError {
    /// Construct from a status, body, and headers.
    #[must_use]
    pub fn new(status: StatusCode, body: ApiErrorBody, headers: HeaderMap) -> Self {
        Self {
            status,
            body,
            headers,
        }
    }

    /// `APIError.fromStatus` — construct from a status and body (no extra headers).
    #[must_use]
    pub fn from_status(status: StatusCode, body: ApiErrorBody) -> Self {
        Self::new(status, body, HeaderMap::new())
    }

    /// `APIError.from` — construct from a status and a `{ code, message }` error entry.
    #[must_use]
    pub fn from(status: StatusCode, error: RawError) -> Self {
        Self::from_status(
            status,
            ApiErrorBody {
                code: Some(error.code.to_owned()),
                message: Some(error.message.to_owned()),
                extra: serde_json::Map::new(),
            },
        )
    }

    /// The numeric HTTP status code (better-call's `statusCode`).
    #[must_use]
    pub fn status_code(&self) -> u16 {
        self.status.as_u16()
    }

    /// The error message — the body message, falling back to the status' canonical reason.
    #[must_use]
    pub fn message(&self) -> &str {
        self.body
            .message
            .as_deref()
            .or_else(|| self.status.canonical_reason())
            .unwrap_or("")
    }
}

impl core::fmt::Display for APIError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.message())
    }
}

impl std::error::Error for APIError {}
