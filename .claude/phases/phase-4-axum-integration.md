# Phase 4 — axum integration (runnable end-to-end)

**Goal:** a real running server, proven by HTTP behavior tests.
**Preconditions:** Phase 3.

## Scope (reference → target)

- `packages/better-auth/src/integrations/node.ts` (design reference) → `crates/better-auth-rs/src/integrations/axum.rs`.
- `examples/axum_server.rs`.
- `tests/http/` end-to-end behavior tests driving the running server.

## What to build

- Expose the core as `tower::Service<http::Request<Body>>`; mount via `Router::nest_service`
  behind the `axum` feature.
- A minimal runnable example wiring storage via the `sqlx-postgres` adapter crate + email/password.
- **HTTP behavior tests:** boot the Rust server (port 0), drive it over HTTP, and assert correct,
  secure status / JSON body / cookie semantics for the auth flows (our own Rust tests, not a
  TS-vs-Rust comparison).

## Gates

Example server boots; HTTP behavior tests green for the Phase-3 flows.

## Exit criteria

sign-up → sign-in → session works correctly and securely over HTTP, covered by passing Rust tests.
