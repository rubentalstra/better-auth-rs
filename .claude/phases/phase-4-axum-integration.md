# Phase 4 — axum integration (runnable end-to-end)

**Goal:** a real running server, and stand up the differential harness.
**Preconditions:** Phase 3.

## Scope (reference → target)

- `packages/better-auth/src/integrations/node.ts` (pattern) → `crates/better-auth-rs/src/integrations/axum.rs`.
- `examples/axum_server.rs`.
- `tests/differential/` harness + `cargo xtask differential`.

## What to build

- Expose the core as `tower::Service<http::Request<Body>>`; mount via `Router::nest_service`
  behind the `axum` feature.
- A minimal runnable example wiring `sqlx-postgres` + email/password.
- **Differential harness:** boot the vendored TS reference server (Node, port 0) and the Rust
  server; replay request vectors; assert identical status / JSON body / cookie semantics
  (normalize dynamic ids/timestamps/tokens).

## Gates

Example server boots; differential harness green on the Phase-3 vectors.

## Exit criteria

sign-up → sign-in → session over HTTP matches the TS server (observable behavior).
