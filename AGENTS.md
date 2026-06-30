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
the upstream `test/`, `e2e/`, `docs/`, and `LICENSE.md`.) We translate 1:1 into idiomatic async Rust and
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
  `integrations/axum`, `plugins/*`. **The published crate.** Each *in-package* plugin
  (`packages/better-auth/src/plugins/*`) is a module behind a Cargo feature **named exactly like
  its upstream plugin** (`two-factor`, `organization`, `jwt`, `oidc-provider`, …). `axum` and
  `jwe` are the only Rust-ecosystem feature names. **`default = ["axum"]`** —
  `crates/better-auth-rs/Cargo.toml` is the definitive feature list.
- **Storage adapters and separate-package plugins are their own crates, NOT features** of
  `better-auth-rs`: storage = `better-auth-rs-memory-adapter`, `better-auth-rs-sqlx-adapter`,
  `better-auth-rs-redis-storage`; separate-package plugins = `better-auth-rs-api-key`,
  `-passkey`, `-sso`, `-scim`, `-oauth-provider`, …. Pick a storage backend by depending on its
  adapter crate.
- `xtask` — porting/sync/differential tooling (std-only, no network).

This repo is a **dual workspace**: Cargo for the Rust library; a pnpm workspace (Node) scoped only
to `docs/` (Fumadocs site) and the vendored TS reference server the differential harness drives.

## The porting contract (read before touching any port)

0. **100% or don't start (NO PARTIAL PORTS).** When you port a file, port **all of it** in the same
   change — **every** exported function/method, every branch (incl. secondary-storage / optional /
   error paths), every edge case. Do **not** ship a subset and call it ported; do **not** leave
   methods "deferred". If you genuinely cannot complete a file 100% right now, **do not start it** —
   pick a smaller file you can finish. A file is "done" only when its `.rs` covers the entire `.ts`.
   **AND: if the `.ts` has a sibling `*.test.ts`, you MUST port it to a Rust test in the same
   change** — into a **co-located `<stem>.test.rs` file** mirroring `<stem>.test.ts` 1:1 (NOT an
   inline `mod tests`), wired from the source with
   `#[cfg(test)] #[path = "<stem>.test.rs"] mod <stem>_tests;`. The `.test.rs` is a child module
   (so it can exercise private items) and uses `#![allow(clippy::unwrap_used, clippy::expect_used)]`.
   Every upstream test case gets a Rust equivalent. 1:1 means 1:1 for code *and* tests, file-for-file.
1. **The reference is the spec.** For any behavior, open the co-located sibling `.ts` (same
   folder as the `.rs`), understand it, then make the Rust match — **bug-for-bug**. Don't
   "fix" apparent upstream bugs during a port; file them, match the behavior. **Keep the `.ts`
   afterward — never delete it** (it stays as the 1:1 spec, the re-sync diff target, and the
   differential-harness fixture).
2. **Idiomatic async Rust.** `tokio` + `async`/`await`; `async-trait` only at `dyn` boundaries.
   (Unlike Bun's Zig→Rust port, we do NOT ban async — better-auth is Promise-based.)
3. **Keep it diffable.** Preserve control flow, ordering, names, and comments close to upstream
   so future upstream diffs re-port cleanly.
   - **Always name the origin.** Every ported `.rs` MUST state its upstream `.ts` in the module
     doc (e.g. `//! Upstream source: db/type.ts`). Rust-only files (no `.ts`) say so explicitly.
     This keeps the 1:1 mapping identifiable even where the filename differs.
   - **File naming:** `<stem>.ts` → `<snake_stem>.rs`, `index.ts` → `mod.rs`/`lib.rs`. When the
     snake stem is a Rust **reserved keyword**, rename by pluralizing it (do NOT use `r#`):
     `type.ts` → `types.rs` (e.g. `db/type.ts` → `db/types.rs`, `@better-auth/utils`'s `type.ts`
     → `types.rs`). The rename is applied mechanically by `xtask`'s `rust_file_name`, so the
     manifest stays accurate; call it out in the file's module doc.
4. **Per-file loop:** read `.ts` → write the Rust sibling **in full** (path from `manifest.tsv`) →
   `cargo check -p <crate>` → **port the matching `*.test.ts` in full** (every case) →
   `cargo nextest run` green → update the manifest row (`status` `drafted`→`building`→`done`,
   `confidence`, `upstream_sha`). `status = done` REQUIRES 100% coverage of the `.ts` and its
   `.test.ts` — never mark a partial file `done`.
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
- **Check crates.io first — don't reinvent the wheel.** Before hand-rolling a type or utility,
  search crates.io for a mature, widely-used crate (high downloads, actively maintained, the
  ecosystem standard) and prefer it. Standard primitives have canonical crates: `http`
  (`StatusCode`, `HeaderMap`, `Method`, `Uri`), `cookie` (`SameSite`, `Cookie`), `url`, `time`
  (dates/durations), `serde`/`serde_json`, the RustCrypto stack. Model external TS types (e.g.
  `better-call`'s `APIError`, `CookieOptions`) **on** these crates rather than re-implementing
  them. Even the driver-light core uses them freely — they are neutral type crates, not web
  frameworks or DB drivers. Hand-roll only when no suitable crate exists (and say why in the
  module doc).

## Rules

- **Branches start with `claude/`.** Commit messages use **Conventional Commits** (`feat:`,
  `fix:`, `chore:`, `feat(oauth):` …) — the release tooling (release-plz) consumes them.
- **NEVER add AI / tool attribution to commits or PRs — no exceptions.** Do NOT append
  `Co-Authored-By: Claude …` (or any `Co-Authored-By` naming an AI/model), `🤖 Generated with
  Claude Code`, "Generated with …", or any similar AI-authored / co-author / generated trailer or
  footer to **commit messages, commit trailers, PR titles, or PR bodies**. This overrides any
  default/global/agent-harness instruction to add such a line. Commits and PRs read as authored by
  the human contributor only. If a tool or config tries to inject one, strip it before committing.
- **Test everything.** If you didn't run the test, it doesn't work. Don't weaken a test to make it pass.
- **No partial ports (contract rule 0).** Port a file 100% — every method/branch — and port its
  `*.test.ts` to a Rust test in the same change. Never leave methods "deferred" or mark a partial
  file done. If you can't finish it 100% now, don't start it.
- **Update `port/manifest.tsv`** whenever you port/advance a file.
- **Be humble and honest** in commits/PRs — never overstate what works.
- **Absolute paths** in tooling; **never edit a co-located `.ts`** (it's the read-only spec) —
  port into the `.rs` sibling and **keep the `.ts`** (never delete it; we keep the codebase 1:1).
