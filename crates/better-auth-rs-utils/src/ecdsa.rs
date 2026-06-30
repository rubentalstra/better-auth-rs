//! ECDSA over NIST P-256 / P-384 / P-521 (port of `ecdsa.ts`).
//!
//! Upstream wraps Web Crypto `ECDSA`: `generateKeyPair` exports the private key as **PKCS#8** DER
//! and the public key as **SPKI** DER; `sign`/`verify` use Web Crypto's fixed-width **IEEE-P1363**
//! (`r ‖ s`) signature encoding; `exportKey` supports `jwk | spki | pkcs8 | raw`. This port mirrors
//! that over the audited `p256`/`p384`/`p521` + `ecdsa` crates.
//!
//! Adaptation: Web Crypto takes the digest as a free `hash` parameter. RustCrypto's high-level
//! signer pairs each curve with its standard digest (P-256→SHA-256, P-384→SHA-384, P-521→SHA-512);
//! better-auth only ever uses the default **SHA-256 with P-256**, which is exactly that pairing, so
//! the [`sign`]/[`verify`] `hash` argument is accepted for API parity and documented to follow the
//! curve's standard digest. The opaque Web Crypto `CryptoKey` becomes the typed [`EcdsaPrivateKey`]
//! / [`EcdsaPublicKey`].

use p256::ecdsa::signature::{Signer, Verifier};
use p256::elliptic_curve::Generate;
use p256::elliptic_curve::sec1::ToSec1Point;
use pkcs8::{DecodePrivateKey, EncodePrivateKey};
use spki::{DecodePublicKey, EncodePublicKey};

use crate::base64::base64_url;
use crate::types::{EcdsaCurve, ExportKeyFormat, ShaFamily};

/// Errors from ECDSA operations.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EcdsaError {
    /// A key could not be parsed from the supplied DER.
    #[error("ECDSA key import failed: {0}")]
    Import(String),
    /// A key could not be serialized to the requested format.
    #[error("ECDSA key export failed: {0}")]
    Export(String),
    /// Signing failed.
    #[error("ECDSA signing failed: {0}")]
    Sign(String),
    /// The export format is not valid for this key kind (e.g. `spki` for a private key).
    #[error("export format {0:?} is not valid for this key kind")]
    UnsupportedFormat(ExportKeyFormat),
}

/// The result of an `export` — mirrors upstream's `E extends "jwk" ? JsonWebKey : ArrayBuffer`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportedKey {
    /// DER bytes (`spki` / `pkcs8`).
    Der(Vec<u8>),
    /// SEC1 point / scalar bytes (`raw`).
    Raw(Vec<u8>),
    /// A JSON Web Key (`jwk`).
    Jwk(serde_json::Value),
}

/// An ECDSA private (signing) key, tagged by curve.
pub enum EcdsaPrivateKey {
    /// NIST P-256.
    P256(p256::ecdsa::SigningKey),
    /// NIST P-384.
    P384(p384::ecdsa::SigningKey),
    /// NIST P-521.
    P521(p521::ecdsa::SigningKey),
}

// Hand-written, redacted Debug — never expose private scalar material.
impl core::fmt::Debug for EcdsaPrivateKey {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("EcdsaPrivateKey")
            .field("curve", &self.curve().jwk_name())
            .field("scalar", &"<redacted>")
            .finish()
    }
}

/// An ECDSA public (verifying) key, tagged by curve.
#[derive(Debug, Clone)]
pub enum EcdsaPublicKey {
    /// NIST P-256.
    P256(p256::ecdsa::VerifyingKey),
    /// NIST P-384.
    P384(p384::ecdsa::VerifyingKey),
    /// NIST P-521.
    P521(p521::ecdsa::VerifyingKey),
}

fn import_err(e: impl core::fmt::Display) -> EcdsaError {
    EcdsaError::Import(e.to_string())
}
fn export_err(e: impl core::fmt::Display) -> EcdsaError {
    EcdsaError::Export(e.to_string())
}

/// Generate a key pair, returning `(pkcs8_private_der, spki_public_der)` — matching
/// `ecdsa.generateKeyPair`.
pub fn generate_key_pair(curve: EcdsaCurve) -> Result<(Vec<u8>, Vec<u8>), EcdsaError> {
    macro_rules! make {
        ($m:ident) => {{
            let sk = $m::ecdsa::SigningKey::generate();
            let vk = sk.verifying_key();
            let priv_der = sk.to_pkcs8_der().map_err(export_err)?.as_bytes().to_vec();
            let pub_der = vk
                .to_public_key_der()
                .map_err(export_err)?
                .as_bytes()
                .to_vec();
            (priv_der, pub_der)
        }};
    }
    Ok(match curve {
        EcdsaCurve::P256 => make!(p256),
        EcdsaCurve::P384 => make!(p384),
        EcdsaCurve::P521 => make!(p521),
    })
}

/// Import a PKCS#8-DER private key for `curve`.
pub fn import_private_key(
    pkcs8_der: &[u8],
    curve: EcdsaCurve,
) -> Result<EcdsaPrivateKey, EcdsaError> {
    Ok(match curve {
        EcdsaCurve::P256 => EcdsaPrivateKey::P256(
            p256::ecdsa::SigningKey::from_pkcs8_der(pkcs8_der).map_err(import_err)?,
        ),
        EcdsaCurve::P384 => EcdsaPrivateKey::P384(
            p384::ecdsa::SigningKey::from_pkcs8_der(pkcs8_der).map_err(import_err)?,
        ),
        EcdsaCurve::P521 => EcdsaPrivateKey::P521(
            p521::ecdsa::SigningKey::from_pkcs8_der(pkcs8_der).map_err(import_err)?,
        ),
    })
}

/// Import an SPKI-DER public key for `curve`.
pub fn import_public_key(spki_der: &[u8], curve: EcdsaCurve) -> Result<EcdsaPublicKey, EcdsaError> {
    Ok(match curve {
        EcdsaCurve::P256 => EcdsaPublicKey::P256(
            p256::ecdsa::VerifyingKey::from_public_key_der(spki_der).map_err(import_err)?,
        ),
        EcdsaCurve::P384 => EcdsaPublicKey::P384(
            p384::ecdsa::VerifyingKey::from_public_key_der(spki_der).map_err(import_err)?,
        ),
        EcdsaCurve::P521 => EcdsaPublicKey::P521(
            p521::ecdsa::VerifyingKey::from_public_key_der(spki_der).map_err(import_err)?,
        ),
    })
}

/// Sign `data`, returning a fixed-width IEEE-P1363 (`r ‖ s`) signature. `_hash` mirrors the upstream
/// API; the curve's standard digest is used (SHA-256 for P-256 — better-auth's only case).
pub fn sign(key: &EcdsaPrivateKey, data: &[u8], _hash: ShaFamily) -> Result<Vec<u8>, EcdsaError> {
    macro_rules! sign_with {
        ($sk:expr, $m:ident) => {{
            let sig: $m::ecdsa::Signature = $sk
                .try_sign(data)
                .map_err(|e| EcdsaError::Sign(e.to_string()))?;
            sig.to_bytes().as_slice().to_vec()
        }};
    }
    Ok(match key {
        EcdsaPrivateKey::P256(sk) => sign_with!(sk, p256),
        EcdsaPrivateKey::P384(sk) => sign_with!(sk, p384),
        EcdsaPrivateKey::P521(sk) => sign_with!(sk, p521),
    })
}

/// Verify an IEEE-P1363 (`r ‖ s`) `signature` over `data`. A malformed signature returns `Ok(false)`.
/// `_hash` mirrors the upstream API (see [`sign`]).
pub fn verify(
    key: &EcdsaPublicKey,
    signature: &[u8],
    data: &[u8],
    _hash: ShaFamily,
) -> Result<bool, EcdsaError> {
    macro_rules! verify_with {
        ($vk:expr, $m:ident) => {{
            match $m::ecdsa::Signature::from_slice(signature) {
                Ok(sig) => $vk.verify(data, &sig).is_ok(),
                Err(_) => false,
            }
        }};
    }
    Ok(match key {
        EcdsaPublicKey::P256(vk) => verify_with!(vk, p256),
        EcdsaPublicKey::P384(vk) => verify_with!(vk, p384),
        EcdsaPublicKey::P521(vk) => verify_with!(vk, p521),
    })
}

impl EcdsaPrivateKey {
    /// The curve this key belongs to.
    #[must_use]
    pub fn curve(&self) -> EcdsaCurve {
        match self {
            EcdsaPrivateKey::P256(_) => EcdsaCurve::P256,
            EcdsaPrivateKey::P384(_) => EcdsaCurve::P384,
            EcdsaPrivateKey::P521(_) => EcdsaCurve::P521,
        }
    }

    /// Export this private key (`pkcs8 | jwk | raw`). `spki` is rejected (public-only).
    pub fn export(&self, format: ExportKeyFormat) -> Result<ExportedKey, EcdsaError> {
        macro_rules! export_priv {
            ($sk:expr, $m:ident, $crv:expr) => {{
                match format {
                    ExportKeyFormat::Pkcs8 => Ok(ExportedKey::Der(
                        $sk.to_pkcs8_der().map_err(export_err)?.as_bytes().to_vec(),
                    )),
                    ExportKeyFormat::Raw => Ok(ExportedKey::Raw($sk.to_bytes().as_slice().to_vec())),
                    ExportKeyFormat::Jwk => {
                        let pt = $sk.verifying_key().as_affine().to_sec1_point(false);
                        let x = pt.x().ok_or_else(|| EcdsaError::Export("missing affine x".into()))?;
                        let y = pt.y().ok_or_else(|| EcdsaError::Export("missing affine y".into()))?;
                        Ok(ExportedKey::Jwk(serde_json::json!({
                            "kty": "EC",
                            "crv": $crv,
                            "x": base64_url::encode(x.as_slice(), false),
                            "y": base64_url::encode(y.as_slice(), false),
                            "d": base64_url::encode($sk.to_bytes().as_slice(), false),
                        })))
                    }
                    ExportKeyFormat::Spki => Err(EcdsaError::UnsupportedFormat(ExportKeyFormat::Spki)),
                }
            }};
        }
        match self {
            EcdsaPrivateKey::P256(sk) => export_priv!(sk, p256, "P-256"),
            EcdsaPrivateKey::P384(sk) => export_priv!(sk, p384, "P-384"),
            EcdsaPrivateKey::P521(sk) => export_priv!(sk, p521, "P-521"),
        }
    }
}

impl EcdsaPublicKey {
    /// The curve this key belongs to.
    #[must_use]
    pub fn curve(&self) -> EcdsaCurve {
        match self {
            EcdsaPublicKey::P256(_) => EcdsaCurve::P256,
            EcdsaPublicKey::P384(_) => EcdsaCurve::P384,
            EcdsaPublicKey::P521(_) => EcdsaCurve::P521,
        }
    }

    /// Export this public key (`spki | jwk | raw`). `pkcs8` is rejected (private-only).
    pub fn export(&self, format: ExportKeyFormat) -> Result<ExportedKey, EcdsaError> {
        macro_rules! export_pub {
            ($vk:expr, $m:ident, $crv:expr) => {{
                match format {
                    ExportKeyFormat::Spki => Ok(ExportedKey::Der(
                        $vk.to_public_key_der().map_err(export_err)?.as_bytes().to_vec(),
                    )),
                    ExportKeyFormat::Raw => {
                        Ok(ExportedKey::Raw($vk.as_affine().to_sec1_point(false).as_bytes().to_vec()))
                    }
                    ExportKeyFormat::Jwk => {
                        let pt = $vk.as_affine().to_sec1_point(false);
                        let x = pt.x().ok_or_else(|| EcdsaError::Export("missing affine x".into()))?;
                        let y = pt.y().ok_or_else(|| EcdsaError::Export("missing affine y".into()))?;
                        Ok(ExportedKey::Jwk(serde_json::json!({
                            "kty": "EC",
                            "crv": $crv,
                            "x": base64_url::encode(x.as_slice(), false),
                            "y": base64_url::encode(y.as_slice(), false),
                        })))
                    }
                    ExportKeyFormat::Pkcs8 => Err(EcdsaError::UnsupportedFormat(ExportKeyFormat::Pkcs8)),
                }
            }};
        }
        match self {
            EcdsaPublicKey::P256(vk) => export_pub!(vk, p256, "P-256"),
            EcdsaPublicKey::P384(vk) => export_pub!(vk, p384, "P-384"),
            EcdsaPublicKey::P521(vk) => export_pub!(vk, p521, "P-521"),
        }
    }
}

#[cfg(test)]
#[path = "ecdsa.test.rs"]
mod ecdsa_tests;
