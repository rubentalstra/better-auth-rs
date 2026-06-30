//! Upstream reference: utils/id.ts

use better_auth_rs_utils::random::{Alphabet, create_random_string_generator};

/// Generate a random id from the alphabet `[a-z A-Z 0-9]`; defaults to length 32
/// (`generateId(size?)` → `createRandomStringGenerator("a-z","A-Z","0-9")(size || 32)`).
#[must_use]
pub fn generate_id(size: Option<usize>) -> String {
    create_random_string_generator(&[Alphabet::LowerAlpha, Alphabet::UpperAlpha, Alphabet::Digits])
        .generate(size.unwrap_or(32))
}

#[cfg(test)]
#[path = "id.test.rs"]
mod id_tests;
