//! HOTP / TOTP one-time passwords (port of `otp.ts`).
//!
//! HOTP per RFC 4226 (8-byte big-endian counter, HMAC-**SHA-1**, dynamic truncation, `mod 10^digits`,
//! zero-padded) and TOTP per RFC 6238 (`counter = unix_time / period`). The HMAC key is the secret
//! string's UTF-8 bytes, exactly as upstream (`createHMAC(...).sign(secret, ...)`).
//!
//! Adaptations from `otp.ts`, all behavior-preserving:
//! - **Time injection.** Upstream reads `Date.now()` directly; tests fake the clock. Here the public
//!   [`Otp::totp`]/[`Otp::verify`] read the system clock, while [`Otp::totp_at`]/[`Otp::verify_at`]
//!   take an explicit unix-millis timestamp, so tests are deterministic without a global clock mock.
//! - **Constant-time compare.** [`constant_time_equal_otp`] is a 1:1 port of upstream's XOR loop
//!   (added in better-auth/utils#25): it iterates over `expected.len()` so timing never depends on
//!   the candidate, and `verify` OR-accumulates across the whole window with **no early return**.

use std::time::{SystemTime, UNIX_EPOCH};

use crate::base32::base32;
use crate::hmac::create_hmac;
use crate::types::{Encoded, EncodingFormat, ShaFamily};

const DEFAULT_PERIOD: u64 = 30;
const DEFAULT_DIGITS: u32 = 6;

/// Errors from OTP generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum OtpError {
    /// `digits` was outside `1..=8` (upstream throws `TypeError("Digits must be between 1 and 8")`).
    #[error("Digits must be between 1 and 8")]
    InvalidDigits,
}

/// Constant-time string comparison for OTP codes — a 1:1 port of upstream's `constantTimeEqualOTP`.
///
/// Loops over `expected.len()` so timing never depends on `input`; the initial length XOR makes a
/// length mismatch fail. OTP codes are ASCII digits, so byte values equal the JS UTF-16 code units.
#[must_use]
pub fn constant_time_equal_otp(input: &str, expected: &str) -> bool {
    let a = input.as_bytes();
    let b = expected.as_bytes();
    let mut difference = (a.len() ^ b.len()) as u32;
    for (i, &eb) in b.iter().enumerate() {
        // Upstream indexes `input.charCodeAt(i)`; out-of-range yields NaN → ToInt32 → 0.
        let ib = if i < a.len() { u32::from(a[i]) } else { 0 };
        difference |= ib ^ u32::from(eb);
    }
    difference == 0
}

/// Generate an HOTP value for `counter` (RFC 4226). `hash` is `SHA-1` for the public API.
fn generate_hotp(
    secret: &str,
    counter: u64,
    digits: u32,
    hash: ShaFamily,
) -> Result<String, OtpError> {
    if !(1..=8).contains(&digits) {
        return Err(OtpError::InvalidDigits);
    }
    let counter_bytes = counter.to_be_bytes(); // 8-byte big-endian, matching `setBigUint64(..., false)`.
    let mac = match create_hmac(hash, EncodingFormat::None).sign(secret, counter_bytes) {
        Encoded::Raw(b) => b,
        Encoded::Text(_) => unreachable!("`None` encoding always yields `Encoded::Raw`"),
    };
    // Dynamic truncation (RFC 4226 §5.3): low nibble of the last byte selects the offset.
    let offset = (mac[mac.len() - 1] & 0x0f) as usize;
    let truncated = ((u32::from(mac[offset]) & 0x7f) << 24)
        | (u32::from(mac[offset + 1]) << 16)
        | (u32::from(mac[offset + 2]) << 8)
        | u32::from(mac[offset + 3]);
    let otp = truncated % 10u32.pow(digits);
    Ok(format!("{otp:0width$}", width = digits as usize))
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Options for [`create_otp`], mirroring upstream's `{ digits?, period? }`.
#[derive(Debug, Clone, Copy, Default)]
pub struct OtpOptions {
    /// Number of digits (default 6, must be `1..=8`).
    pub digits: Option<u32>,
    /// TOTP period in seconds (default 30).
    pub period: Option<u64>,
}

/// An OTP generator bound to a secret — the analogue of upstream's `createOTP(secret, opts)`.
#[derive(Debug, Clone)]
pub struct Otp {
    secret: String,
    digits: u32,
    period: u64,
}

/// Build an [`Otp`] from a secret and options. The HMAC algorithm is always SHA-1 (as in upstream's
/// public `createOTP`).
#[must_use]
pub fn create_otp(secret: impl Into<String>, opts: OtpOptions) -> Otp {
    Otp {
        secret: secret.into(),
        digits: opts.digits.unwrap_or(DEFAULT_DIGITS),
        period: opts.period.unwrap_or(DEFAULT_PERIOD),
    }
}

impl Otp {
    /// HOTP for an explicit `counter`.
    pub fn hotp(&self, counter: u64) -> Result<String, OtpError> {
        generate_hotp(&self.secret, counter, self.digits, ShaFamily::Sha1)
    }

    /// TOTP for the current system time.
    pub fn totp(&self) -> Result<String, OtpError> {
        self.totp_at(now_ms())
    }

    /// TOTP for an explicit unix-millis timestamp (deterministic; used by tests).
    pub fn totp_at(&self, unix_ms: u64) -> Result<String, OtpError> {
        let counter = unix_ms / (self.period * 1000);
        generate_hotp(&self.secret, counter, self.digits, ShaFamily::Sha1)
    }

    /// Verify `otp` against the current system time. `window` defaults to 1 (`None`); a negative
    /// window yields an empty candidate range (so it never matches), exactly as upstream.
    pub fn verify(&self, otp: &str, window: Option<i64>) -> Result<bool, OtpError> {
        self.verify_at(otp, now_ms(), window.unwrap_or(1))
    }

    /// Verify `otp` against an explicit unix-millis timestamp. Checks every counter in
    /// `[-window, +window]` and OR-accumulates — no early return.
    pub fn verify_at(&self, otp: &str, unix_ms: u64, window: i64) -> Result<bool, OtpError> {
        let counter = (unix_ms / (self.period * 1000)) as i64;
        let mut matched = false;
        let mut i = -window;
        while i <= window {
            let candidate = counter + i;
            if candidate >= 0 {
                let generated =
                    generate_hotp(&self.secret, candidate as u64, self.digits, ShaFamily::Sha1)?;
                // OR-accumulate; do NOT short-circuit (constant work across the window).
                matched = constant_time_equal_otp(otp, &generated) || matched;
            }
            i += 1;
        }
        Ok(matched)
    }

    /// Build an `otpauth://totp/...` provisioning URL (for QR codes).
    #[must_use]
    pub fn url(&self, issuer: &str, account: &str) -> String {
        let base = format!(
            "otpauth://totp/{}:{}",
            encode_uri_component(issuer),
            encode_uri_component(account)
        );
        // URLSearchParams insertion order: secret, issuer, digits, period.
        let query = form_urlencode(&[
            ("secret", &base32::encode(&self.secret, false)),
            ("issuer", issuer),
            ("digits", &self.digits.to_string()),
            ("period", &self.period.to_string()),
        ]);
        format!("{base}?{query}")
    }
}

/// Port of JS `encodeURIComponent`: percent-encode every UTF-8 byte except the unreserved set
/// `A–Z a–z 0–9 - _ . ! ~ * ' ( )`.
fn encode_uri_component(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for &b in s.as_bytes() {
        if b.is_ascii_alphanumeric()
            || matches!(
                b,
                b'-' | b'_' | b'.' | b'!' | b'~' | b'*' | b'\'' | b'(' | b')'
            )
        {
            out.push(b as char);
        } else {
            out.push('%');
            out.push_str(&hex_byte(b));
        }
    }
    out
}

/// Port of the `application/x-www-form-urlencoded` serializer used by `URLSearchParams`:
/// space → `+`, keep `* - . _` and alphanumerics, percent-encode everything else.
fn form_urlencode(pairs: &[(&str, &str)]) -> String {
    pairs
        .iter()
        .map(|(k, v)| format!("{}={}", form_encode(k), form_encode(v)))
        .collect::<Vec<_>>()
        .join("&")
}

fn form_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for &b in s.as_bytes() {
        if b == b' ' {
            out.push('+');
        } else if b.is_ascii_alphanumeric() || matches!(b, b'*' | b'-' | b'.' | b'_') {
            out.push(b as char);
        } else {
            out.push('%');
            out.push_str(&hex_byte(b));
        }
    }
    out
}

fn hex_byte(b: u8) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut s = String::with_capacity(2);
    s.push(HEX[(b >> 4) as usize] as char);
    s.push(HEX[(b & 0x0f) as usize] as char);
    s
}

#[cfg(test)]
#[path = "otp.test.rs"]
mod otp_tests;
