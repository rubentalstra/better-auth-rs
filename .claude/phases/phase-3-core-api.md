# Phase 3 — Core API

**Goal:** endpoint/router/hook pipeline + the core auth routes.
**Preconditions:** Phase 2.

## Scope (design reference → target)

- `packages/better-auth/src/api/{dispatch,to-auth-endpoints,index,middlewares,rate-limiter,routes}`
  + `packages/core/src/api` inform `crates/better-auth-rs/src/api/*`.

## Design-reference reading

`dispatch.ts` (before/after hook chain, middleware matching, response conversion),
`to-auth-endpoints.ts`, route files (sign-up, sign-in, sign-out, session, password,
email-verification, update-user, account, callback), `middlewares/{origin-check,authorization}`,
`rate-limiter`. Read these to understand the feature surface and behavior, then reimplement
idiomatically and securely in Rust — do not copy them line-for-line.

## What to build

- The `better-call`-analogue: an endpoint type + router + before/after hook pipeline.
- Core routes: sign-up, sign-in/email, sign-out, session (get/list/revoke/revoke-others),
  password (change/set/reset/verify), email-verification, update-user.
- Origin/CSRF middleware; per-route rate limiter (`governor`).

## Gates

Rust behavior tests for the routes; the full sign-up → sign-in → session flow works via
direct dispatch against the `memory` adapter (no axum yet).

## Exit criteria

Route tests green (cargo nextest), clippy `-D warnings`, fmt clean.
