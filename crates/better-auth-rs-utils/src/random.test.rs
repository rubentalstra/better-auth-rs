//! Port of `random.test.ts`.
//!
//! Adaptations: the upstream "no valid characters" / "length not positive" `throw`s are `panic!`s
//! here (the generator asserts), tested via `#[should_panic]`. Negative length is unrepresentable
//! (`usize`), so only `0` is tested. The "combines multiple alphabets" case can't stub `getrandom`,
//! so it uses a long unmocked sample + a deterministic membership check instead of a mocked sequence.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::HashMap;

use super::*;

fn large_sample(
    generator: &RandomStringGenerator,
    sample_count: usize,
    string_length: usize,
) -> String {
    (0..sample_count)
        .map(|_| generator.generate(string_length))
        .collect()
}

fn char_counts(random_string: &str, expected_char_set: &str) -> HashMap<char, usize> {
    let mut counts: HashMap<char, usize> = expected_char_set.chars().map(|c| (c, 0)).collect();
    for c in random_string.chars() {
        *counts.entry(c).or_insert(0) += 1;
    }
    counts
}

fn chi_squared(counts: &HashMap<char, usize>, total_chars: usize, char_set_length: usize) -> f64 {
    let expected = total_chars as f64 / char_set_length as f64;
    counts
        .values()
        .map(|&count| {
            let deviation = count as f64 - expected;
            deviation * deviation / expected
        })
        .sum()
}

#[test]
fn generates_a_random_string_of_specified_length() {
    let generator = create_random_string_generator(&[Alphabet::LowerAlpha]);
    let s = generator.generate(16);
    assert_eq!(s.len(), 16);
}

#[test]
fn uses_a_custom_alphabet() {
    let generator = create_random_string_generator(&[Alphabet::UpperAlpha, Alphabet::Digits]);
    let s = generator.generate(8);
    let allowed = "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    assert!(s.chars().all(|c| allowed.contains(c)));
}

#[test]
#[should_panic(expected = "No valid characters provided for random string generation.")]
fn panics_when_no_valid_characters() {
    let _ = create_random_string_generator(&[]);
}

#[test]
#[should_panic(expected = "Length must be a positive integer.")]
fn panics_when_length_not_positive() {
    let generator = create_random_string_generator(&[Alphabet::LowerAlpha]);
    let _ = generator.generate(0);
}

#[test]
fn respects_a_new_alphabet_during_generation() {
    let generator = create_random_string_generator(&[Alphabet::LowerAlpha]);
    let s = generator.generate_with(10, &[Alphabet::UpperAlpha]);
    let allowed = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    assert!(s.chars().all(|c| allowed.contains(c)));
}

#[test]
fn consistent_randomness_with_valid_mask() {
    let generator = create_random_string_generator(&[Alphabet::Digits]);
    let s = generator.generate(10);
    assert!(s.chars().all(|c| "0123456789".contains(c)));
}

#[test]
fn combines_multiple_alphabets_during_generation() {
    let generator = create_random_string_generator(&[Alphabet::LowerAlpha]);
    // Long unmocked sample so every char of the combined alphabet appears w.h.p.
    let s = generator.generate_with(4096, &[Alphabet::UpperAlpha, Alphabet::Digits]);
    let expected = "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    assert!(
        expected.chars().all(|c| s.contains(c)),
        "every expected char present"
    );
    assert!(
        s.chars().all(|c| expected.contains(c)),
        "only combined-alphabet chars"
    );
    assert_eq!(s.chars().count(), 4096);
}

// describe("produces unbiased distribution across characters")
#[test]
fn unbiased_distribution_26_char_alphabet() {
    let generator = create_random_string_generator(&[Alphabet::LowerAlpha]);
    let expected_char_set = "abcdefghijklmnopqrstuvwxyz";
    let sample = large_sample(&generator, 1000, 256);
    let counts = char_counts(&sample, expected_char_set);
    let chi = chi_squared(&counts, sample.chars().count(), expected_char_set.len());
    // 25 d.o.f. @ 99.9% ~= 52.62; x3 to avoid random failures.
    assert!(chi < 52.62 * 3.0, "chi-squared {chi} too high");
}

#[test]
fn unbiased_distribution_10_char_alphabet() {
    let generator = create_random_string_generator(&[Alphabet::Digits]);
    let expected_char_set = "0123456789";
    let sample = large_sample(&generator, 1000, 256);
    let total = sample.chars().count();
    let counts = char_counts(&sample, expected_char_set);
    let chi = chi_squared(&counts, total, expected_char_set.len());
    // 9 d.o.f. @ 99.9% ~= 27.877; x3.
    assert!(chi < 27.877 * 3.0, "chi-squared {chi} too high");

    let min = *counts.values().min().unwrap();
    let max = *counts.values().max().unwrap();
    let expected_count = total as f64 / expected_char_set.len() as f64;
    let max_allowed_deviation = expected_count * 0.1;
    assert!(
        ((max - min) as f64) < max_allowed_deviation,
        "max-min {} exceeds {}",
        max - min,
        max_allowed_deviation
    );
}
