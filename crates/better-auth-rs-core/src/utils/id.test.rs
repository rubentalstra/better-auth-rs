//! Behavior tests for `generate_id`.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;

#[test]
fn default_length_is_32_and_alnum() {
    let id = generate_id(None);
    assert_eq!(id.len(), 32);
    assert!(id.chars().all(|c| c.is_ascii_alphanumeric()));
}

#[test]
fn honors_custom_size() {
    assert_eq!(generate_id(Some(8)).len(), 8);
    assert_eq!(generate_id(Some(64)).len(), 64);
}

#[test]
fn ids_are_distinct() {
    assert_ne!(generate_id(None), generate_id(None));
}
