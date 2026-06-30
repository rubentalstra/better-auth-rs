//! UTF-8 ↔ bytes (port of `binary.ts`).
//!
//! Upstream wraps `TextEncoder`/`TextDecoder`. In Rust a `&str` is already guaranteed UTF-8, so
//! `encode` is just `as_bytes` and `decode` is a checked conversion.

/// Encode a string as its UTF-8 bytes.
#[must_use]
pub fn encode(data: &str) -> &[u8] {
    data.as_bytes()
}

/// Decode UTF-8 bytes into a `String`. Errors on invalid UTF-8 (unlike `TextDecoder`, which would
/// lossily substitute U+FFFD — callers in this codebase only ever decode data they just encoded).
pub fn decode(data: &[u8]) -> Result<String, std::str::Utf8Error> {
    std::str::from_utf8(data).map(str::to_owned)
}
