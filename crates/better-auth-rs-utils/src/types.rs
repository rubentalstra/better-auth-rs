//! Shared type vocabulary (port of `type.ts`).
//!
//! Renamed `type.ts` → `types.rs` because `type` is a reserved keyword in Rust (`mod type;` is
//! illegal, `r#type` is avoided). Upstream's string-literal unions become closed Rust enums, so an
//! unknown variant is a compile error rather than a runtime `throw`.
//!
//! The TS `TypedArray` and `Uint8Array_` aliases have **no Rust analog**: the first is JS
//! typed-array interop, the second a TypeScript-5.7 generics shim. In Rust, byte input is passed as
//! `impl AsRef<[u8]>` / `&[u8]` directly.

/// SHA family — upstream `SHAFamily` (`"SHA-1" | "SHA-256" | "SHA-384" | "SHA-512"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShaFamily {
    /// SHA-1 (20-byte digest). Legacy; used by HOTP/TOTP (`otp`), not for password/secret hashing.
    Sha1,
    /// SHA-256 (32-byte digest).
    Sha256,
    /// SHA-384 (48-byte digest).
    Sha384,
    /// SHA-512 (64-byte digest).
    Sha512,
}

/// Output encoding — upstream `EncodingFormat`
/// (`"hex" | "base64" | "base64url" | "base64urlnopad" | "none"`).
///
/// Note: the meaning of [`EncodingFormat::Base64`] is **module-dependent**, matching upstream's
/// quirk — `hash.ts` encodes it with the *standard* alphabet, while `hmac.ts` encodes it (and all
/// three base64 variants) with the *URL-safe* alphabet. Each module applies its own rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncodingFormat {
    /// Lowercase hexadecimal.
    Hex,
    /// Base64 (alphabet is module-dependent; see the type note).
    Base64,
    /// URL-safe base64 with padding.
    Base64Url,
    /// URL-safe base64 without padding.
    Base64UrlNoPad,
    /// No encoding — the raw bytes.
    None,
}

/// ECDSA curve — upstream `ECDSACurve` (`"P-256" | "P-384" | "P-521"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EcdsaCurve {
    /// NIST P-256 (secp256r1 / prime256v1).
    P256,
    /// NIST P-384 (secp384r1).
    P384,
    /// NIST P-521 (secp521r1).
    P521,
}

impl EcdsaCurve {
    /// The JWK `crv` value (`"P-256"` etc.) — used by `ecdsa`'s JWK export.
    #[must_use]
    pub const fn jwk_name(self) -> &'static str {
        match self {
            EcdsaCurve::P256 => "P-256",
            EcdsaCurve::P384 => "P-384",
            EcdsaCurve::P521 => "P-521",
        }
    }
}

/// Key export format — upstream `ExportKeyFormat` (`"jwk" | "spki" | "pkcs8" | "raw"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportKeyFormat {
    /// JSON Web Key.
    Jwk,
    /// SubjectPublicKeyInfo DER (public keys).
    Spki,
    /// PKCS#8 DER (private keys).
    Pkcs8,
    /// Raw key bytes.
    Raw,
}

/// The runtime-encoded output of a digest/MAC, mirroring upstream's `ArrayBuffer | string` union:
/// [`EncodingFormat::None`] yields [`Encoded::Raw`]; every other encoding yields [`Encoded::Text`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Encoded {
    /// Raw bytes (the `"none"` encoding → upstream `ArrayBuffer`).
    Raw(Vec<u8>),
    /// An encoded string (hex / base64 variants → upstream `string`).
    Text(String),
}

impl Encoded {
    /// The text, if this is [`Encoded::Text`].
    #[must_use]
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Encoded::Text(s) => Some(s),
            Encoded::Raw(_) => None,
        }
    }

    /// The raw bytes, if this is [`Encoded::Raw`].
    #[must_use]
    pub fn as_raw(&self) -> Option<&[u8]> {
        match self {
            Encoded::Raw(b) => Some(b),
            Encoded::Text(_) => None,
        }
    }
}
