# Phase 3 — Core API

**Goal:** endpoint/router/hook pipeline + the core auth routes.
**Preconditions:** Phase 2.

## Scope (reference → target)

- `packages/better-auth/src/api/{dispatch,to-auth-endpoints,index,middlewares,rate-limiter,routes}`
  + `packages/core/src/api` → `crates/better-auth-rs/src/api/*`.

## Reference reading

`dispatch.ts` (before/after hook chain, middleware matching, response conversion),
`to-auth-endpoints.ts`, route files (sign-up, sign-in, sign-out, session, password,
email-verification, update-user, account, callback), `middlewares/{origin-check,authorization}`,
`rate-limiter`.

## What to build

- The `better-call`-analogue: an endpoint type + router + before/after hook pipeline.
- Core routes: sign-up, sign-in/email, sign-out, session (get/list/revoke/revoke-others),
  password (change/set/reset/verify), email-verification, update-user.
- Origin/CSRF middleware; per-route rate limiter (`governor`).

## Gates

Ported route tests; the full sign-up → sign-in → session flow works via direct dispatch
against the `memory` adapter (no axum yet).

## Exit criteria

Route tests green.
