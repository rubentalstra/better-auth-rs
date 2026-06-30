# Porting phases

Each file here is a standalone, executable-by-an-agent document for one phase of the
better-auth-inspired Rust reimplementation, in the spirit of Bun's Zig→Rust phase files. Work
the phases in order; each lists its preconditions.

## Shared per-file loop (applies to every phase)

For each file the phase covers (rows in `port/manifest.tsv`):

1. **Read the design reference** — open the **co-located sibling `.ts`** in `crates/*/src/` (same
   folder as the `.rs` you're writing). It documents what feature to build and how better-auth
   behaves. Reimplement it **idiomatically and securely** on audited crates — don't copy upstream
   bugs or weaker choices. Keep the `.ts` — never edit or delete it.
2. **Write the Rust sibling** at the `rust_path` from the manifest (idiomatic async Rust).
3. `cargo check -p <crate>` until it compiles.
4. **Write our own Rust behavior tests** (translating the useful cases from any sibling `*.test.ts`)
   and `cargo nextest run -p <crate> <filter>` until green. Assert on behavior, not internal byte
   formats (crypto/storage are idiomatic Rust, built on audited crates).
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
| 0 | `phase-0-scaffolding.md` | Workspace, tooling, `.ts` design references, `.claude/`, `.github/` |
| 1 | `phase-1-core-model-adapters.md` | Data model + `DatabaseAdapter` + sqlx-postgres/memory + conformance |
| 2 | `phase-2-crypto-cookies-context.md` | Crypto, cookies, `AuthContext` |
| 3 | `phase-3-core-api.md` | Endpoint/router/hook pipeline + core auth routes |
| 4 | `phase-4-axum-integration.md` | Runnable axum server + HTTP behavior tests |
| 5 | `phase-5-oauth-social.md` | OAuth2 + ~35 social providers |
| 6 | `phase-6-plugins.md` | v1 plugins (RBAC, 2FA/passwordless, tokens) |
| 7 | `phase-7-hardening-parity.md` | Full hardening + test gate, ship without bundling TS, tag v0.1 |
| ∞ | `phase-sync-upstream.md` | Full-auto upstream release tracking |
