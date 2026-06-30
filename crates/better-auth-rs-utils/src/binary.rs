//! Text encoding / decoding (port of `binary.ts`).
//!
//! Upstream wraps `TextEncoder`/`TextDecoder`: `encode` is always UTF-8, and `decode` supports
//! `utf-8 | utf-16 | iso-8859-1`, lossily substituting U+FFFD for malformed input (matching
//! `TextDecoder`'s default, non-fatal mode). `TextDecoder("utf-16")` is little-endian and consumes a
//! leading byte-order mark.

/// Decoding charset — upstream's `Encoding` (`"utf-8" | "utf-16" | "iso-8859-1"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BinaryEncoding {
    /// UTF-8 (upstream default).
    #[default]
    Utf8,
    /// UTF-16 little-endian; a leading BOM is consumed (matching `TextDecoder("utf-16")`).
    Utf16,
    /// ISO-8859-1 / latin1 — each byte maps to the identical Unicode code point.
    Iso8859_1,
}

/// Encode a string as its UTF-8 bytes (upstream `encoder.encode`, always UTF-8).
#[must_use]
pub fn encode(data: &str) -> &[u8] {
    data.as_bytes()
}

/// Decode `data` to a `String` under `encoding`, lossily substituting U+FFFD for malformed input
/// (matching `TextDecoder`'s default non-fatal behavior).
#[must_use]
pub fn decode(data: &[u8], encoding: BinaryEncoding) -> String {
    match encoding {
        BinaryEncoding::Utf8 => String::from_utf8_lossy(data).into_owned(),
        BinaryEncoding::Utf16 => {
            // `TextDecoder("utf-16")` is little-endian and strips a leading BOM (U+FEFF → `FF FE`).
            let bytes = match data {
                [0xff, 0xfe, rest @ ..] => rest,
                _ => data,
            };
            let units = bytes
                .chunks_exact(2)
                .map(|p| u16::from_le_bytes([p[0], p[1]]));
            char::decode_utf16(units)
                .map(|r| r.unwrap_or('\u{FFFD}'))
                .collect()
        }
        BinaryEncoding::Iso8859_1 => data.iter().map(|&b| b as char).collect(),
    }
}

#[cfg(test)]
#[path = "binary.test.rs"]
mod binary_tests;
