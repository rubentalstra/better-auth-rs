---
name: axum-integration
description: How the framework-agnostic better-auth-rs core is mounted into axum (and how to add other frameworks later). Use when working on crates/better-auth-rs/src/integrations or wiring the server example.
---

# axum integration

The core is framework-agnostic: it operates on `http::Request<Body>`/`http::Response<Body>` and
implements `tower::Service`. The `axum` feature provides a thin mount; other frameworks can add
their own thin adapter later without touching the core.

## Mounting

- Build the auth service from options (db adapter, secret, enabled plugins).
- Because the service is `tower::Service<http::Request<Body>>`, mount it under a path with
  `Router::nest_service("/api/auth", auth_service)`.
- A framework that isn't tower-native (e.g. actix/poem, future) gets a small adapter converting
  its native req/res to/from `http` types — the core stays unchanged.

## Example

`examples/axum_server.rs` wires `sqlx-postgres` + email/password and serves on `127.0.0.1:0`
(or an env port). It's the smoke target for the differential harness and manual `curl` checks.

## Rules

- Keep all axum/tower deps behind the `axum` feature so the core compiles framework-free.
- No business logic in the integration layer — it only adapts transport.
