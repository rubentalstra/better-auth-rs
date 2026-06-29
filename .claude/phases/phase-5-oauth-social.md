# Phase 5 — OAuth2 + social providers

**Goal:** the OAuth2 protocol layer + ~35 social providers.
**Preconditions:** Phase 4.

## Scope (reference → target)

- `packages/better-auth/src/oauth2` + `packages/core/src/oauth2` → core oauth2.
- `packages/core/src/social-providers/*` (~35 providers) → `crates/better-auth-rs-core/src/social_providers`.
- callback route + account linking in the main crate.

## Reference reading

`oauth2/{create-authorization-url,validate-authorization-code,refresh-access-token,
client-credentials-token,verify,oauth-provider,reject-redirects,utils}`, `state.ts`,
`link-account.ts`, each `social-providers/*.ts`.

## What to build

- OAuth2 flows on `openidconnect`/`oauth2` (auth URL, code exchange, refresh, PKCE,
  encrypted state, open-redirect rejection).
- Provider registry as data; per-provider profile normalization (OIDC providers via discovery;
  non-OIDC via custom profile fetch with `reqwest`).
- `/callback/{provider}`, account linking, token storage.

## Gates

Ported oauth tests; differential OAuth flows with mocked provider endpoints (the e2e-smoke
MSW analogue, mocked in Rust).

## Exit criteria

Social sign-in matches the TS server for representative OIDC + non-OIDC providers.
