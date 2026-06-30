//! Constant-time byte comparison (port of `crypto/buffer.ts`).

use subtle::ConstantTimeEq;

/// Compare two byte sequences in constant time, returning `true` iff they are byte-for-byte equal.
///
/// Upstream folds the length difference into the accumulator and loops over the longer input;
/// behaviorally that is "equal iff same length and same bytes", which is exactly what this returns.
/// We short-circuit on a length mismatch (length is not secret) and compare equal-length inputs via
/// `subtle`, whose comparison the optimizer is forbidden from making data-dependent.
#[must_use]
pub fn constant_time_equal(a: impl AsRef<[u8]>, b: impl AsRef<[u8]>) -> bool {
    let (a, b) = (a.as_ref(), b.as_ref());
    if a.len() != b.len() {
        return false;
    }
    a.ct_eq(b).into()
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn equal_and_unequal() {
        assert!(constant_time_equal(b"abc", b"abc"));
        assert!(constant_time_equal("hello", "hello"));
        assert!(!constant_time_equal(b"abc", b"abd"));
        assert!(!constant_time_equal(b"abc", b"abcd")); // length mismatch
        assert!(constant_time_equal([], [])); // both empty
    }
}
