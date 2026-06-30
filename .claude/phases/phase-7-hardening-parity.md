# Phase 7 — Hardening + parity gate + ship without bundling TS

**Goal:** full parity gate, polish, confirm the published crate bundles no `.ts`, tag v0.1.
**Preconditions:** Phase 6.

## Scope

- Run the **full** differential corpus across every ported route + plugin.
- `cargo clippy -- -D warnings`, `cargo fmt --check`, and the full feature matrix
  (`cargo hack check --feature-powerset` or the curated CI set).
- Port the docs-site code blocks (TS → Rust) and ship the static-export GitHub Pages deploy.
- Exclude/prune `reference/` from the published crate (already `exclude`d); confirm the
  packaged crate carries no TS.

## Gates

Full differential green; feature matrix green; `cargo publish --dry-run -p better-auth-rs`
and `-p better-auth-rs-core` succeed; docs static export builds.

## Exit criteria

`v0.1.0` tagged and published to crates.io; docs live on GitHub Pages; the published crate
carries no `.ts` (already `exclude`d). The co-located `.ts` stays in the repo permanently — the
1:1 spec and sync baseline; it is never deleted.
