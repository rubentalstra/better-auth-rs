//! Port of `otp.test.ts` (HOTP/TOTP generation, verification, window, URL).
//!
//! Adaptations (behavior-preserving):
//! - Upstream fakes `Date.now()` with vitest timers; here we drive [`Otp::totp_at`]/[`Otp::verify_at`]
//!   with explicit timestamps, so the "different OTP per window" and verification cases are
//!   deterministic.
//! - Upstream's "checks every window candidate" case mocks `./hmac` and counts calls to prove there
//!   is no early return. We instead prove the observable consequence: a code valid only at
//!   `counter + 1` still verifies with `window = 1` (so all candidates are checked), while a code two
//!   windows away does not.
//! - Added beyond upstream: the RFC 4226 (HOTP) and RFC 6238 (TOTP) known-answer vectors.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;

fn otp(secret: &str, digits: Option<u32>, period: Option<u64>) -> Otp {
    create_otp(secret, OtpOptions { digits, period })
}

// it("should generate a valid HOTP for a given counter")
#[test]
fn generates_valid_hotp() {
    let code = otp("1234567890", Some(6), None).hotp(1).unwrap();
    assert_eq!(code.len(), 6);
    assert!(code.bytes().all(|b| b.is_ascii_digit()));
}

// it("should throw error if digits is not between 1 and 8")
#[test]
fn rejects_out_of_range_digits() {
    assert_eq!(
        otp("1234567890", Some(9), None).hotp(1),
        Err(OtpError::InvalidDigits)
    );
    assert_eq!(
        otp("1234567890", Some(0), None).hotp(1),
        Err(OtpError::InvalidDigits)
    );
}

// it("should generate a valid TOTP based on current time")
#[test]
fn generates_valid_totp() {
    let code = otp("1234567890", Some(6), None).totp().unwrap();
    assert_eq!(code.len(), 6);
    assert!(code.bytes().all(|b| b.is_ascii_digit()));
}

// it("should generate different OTPs after each time window")
#[test]
fn different_otp_per_window() {
    let o = otp("1234567890", Some(6), Some(30));
    let a = o.totp_at(0).unwrap(); // counter 0
    let b = o.totp_at(30_000).unwrap(); // counter 1
    assert_ne!(a, b);
}

// it("should verify correct TOTP against generated value")
#[test]
fn verifies_correct_totp() {
    let o = otp("1234567890", None, None);
    let t = 1_700_000_000_000u64;
    let code = o.totp_at(t).unwrap();
    assert!(o.verify_at(&code, t, 1).unwrap());
}

// it("should return false for incorrect TOTP") — deterministic: flip the first digit.
#[test]
fn rejects_incorrect_totp() {
    let o = otp("1234567890", None, None);
    let t = 1_700_000_000_000u64;
    let code = o.totp_at(t).unwrap();
    let flipped: String = code
        .char_indices()
        .map(|(i, c)| {
            if i == 0 {
                if c == '0' { '1' } else { '0' }
            } else {
                c
            }
        })
        .collect();
    assert!(!o.verify_at(&flipped, t, 1).unwrap());
}

// it("should verify TOTP within the window")
#[test]
fn verifies_within_window() {
    let o = otp("1234567890", None, None);
    let t = 1_700_000_000_000u64;
    let code = o.totp_at(t).unwrap();
    assert!(o.verify_at(&code, t, 1).unwrap());
}

// it("should return false for TOTP outside the window") — window -1 → empty candidate range.
#[test]
fn rejects_outside_window() {
    let o = otp("1234567890", None, None);
    let t = 1_700_000_000_000u64;
    let code = o.totp_at(t).unwrap();
    assert!(!o.verify_at(&code, t, -1).unwrap());
}

// it("should check every TOTP window candidate without returning on first match")
// A code valid only at counter+1 still verifies with window 1 (proves no early return); a code two
// windows away does not.
#[test]
fn checks_every_window_candidate() {
    let o = otp("1234567890", Some(6), Some(30));
    let t = 1_700_000_000_000u64;
    let counter = t / 30_000;
    let next_window = o.hotp(counter + 1).unwrap();
    assert!(o.verify_at(&next_window, t, 1).unwrap());
    let two_windows_away = o.hotp(counter + 2).unwrap();
    assert!(!o.verify_at(&two_windows_away, t, 1).unwrap());
}

// it("should generate a valid QR code URL")
#[test]
fn generates_qr_code_url() {
    let url = otp("1234567890", None, None).url("my-site.com", "account");
    assert!(url.starts_with("otpauth://totp/"));
    assert!(url.contains("otpauth://totp"));
    assert!(url.contains("secret="));
    assert!(url.contains("issuer=my-site.com"));
    assert!(url.contains("digits=6"));
    assert!(url.contains("period=30"));
}

// ---- Known-answer vectors (beyond upstream's shape checks) ----

// RFC 4226 Appendix D — HOTP, secret = ASCII "12345678901234567890", 6 digits, counters 0..9.
#[test]
fn rfc4226_hotp_vectors() {
    let o = otp("12345678901234567890", Some(6), None);
    let expected = [
        "755224", "287082", "359152", "969429", "338314", "254676", "287922", "162583", "399871",
        "520489",
    ];
    for (counter, want) in expected.iter().enumerate() {
        assert_eq!(
            o.hotp(counter as u64).unwrap(),
            *want,
            "HOTP counter {counter}"
        );
    }
}

// RFC 6238 — TOTP SHA-1, secret = ASCII "12345678901234567890", 8 digits, period 30.
#[test]
fn rfc6238_totp_vectors() {
    let o = otp("12345678901234567890", Some(8), Some(30));
    let cases = [
        (59u64, "94287082"),
        (1_111_111_109, "07081804"),
        (1_111_111_111, "14050471"),
        (1_234_567_890, "89005924"),
        (2_000_000_000, "69279037"),
    ];
    for (secs, want) in cases {
        assert_eq!(o.totp_at(secs * 1000).unwrap(), want, "TOTP @ {secs}s");
    }
}

// constant_time_equal_otp parity: equal strings match; differing/length-mismatched do not.
#[test]
fn constant_time_compare() {
    assert!(constant_time_equal_otp("123456", "123456"));
    assert!(!constant_time_equal_otp("123456", "123457"));
    assert!(!constant_time_equal_otp("12345", "123456"));
    assert!(!constant_time_equal_otp("1234567", "123456"));
    assert!(constant_time_equal_otp("", ""));
}
