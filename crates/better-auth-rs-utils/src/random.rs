//! Cryptographically secure random string generation (port of `random.ts`).
//!
//! Faithfully reproduces upstream's rejection-sampling algorithm (reject bytes `>= maxValid` to
//! avoid modulo bias; refill a `length * 2` buffer as needed). The only adaptation: alphabets are a
//! type-safe [`Alphabet`] enum instead of string literals, so an unknown spec is a compile error
//! rather than upstream's runtime `throw`. Randomness comes from `getrandom` (the OS CSPRNG).

/// The character classes upstream accepts (`"a-z" | "A-Z" | "0-9" | "-_"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Alphabet {
    /// `abcdefghijklmnopqrstuvwxyz`
    LowerAlpha,
    /// `ABCDEFGHIJKLMNOPQRSTUVWXYZ`
    UpperAlpha,
    /// `0123456789`
    Digits,
    /// `-_`
    Symbols,
}

impl Alphabet {
    const fn chars(self) -> &'static str {
        match self {
            Alphabet::LowerAlpha => "abcdefghijklmnopqrstuvwxyz",
            Alphabet::UpperAlpha => "ABCDEFGHIJKLMNOPQRSTUVWXYZ",
            Alphabet::Digits => "0123456789",
            Alphabet::Symbols => "-_",
        }
    }
}

fn charset_of(alphabets: &[Alphabet]) -> Vec<u8> {
    alphabets.iter().flat_map(|a| a.chars().bytes()).collect()
}

/// A reusable generator bound to a base alphabet — the analogue of upstream's
/// `createRandomStringGenerator(...baseAlphabets)`.
#[derive(Debug, Clone)]
pub struct RandomStringGenerator {
    base_charset: Vec<u8>,
}

/// Build a generator from one or more base alphabets.
///
/// # Panics
/// Panics if `base_alphabets` is empty (matching upstream, which throws). Callers pass a fixed,
/// non-empty set of constants, so this is unreachable in practice.
#[must_use]
pub fn create_random_string_generator(base_alphabets: &[Alphabet]) -> RandomStringGenerator {
    let base_charset = charset_of(base_alphabets);
    assert!(
        !base_charset.is_empty(),
        "No valid characters provided for random string generation."
    );
    RandomStringGenerator { base_charset }
}

impl RandomStringGenerator {
    /// Generate a string of `length` characters from the base alphabet.
    #[must_use]
    pub fn generate(&self, length: usize) -> String {
        self.generate_with(length, &[])
    }

    /// Generate a string of `length` characters. If `alphabets` is non-empty it overrides the base
    /// alphabet for this call (mirroring upstream's per-call sub-alphabets).
    ///
    /// # Panics
    /// Panics if `length` is zero or the OS CSPRNG is unavailable — both unrecoverable here.
    #[must_use]
    #[allow(clippy::expect_used)]
    pub fn generate_with(&self, length: usize, alphabets: &[Alphabet]) -> String {
        assert!(length > 0, "Length must be a positive integer.");

        let charset = if alphabets.is_empty() {
            self.base_charset.clone()
        } else {
            charset_of(alphabets)
        };
        let charset_len = charset.len();
        let max_valid = (256 / charset_len) * charset_len;

        let mut result = String::with_capacity(length);
        let mut buf = vec![0u8; length * 2];
        let mut buf_index = buf.len();

        while result.len() < length {
            if buf_index >= buf.len() {
                getrandom::fill(&mut buf).expect("OS CSPRNG unavailable");
                buf_index = 0;
            }
            let rand = buf[buf_index];
            buf_index += 1;
            // Reject the high tail so `% charset_len` is unbiased.
            if (rand as usize) < max_valid {
                result.push(charset[rand as usize % charset_len] as char);
            }
        }
        result
    }
}

#[cfg(test)]
#[path = "random.test.rs"]
mod random_tests;
