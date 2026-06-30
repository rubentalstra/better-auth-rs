# Phase 2 — Crypto + cookies + context

**Goal:** security primitives + request context.
**Preconditions:** Phase 1.

## Scope (reference → target)

- `packages/better-auth/src/crypto` → `crates/better-auth-rs/src/crypto`
- `packages/better-auth/src/cookies`, `src/state.ts` → `crates/better-auth-rs/src/cookies`
- `packages/better-auth/src/context`, `packages/core/src/context` → core + main crate context.

## Reference reading

`crypto/{password,jwt,index,random,buffer}`, `cookies/{index,cookie-utils,session-store}`,
`context/{create-context,init,helpers,secret-utils}`, `state.ts`.

## What to build

- `PasswordHasher` trait; default mirrors upstream **scrypt** params/format (Rust `scrypt`
  crate) for test parity; `argon2`/`bcrypt` pluggable.
- HMAC-SHA256 signing, AEAD symmetric encryption (chacha20poly1305/aes-gcm), CSPRNG random,
  constant-time compare (`subtle`).
- Signed + encrypted, chunked session cookies (`cookie` crate). `AuthContext` + secret
  parsing/rotation/versioning.

## Gates

Ported crypto/cookie/context tests (roundtrip + behavior).

## Exit criteria

Tests green; cookie names match upstream (`better-auth.session_token` / `session_data`).
