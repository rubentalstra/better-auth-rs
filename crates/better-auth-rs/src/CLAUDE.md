# crates/better-auth-rs/src — Rust conventions

Local detail for the main crate. See the root `AGENTS.md` for build/test commands and the
porting contract, and `.claude/skills/porting-ts-to-rust` for the TS→Rust cheat-sheet.

## Layout (mirrors `packages/better-auth/src`)

- `api/` — endpoint type, router, before/after hook pipeline (`dispatch`), `routes/`,
  `middlewares/`, `rate_limiter/`.
- `auth/` — the `betterAuth`-equivalent builder + instance wiring.
- `cookies/`, `crypto/` — signed/encrypted cookies; password/HMAC/AEAD/random.
- `db/` — the default DB layer + `with_hooks` lifecycle; `adapters/{sqlx_postgres, memory}`.
- `integrations/axum.rs` — `tower::Service` mount (behind `axum`).
- `plugins/<name>/` — one module per upstream plugin, behind the `<name>` feature.

## Idioms

- Public API errors are typed `thiserror` enums mirroring upstream `*_ERROR_CODES`; return
  `Result<_, _>` and use `?`. `unwrap`/`expect` are clippy-`warn` (provable invariants only).
- Async via `tokio`; `async-trait` only where a trait must be `dyn` (adapters, plugin hooks).
- Inputs: `serde` structs + `garde` validation (the zod replacement). Datetime = `time`.
- Everything optional is feature-gated by the **upstream plugin/package name**; keep the core
  compiling with `--no-default-features`.
- Constant-time compares (`subtle`) for secrets/tokens; wrap secrets in `secrecy`; never log them.

## Tests

Co-locate tests with the module they cover. Prove behavior, not internal byte formats. Add a
differential vector for each externally observable behavior (see `writing-parity-tests`).
