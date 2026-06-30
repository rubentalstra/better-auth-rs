//! RSA-OAEP encryption + RSA-PSS signatures (port of `rsa.ts`).
//!
//! Upstream wraps Web Crypto: `generateKeyPair` makes an RSA-OAEP key (`modulusLength` 2048/4096,
//! public exponent 65537, hash SHA-256/384/512); `encrypt`/`decrypt` use **RSA-OAEP**; `sign`/`verify`
//! use **RSA-PSS** with `saltLength = 32`; `exportKey`/`importKey` cover `jwk | spki | pkcs8`. Ported
//! over the audited `rsa` crate.
//!
//! Test note: `rsa.test.ts` is written entirely against `vi.spyOn(crypto.subtle, ...)` mocks — it
//! asserts the wrapper calls Web Crypto with specific parameters, never exercising real crypto. Rust
//! has no `subtle` to mock, so `rsa.test.rs` instead proves the real behavior with round-trips
//! (encrypt→decrypt, sign→verify, export→import), per the parity-test guidance.
//!
//! Adaptation: OAEP/PSS are instantiated with **SHA-256** (the upstream default and better-auth's
//! only usage); the `_hash` parameter is accepted for API parity. The opaque Web Crypto `CryptoKey`
//! becomes the typed [`RsaPrivate`] / [`RsaPublic`].

use pkcs8::{DecodePrivateKey, EncodePrivateKey};
use rsa::pss::{SigningKey, VerifyingKey};
use rsa::signature::{RandomizedSigner, SignatureEncoding, Verifier};
use rsa::traits::{PrivateKeyParts, PublicKeyParts};
use rsa::{BoxedUint, Oaep, RsaPrivateKey, RsaPublicKey};
use sha2::Sha256;
use spki::{DecodePublicKey, EncodePublicKey};

use crate::base64::base64_url;
use crate::rng::OsCsprng;
use crate::types::{ExportKeyFormat, ShaFamily};

/// RSA-PSS salt length used by upstream (`saltLength: 32`).
const PSS_SALT_LEN: usize = 32;

/// Errors from RSA operations.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RsaError {
    /// Key generation failed.
    #[error("RSA key generation failed: {0}")]
    KeyGen(String),
    /// A key could not be parsed.
    #[error("RSA key import failed: {0}")]
    Import(String),
    /// A key could not be serialized to the requested format.
    #[error("RSA key export failed: {0}")]
    Export(String),
    /// Encryption/decryption failed.
    #[error("RSA cipher operation failed: {0}")]
    Cipher(String),
    /// Signing failed.
    #[error("RSA signing failed: {0}")]
    Sign(String),
    /// The export format is not valid for this key kind.
    #[error("export format {0:?} is not valid for this key kind")]
    UnsupportedFormat(ExportKeyFormat),
}

/// The result of an `export` — mirrors upstream's `E extends "jwk" ? JsonWebKey : ArrayBuffer`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportedKey {
    /// DER bytes (`spki` / `pkcs8`).
    Der(Vec<u8>),
    /// A JSON Web Key (`jwk`).
    Jwk(serde_json::Value),
}

/// An RSA private key.
pub struct RsaPrivate(RsaPrivateKey);

// Redacted Debug — never expose private key material.
impl core::fmt::Debug for RsaPrivate {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("RsaPrivate")
            .field("d", &"<redacted>")
            .finish()
    }
}

/// An RSA public key.
#[derive(Debug, Clone)]
pub struct RsaPublic(RsaPublicKey);

/// A generated RSA key pair.
#[derive(Debug)]
pub struct RsaKeyPair {
    /// The private key.
    pub private: RsaPrivate,
    /// The public key.
    pub public: RsaPublic,
}

fn b64_uint(u: &BoxedUint) -> String {
    base64_url::encode(u.to_be_bytes_trimmed_vartime(), false)
}

/// Generate an RSA key pair (public exponent 65537). `modulus_bits` is typically 2048 or 4096;
/// `_hash` mirrors the upstream OAEP hash parameter (SHA-256 is used — see the module note).
pub fn generate_key_pair(modulus_bits: usize, _hash: ShaFamily) -> Result<RsaKeyPair, RsaError> {
    let mut rng = OsCsprng;
    let private =
        RsaPrivateKey::new(&mut rng, modulus_bits).map_err(|e| RsaError::KeyGen(e.to_string()))?;
    let public = RsaPublicKey::from(&private);
    Ok(RsaKeyPair {
        private: RsaPrivate(private),
        public: RsaPublic(public),
    })
}

/// Import a PKCS#8-DER private key.
pub fn import_private_pkcs8(der: &[u8]) -> Result<RsaPrivate, RsaError> {
    RsaPrivateKey::from_pkcs8_der(der)
        .map(RsaPrivate)
        .map_err(|e| RsaError::Import(e.to_string()))
}

/// Import an SPKI-DER public key.
pub fn import_public_spki(der: &[u8]) -> Result<RsaPublic, RsaError> {
    RsaPublicKey::from_public_key_der(der)
        .map(RsaPublic)
        .map_err(|e| RsaError::Import(e.to_string()))
}

/// Import a public key from a JWK (`{ kty: "RSA", n, e }`), mirroring `rsa.importKey("jwk", ...)`.
pub fn import_public_jwk(jwk: &serde_json::Value) -> Result<RsaPublic, RsaError> {
    let decode = |field: &str| -> Result<BoxedUint, RsaError> {
        let s = jwk[field]
            .as_str()
            .ok_or_else(|| RsaError::Import(format!("JWK missing `{field}`")))?;
        let bytes = base64_url::decode(s).map_err(|e| RsaError::Import(e.to_string()))?;
        Ok(BoxedUint::from_be_slice_vartime(&bytes))
    };
    let n = decode("n")?;
    let e = decode("e")?;
    RsaPublicKey::new(n, e)
        .map(RsaPublic)
        .map_err(|e| RsaError::Import(e.to_string()))
}

impl RsaPublic {
    /// Encrypt `data` with RSA-OAEP (SHA-256).
    pub fn encrypt(&self, data: &[u8]) -> Result<Vec<u8>, RsaError> {
        let mut rng = OsCsprng;
        self.0
            .encrypt(&mut rng, Oaep::<Sha256>::new(), data)
            .map_err(|e| RsaError::Cipher(e.to_string()))
    }

    /// Verify an RSA-PSS (SHA-256, salt 32) `signature` over `data`. Malformed → `false`.
    #[must_use]
    pub fn verify(&self, signature: &[u8], data: &[u8]) -> bool {
        let vk = VerifyingKey::<Sha256>::new_with_salt_len(self.0.clone(), PSS_SALT_LEN);
        match rsa::pss::Signature::try_from(signature) {
            Ok(sig) => vk.verify(data, &sig).is_ok(),
            Err(_) => false,
        }
    }

    /// Export (`spki | jwk`). `pkcs8` is rejected (private-only).
    pub fn export(&self, format: ExportKeyFormat) -> Result<ExportedKey, RsaError> {
        match format {
            ExportKeyFormat::Spki => Ok(ExportedKey::Der(
                self.0
                    .to_public_key_der()
                    .map_err(|e| RsaError::Export(e.to_string()))?
                    .as_bytes()
                    .to_vec(),
            )),
            ExportKeyFormat::Jwk => Ok(ExportedKey::Jwk(serde_json::json!({
                "kty": "RSA",
                "n": b64_uint(self.0.n()),
                "e": b64_uint(self.0.e()),
            }))),
            other => Err(RsaError::UnsupportedFormat(other)),
        }
    }
}

impl RsaPrivate {
    /// Decrypt `data` with RSA-OAEP (SHA-256).
    pub fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>, RsaError> {
        self.0
            .decrypt(Oaep::<Sha256>::new(), data)
            .map_err(|e| RsaError::Cipher(e.to_string()))
    }

    /// Sign `data` with RSA-PSS (SHA-256, salt 32).
    pub fn sign(&self, data: &[u8]) -> Result<Vec<u8>, RsaError> {
        let mut rng = OsCsprng;
        let sk = SigningKey::<Sha256>::new_with_salt_len(self.0.clone(), PSS_SALT_LEN);
        let sig = sk
            .try_sign_with_rng(&mut rng, data)
            .map_err(|e| RsaError::Sign(e.to_string()))?;
        Ok(sig.to_vec())
    }

    /// The matching public key.
    #[must_use]
    pub fn to_public(&self) -> RsaPublic {
        RsaPublic(RsaPublicKey::from(&self.0))
    }

    /// Export (`pkcs8 | jwk`). `spki` is rejected (public-only).
    pub fn export(&self, format: ExportKeyFormat) -> Result<ExportedKey, RsaError> {
        match format {
            ExportKeyFormat::Pkcs8 => Ok(ExportedKey::Der(
                self.0
                    .to_pkcs8_der()
                    .map_err(|e| RsaError::Export(e.to_string()))?
                    .as_bytes()
                    .to_vec(),
            )),
            ExportKeyFormat::Jwk => {
                let primes = self.0.primes();
                let mut jwk = serde_json::json!({
                    "kty": "RSA",
                    "n": b64_uint(self.0.n()),
                    "e": b64_uint(self.0.e()),
                    "d": b64_uint(self.0.d()),
                });
                if let [p, q, ..] = primes {
                    jwk["p"] = serde_json::Value::String(b64_uint(p));
                    jwk["q"] = serde_json::Value::String(b64_uint(q));
                }
                Ok(ExportedKey::Jwk(jwk))
            }
            other => Err(RsaError::UnsupportedFormat(other)),
        }
    }
}

#[cfg(test)]
#[path = "rsa.test.rs"]
mod rsa_tests;
