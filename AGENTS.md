# better-auth-rs — project memory

A faithful, file-by-file Rust port of [better-auth](https://github.com/better-auth/better-auth).
Each upstream `.ts` source is **co-located inside the crate `src/`**, right next to where its
`.rs` goes (e.g. `crates/better-auth-rs/src/crypto/password.ts` beside `password.rs`) — "siblings,
not compiled, not shipped". The `.ts` is the **source of truth for intended behavior**; Cargo
ignores it and `exclude = ["**/*.ts"]` keeps it unpublished.

**Keep every `.ts` permanently — NEVER delete a `.ts`, even after its `.rs` is fully ported.** The
co-located `.ts` set is how we know the port is complete (1:1 coverage at a glance), it is the
read-only spec we re-diff against on every upstream sync, and it is the fixture the TS-vs-Rust
differential harness drives. Deleting them loses that. The baseline is pinned in
`port/UPSTREAM_PORTED`, every file tracked in `port/manifest.tsv`. (`reference/better-auth/` holds
the upstream `test/`, `e2e/`, and `LICENSE.md`.) We translate 1:1 into idiomatic async Rust and
prove each port with ported tests + a TS-vs-Rust differential harness.

> `CLAUDE.md` is a symlink to this file. Per-directory `CLAUDE.md` files add local detail
> (e.g. `crates/better-auth-rs/src/CLAUDE.md`).

## Build / test / lint commands

- **Toolchain is pinned** to `1.96.0` (edition 2024) via `rust-toolchain.toml`. Don't fight it.
- **Check (fast inner loop):** `cargo check -p <crate>` — e.g. `cargo check -p better-auth-rs-core`.
- **Test:** `cargo nextest run -p <crate> [filter]`. Never run the whole suite from the repo
  root without a `-p`/filter (slow, and the pre-bash hook blocks it). Fallback: `cargo test -p <crate>`.
- **Format:** `cargo fmt --all`. **Never call `rustfmt` directly** — it ignores the workspace
  edition; CI checks `cargo fmt --all --check` (the pre-bash hook blocks bare `rustfmt`).
- **Lint:** `cargo clippy --workspace --all-targets` (CI runs `-- -D warnings`).
- **Feature matrix:** `cargo hack check --feature-powerset --depth 2` (or the curated set in CI).
- **Porting tooling:** `cargo xtask manifest` (regen the manifest), `cargo xtask sync --to <tag>`
  (upstream diff), `cargo xtask differential` (TS-vs-Rust harness).

## Crate map (Cargo workspace)

- `crates/better-auth-rs-core` — `@better-auth/core`: data model, `DatabaseAdapter` trait,
  OAuth2 protocol, social-provider registry, error codes, plugin/context types. No web/db driver deps.
- `crates/better-auth-rs` — `packages/better-auth`: api/routes, auth, cookies, crypto, db,
  adapters (`sqlx_postgres`, `memory`), `integrations/axum`, `plugins/*`. **The published crate.**
  Everything optional is behind a Cargo feature **named exactly like its upstream plugin/package**
  (`two-factor`, `organization`, `api-key`, `jwt`, `oidc-provider`, …). `axum`/`sqlx-postgres`
  are the only Rust-ecosystem feature names. `default = ["axum","sqlx-postgres"]`.
- `xtask` — porting/sync/differential tooling (std-only, no network).

This repo is a **dual workspace**: Cargo for the Rust library; a pnpm workspace (Node) scoped only
to `docs/` (Fumadocs site) and the vendored TS reference server the differential harness drives.

## The porting contract (read before touching any port)

1. **The reference is the spec.** For any behavior, open the co-located sibling `.ts` (same
   folder as the `.rs`), understand it, then make the Rust match — **bug-for-bug**. Don't
   "fix" apparent upstream bugs during a port; file them, match the behavior. **Keep the `.ts`
   afterward — never delete it** (it stays as the 1:1 spec, the re-sync diff target, and the
   differential-harness fixture).
2. **Idiomatic async Rust.** `tokio` + `async`/`await`; `async-trait` only at `dyn` boundaries.
   (Unlike Bun's Zig→Rust port, we do NOT ban async — better-auth is Promise-based.)
3. **Keep it diffable.** Preserve control flow, ordering, names, and comments close to upstream
   so future upstream diffs re-port cleanly.
4. **Per-file loop:** read `.ts` → write the Rust sibling (path from `manifest.tsv`) →
   `cargo check -p <crate>` → port the matching `*.test.ts` → `cargo nextest run` green →
   update the manifest row (`status` `drafted`→`building`→`done`, `confidence`, `upstream_sha`).
5. **Win condition:** the ported behavior test passes in Rust **and** its differential vector
   matches the live TS server.
6. **Crypto/storage may be idiomatic Rust** (we're schema- and API-compatible, not a drop-in
   binary replacement). Adapt internal-byte-format assertions to behavior assertions.

## What reviewers catch (Rust-adapted)

- **Security:** validate untrusted input before any side effect; fail closed; constant-time
  compares (`subtle`) for tokens/MACs; never leak secrets in `Debug`/logs (wrap in `secrecy`);
  no panics on user-reachable paths.
- **Correctness:** fix the bug *class*, not one instance; one source of truth; every `match`
  arm/branch reachable; verify against the reference, don't guess.
- **Errors:** typed `thiserror` enums in public APIs; never swallow a failure; `?` over `unwrap`.
  `unwrap`/`expect` are clippy-`warn` here — only on provable invariants, with a `// SAFETY:`-style note.
- **Style:** match the neighboring file; reuse existing helpers before writing new ones; delete
  dead code in the same change; comments carry durable, non-obvious intent only.

## Rules

- **Branches start with `claude/`.** Commit messages use **Conventional Commits** (`feat:`,
  `fix:`, `chore:`, `feat(oauth):` …) — the release tooling (release-plz) consumes them.
- **Test everything.** If you didn't run the test, it doesn't work. Don't weaken a test to make it pass.
- **Update `port/manifest.tsv`** whenever you port/advance a file.
- **Be humble and honest** in commits/PRs — never overstate what works.
- **Absolute paths** in tooling; **never edit a co-located `.ts`** (it's the read-only spec) —
  port into the `.rs` sibling and **keep the `.ts`** (never delete it; we keep the codebase 1:1).
