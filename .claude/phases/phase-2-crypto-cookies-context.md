# Phase 2 — Crypto + cookies + context

**Goal:** security primitives + request context.
**Preconditions:** Phase 1.

## Scope (reference → target)

- `packages/better-auth/src/crypto` → `crates/better-auth-rs/src/crypto`
- `packages/better-auth/src/cookies`, `src/state.ts` → `crates/better-auth-rs/src/cookies`
- `packages/better-auth/src/context`, `packages/core/src/context` → core + main crate context.

## Design reference reading

`crypto/{password,jwt,index,random,buffer}`, `cookies/{index,cookie-utils,session-store}`,
`context/{create-context,init,helpers,secret-utils}`, `state.ts` — read these for the feature
set and how better-auth behaves, then build the secure idiomatic Rust equivalent.

## What to build

- `PasswordHasher` trait; default is **argon2id** (the `argon2` crate) — a modern best-in-class
  default; other audited hashers (`scrypt`, `bcrypt`) pluggable. Never hand-roll the KDF.
- HMAC-SHA256 signing, AEAD symmetric encryption (chacha20poly1305/aes-gcm), CSPRNG random,
  constant-time compare (`subtle`) — all from audited RustCrypto crates.
- Signed + encrypted, chunked session cookies via the `cookie` crate's signed/private jars.
  `AuthContext` + secret parsing/rotation/versioning.

## Gates

Our own Rust behavior tests for crypto/cookies/context (roundtrip + behavior), `cargo nextest`.

## Exit criteria

Tests green under `cargo nextest`, clippy `-D warnings` and `cargo fmt` clean; cookie names are
the project's own stable defaults (`better-auth.session_token` / `session_data`).
