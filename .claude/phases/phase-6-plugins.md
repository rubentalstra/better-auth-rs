# Phase 6 — v1 plugins

**Goal:** the v1 plugin set, in priority order. Each is a feature-gated module named exactly
like its upstream plugin.
**Preconditions:** Phase 5.

## Order & scope (reference → target)

1. **RBAC / multi-tenant:** `access` → `organization` → `admin`
2. **2FA / passwordless:** `two-factor`, `magic-link`, `email-otp`, `phone-number`
3. **Tokens / machine-auth:** `jwt` (+ JWKS), `bearer`, `api-key`, `oidc-provider`

Reference: `packages/better-auth/src/plugins/<name>` (+ `packages/api-key/src`).
Target: `crates/better-auth-rs/src/plugins/<name>` behind the `<name>` Cargo feature.

## Per-plugin loop

Port the plugin's endpoints/hooks/schema → wire its feature (and any deps: `jwt` →
`jsonwebtoken`+`jose-jwk`; `oidc-provider` → `openidconnect`) → port its tests → add differential
vectors → run e2e server-behavior smoke where relevant (e.g. api-key SSR).

## Gates

Per-plugin ported tests + differential vectors green; `cargo check` with that feature on/off.

## Exit criteria

Each v1 plugin's tests + differential pass; `cargo check` across the feature matrix is green.
