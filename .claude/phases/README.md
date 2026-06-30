# Porting phases

Each file here is a standalone, executable-by-an-agent document for one phase of the
better-auth → Rust port, in the spirit of Bun's Zig→Rust phase files. Work the phases in
order; each lists its preconditions.

## Shared per-file loop (applies to every phase)

For each file the phase covers (rows in `port/manifest.tsv`):

1. **Read the spec** — open the sibling `.ts` under `reference/better-auth/`. It is the
   source of truth for intended behavior. Port **bug-for-bug**; don't "fix" upstream here.
2. **Write the Rust sibling** at the `rust_path` from the manifest (idiomatic async Rust).
3. `cargo check -p <crate>` until it compiles.
4. **Port the matching test** (`*.test.ts` → Rust) and `cargo nextest run -p <crate> <filter>`
   until green. Adapt internal-byte-format assertions to behavior assertions (crypto/storage
   are idiomatic Rust, not byte-identical).
5. **Update the manifest row:** `status` (`todo`→`drafted`→`building`→`done`), `confidence`
   (`high|med|low`), and `upstream_sha`.
6. **Low-confidence ports** get an adversarial review pass before `done`.

## Conventions

- Branch per phase: `claude/phase-N-<slug>`. Conventional-commit messages.
- A phase may be driven by a `Workflow` script that fans out one agent per manifest file in
  the phase, then adversarially reviews — the Bun `*.workflow.js` analogue.
- Regenerate the manifest anytime with `cargo xtask manifest` (it preserves status/confidence).

## Phases

| Phase | Doc | Outcome |
|---|---|---|
| 0 | `phase-0-scaffolding.md` | Workspace, tooling, vendored reference, `.claude/`, `.github/` |
| 1 | `phase-1-core-model-adapters.md` | Data model + `DatabaseAdapter` + sqlx-postgres/memory + conformance |
| 2 | `phase-2-crypto-cookies-context.md` | Crypto, cookies, `AuthContext` |
| 3 | `phase-3-core-api.md` | Endpoint/router/hook pipeline + core auth routes |
| 4 | `phase-4-axum-integration.md` | Runnable axum server + differential harness |
| 5 | `phase-5-oauth-social.md` | OAuth2 + ~35 social providers |
| 6 | `phase-6-plugins.md` | v1 plugins (RBAC, 2FA/passwordless, tokens) |
| 7 | `phase-7-hardening-parity.md` | Full parity gate, retire TS, tag v0.1 |
| ∞ | `phase-sync-upstream.md` | Full-auto upstream release tracking |
