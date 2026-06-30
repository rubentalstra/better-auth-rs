---
name: writing-parity-tests
description: How to write Rust behavior tests for a better-auth-rs feature, translating the useful cases from the sibling *.test.ts. Use when implementing any unit that has a sibling test, or when adding test coverage.
---

# Writing behavior tests

We prove each feature with our **own Rust tests** asserting correct, secure behavior. There is **no
TS-vs-Rust differential harness** and **no wire/byte-format parity** with better-auth — a Rust
client talks to a Rust server.

## Translate the useful cases

- The sibling `*.test.ts` is a **source of test cases**, not a spec to match byte-for-byte. Pull
  the cases worth keeping (edge cases, parsing tables, error conditions) into a Rust test
  **co-located with the module it covers** — a `<stem>.test.rs` child module wired with
  `#[cfg(test)] #[path = "<stem>.test.rs"] mod <stem>_tests;` (header
  `#![allow(clippy::unwrap_used, clippy::expect_used)]`).
- A test must be able to fail for the right reason. No flaky/time-based waits — await the
  condition, not the clock.
- Assert **behavior**, not internal byte formats: hash-then-verify round-trips; a signed cookie
  verifies and a tampered one is rejected; an expired token is refused. (Crypto/cookies are audited
  crates with their own formats.)
- For security-sensitive logic (auth flows, token/session handling, input parsing), add thorough
  edge-case and negative tests — the platform's security rests on them.
- DB-touching tests use the `memory-adapter` (no DB) or the docker-compose Postgres service.

## Win condition

The feature works correctly and **securely**, is built on audited crates, and is covered by passing
Rust tests (`cargo nextest run`), with `clippy --all-targets -- -D warnings` and `fmt --all --check`
clean.
