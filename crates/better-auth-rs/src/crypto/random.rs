//! Random string generation (port of `crypto/random.ts`).
//!
//! Upstream is `createRandomStringGenerator("a-z", "0-9", "A-Z", "-_")` from `@better-auth/utils`.
//! We configure the same generator once over the same alphabet and expose it as a free function.

use std::sync::LazyLock;

pub use better_auth_rs_utils::random::Alphabet;
use better_auth_rs_utils::random::{RandomStringGenerator, create_random_string_generator};

/// The shared generator over better-auth's default alphabet: `a-z 0-9 A-Z - _`.
static GENERATOR: LazyLock<RandomStringGenerator> = LazyLock::new(|| {
    create_random_string_generator(&[
        Alphabet::LowerAlpha,
        Alphabet::Digits,
        Alphabet::UpperAlpha,
        Alphabet::Symbols,
    ])
});

/// Generate a random string of `length` characters over the default alphabet (`a-z 0-9 A-Z - _`).
#[must_use]
pub fn generate_random_string(length: usize) -> String {
    GENERATOR.generate(length)
}

/// Generate a random string of `length` characters, overriding the alphabet for this call —
/// the analogue of upstream's `generateRandomString(length, ...alphabets)`.
#[must_use]
pub fn generate_random_string_with(length: usize, alphabets: &[Alphabet]) -> String {
    GENERATOR.generate_with(length, alphabets)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn generates_expected_length_and_charset() {
        let s = generate_random_string(32);
        assert_eq!(s.len(), 32);
        let allowed = "abcdefghijklmnopqrstuvwxyz0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ-_";
        assert!(s.chars().all(|c| allowed.contains(c)));
    }

    #[test]
    fn override_alphabet() {
        let s = generate_random_string_with(40, &[Alphabet::Digits]);
        assert!(s.chars().all(|c| c.is_ascii_digit()));
    }
}
