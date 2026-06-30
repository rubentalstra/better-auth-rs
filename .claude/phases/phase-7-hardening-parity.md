# Phase 7 — Hardening + security review + ship without bundling TS

**Goal:** harden + polish, run the comprehensive Rust test suite + feature matrix, confirm the published crate bundles no `.ts`, tag v0.1.
**Preconditions:** Phase 6.

## Scope

- Run the **full** Rust behavior test suite (`cargo nextest run`) across every ported route + plugin.
- Security review of auth/crypto/session paths — confirm we lean on audited crates (argon2,
  cookie's signed/private jars, jsonwebtoken/josekit, RustCrypto, subtle), fail closed, and
  hand-roll no security primitives.
- `cargo clippy -- -D warnings`, `cargo fmt --check`, and the full feature matrix
  (`cargo hack check --feature-powerset` or the curated CI set).
- Port the docs-site code blocks (TS → Rust) and ship the static-export GitHub Pages deploy.
- Exclude/prune `reference/` from the published crate (already `exclude`d); confirm the
  packaged crate carries no TS.

## Gates

Full Rust test suite green; security review complete; feature matrix green;
`cargo publish --dry-run -p better-auth-rs` and `-p better-auth-rs-core` succeed; docs static
export builds.

## Exit criteria

`v0.1.0` tagged and published to crates.io; docs live on GitHub Pages; the published crate
carries no `.ts` (already `exclude`d). The co-located `.ts` stays in the repo permanently — the
read-only design reference and sync baseline; it is never deleted.
