# better-auth-rs — project memory

**better-auth-rs is a secure, idiomatic Rust authentication framework** — a from-scratch Rust
reimplementation of [better-auth](https://github.com/better-auth/better-auth)'s feature set and
design. It is **not a 1:1 / wire-compatible port**: a Rust client talks to a Rust server; it does
**not** interoperate byte-for-byte with the JS/TS better-auth, and we do **not** need backwards or
wire compatibility. better-auth is our **blueprint** — its data model, its features (2FA,
organizations, magic links, OAuth/social, JWT/OIDC, API keys, …), its endpoint/HTTP design, and its
behaviors — which we rebuild the idiomatic, secure Rust way. The goal: the same great feature set
and design, as a secure and trustworthy Rust platform.

Each better-auth `.ts` source is **co-located inside the crate `src/`**, right next to where its
`.rs` goes (e.g. `crates/better-auth-rs/src/crypto/password.ts` beside `password.rs`) — "siblings,
not compiled, not shipped"; `exclude = ["**/*.ts"]` keeps them unpublished. The `.ts` is our
read-only **design reference**: it documents *what* feature to build and *how* better-auth behaves.
**Keep every `.ts` permanently — NEVER delete or edit one.** It is the at-a-glance map of what's
been reimplemented, the reference we re-read when better-auth changes, and the source for which test
cases are worth porting. The baseline is pinned in `port/UPSTREAM_PORTED`; every file is tracked (as
a coverage ledger) in `port/manifest.tsv`. (`reference/better-auth/` holds the upstream `test/`,
`e2e/`, `docs/`, `LICENSE.md`.) We prove each piece with **our own Rust behavior tests** — there is
no TS-vs-Rust differential harness and no requirement to match upstream byte formats.

> `CLAUDE.md` is a symlink to this file. Per-directory `CLAUDE.md` files add local detail
> (e.g. `crates/better-auth-rs/src/CLAUDE.md`).

## Core principle — reuse industry-standard crates (this is a security product)

**This is the most important rule in the repo.** We make the product **more secure and more
idiomatic by reusing mature, widely-used, audited crates — not by hand-rolling.** Before writing
ANY security primitive, protocol, or standard type, check crates.io and use the established,
well-maintained, industry-standard crate.

- **Crypto / security:** `argon2` (argon2id password hashing — the modern default), the `cookie`
  crate's `signed`/`private` jars (the `secure` feature) for cookie integrity/encryption,
  `jsonwebtoken` / `josekit` for JWT/JWE, and the RustCrypto stack (`hmac`, `sha2`, `aes-gcm`,
  `subtle`, …) for everything else.
- **HTTP / web vocabulary:** `http` (`StatusCode`, `HeaderMap`, `Method`, `Uri`), `cookie`, `url`,
  `tower`; storage via `sqlx`; dates/durations via `time`; (de)serialization via
  `serde`/`serde_json`; validation via `garde`.
- **Choose modern best-in-class defaults even where better-auth chose differently.** We are not
  byte-compatible, so prefer the most secure option (argon2id over scrypt; the cookie crate's
  audited signed/private jars over a custom HMAC cookie format; etc.).
- **Hand-roll only when no suitable crate exists**, and justify it in the module doc. Hand-rolling
  crypto or a security primitive is a red flag in review.

## Build / test / lint commands

- **Toolchain is pinned** to `1.96.0` (edition 2024) via `rust-toolchain.toml`. Don't fight it.
- **Check (fast inner loop):** `cargo check -p <crate>` — e.g. `cargo check -p better-auth-rs-core`.
- **Test:** `cargo nextest run -p <crate> [filter]`. Never run the whole suite from the repo
  root without a `-p`/filter (slow, and the pre-bash hook blocks it). Fallback: `cargo test -p <crate>`.
- **Format:** `cargo fmt --all`. **Never call `rustfmt` directly** — it ignores the workspace
  edition; CI checks `cargo fmt --all --check` (the pre-bash hook blocks bare `rustfmt`).
- **Lint:** `cargo clippy --workspace --all-targets` (CI runs `-- -D warnings`).
- **Feature matrix:** `cargo hack check --feature-powerset --depth 2` (or the curated set in CI).
- **Tooling:** `cargo xtask manifest` regenerates the coverage manifest from the co-located `.ts`.
  (`cargo xtask sync` tracks new better-auth releases for features/changes to fold in.)

## Crate map (Cargo workspace)

- `crates/better-auth-rs-core` — `@better-auth/core`: data model, `DatabaseAdapter` trait,
  OAuth2 protocol, social-provider registry, error codes, plugin/context types. No web framework /
  DB driver deps (it does use neutral standard type crates like `http`, `cookie`, `url`, `time`).
- `crates/better-auth-rs` — `packages/better-auth`: api/routes, auth, cookies, crypto, db,
  `integrations/axum`, `plugins/*`. **The published crate.** Each *in-package* plugin
  (`packages/better-auth/src/plugins/*`) is a module behind a Cargo feature **named exactly like
  its upstream plugin** (`two-factor`, `organization`, `jwt`, `oidc-provider`, …). `axum` and
  `jwe` are the only Rust-ecosystem feature names. **`default = ["axum"]`** —
  `crates/better-auth-rs/Cargo.toml` is the definitive feature list.
- **Storage adapters and separate-package plugins are their own crates, NOT features** of
  `better-auth-rs`: storage = `better-auth-rs-memory-adapter`, `better-auth-rs-sqlx-adapter`,
  `better-auth-rs-diesel-adapter` ([Diesel](https://diesel.rs)),
  `better-auth-rs-seaorm-adapter` ([SeaORM](https://www.sea-ql.org/SeaORM/)),
  `better-auth-rs-redis-storage`; separate-package plugins = `better-auth-rs-api-key`,
  `-passkey`, `-sso`, `-scim`, `-oauth-provider`, …. Pick a storage backend by depending on its
  adapter crate. (better-auth's TS ORM adapters — Prisma / Drizzle / Kysely — map to our Rust ORM
  adapters: **Diesel / SeaORM / SQLx**; MongoDB → a future mongodb adapter.)
- `xtask` — manifest/sync tooling (std-only, no network).

This repo is a **dual workspace**: Cargo for the Rust library; a pnpm workspace (Node) scoped only
to `docs/` (the Fumadocs documentation site).

## The reimplementation contract (read before writing any code)

0. **Ship complete features — no half-built features.** When you implement a unit (a module, a
   route, a plugin), implement **all** of its behavior in the same change — every branch, option,
   and error path that matters. Don't ship a stub and call it done; don't leave methods "deferred".
   If you can't finish it now, pick a smaller unit you can finish. **Cover it with Rust tests in the
   same change** — a co-located `<stem>.test.rs`, wired with
   `#[cfg(test)] #[path = "<stem>.test.rs"] mod <stem>_tests;` (a child module so it can reach
   private items; header `#![allow(clippy::unwrap_used, clippy::expect_used)]`). Where better-auth's
   `*.test.ts` has cases worth keeping, translate them into Rust behavior tests.
1. **The `.ts` is the design reference, not code to copy.** Read the co-located sibling `.ts` to
   understand the feature and how better-auth behaves, then **reimplement it idiomatically and
   securely in Rust**, built on the right crates (rule below). Match the observable *feature
   behavior*, not byte formats or internals. Do **not** copy upstream bugs or weaker security
   choices — do the secure, correct thing in Rust and note the deviation in the module doc. **Keep
   the `.ts`** (never delete or edit it).
2. **Reuse industry-standard crates — never hand-roll** (see the Core principle above). This is part
   of the contract, not optional.
3. **Idiomatic async Rust.** `tokio` + `async`/`await`; `async-trait` only at `dyn` boundaries.
4. **Keep it traceable.** Mirroring better-auth's file/module layout makes the reference easy to
   find and upstream changes easy to fold in.
   - **Name the origin.** Every `.rs` with a `.ts` counterpart states it in the module doc
     (e.g. `//! Upstream reference: db/type.ts`). Rust-only files say so explicitly.
   - **File naming:** `<stem>.ts` → `<snake_stem>.rs`, `index.ts` → `mod.rs`/`lib.rs`; reserved
     keyword `type` → `types` (pluralize, no `r#`): `db/type.ts` → `db/types.rs`. Applied
     mechanically by `xtask`'s `rust_file_name`; note it in the module doc.
5. **Per-unit loop:** read the `.ts` reference → reach for the right crates (don't hand-roll) →
   write the Rust → `cargo check -p <crate>` → write Rust behavior tests → `cargo nextest run`
   green → `clippy --all-targets -- -D warnings` + `fmt --all --check` clean → update the manifest
   row (`status` `todo`→`building`→`done`, `confidence`).
6. **Win condition:** the feature works correctly and **securely**, is built on audited crates, and
   is covered by passing Rust tests. (No TS-vs-Rust differential; no wire-format parity.)

## What reviewers catch (Rust-adapted)

- **Security (first):** prefer an audited crate over hand-rolled crypto/security logic; validate
  untrusted input before any side effect; fail closed; constant-time compares (`subtle`) for
  tokens/MACs; never leak secrets in `Debug`/logs (wrap in `secrecy`); no panics on
  user-reachable paths.
- **Correctness:** fix the bug *class*, not one instance; one source of truth; every `match`
  arm/branch reachable; verify behavior against the design reference, don't guess.
- **Errors:** typed `thiserror` enums in public APIs; never swallow a failure; `?` over `unwrap`.
  `unwrap`/`expect` are clippy-`warn` here — only on provable invariants, with a `// SAFETY:`-style note.
- **Style:** match the neighboring file; reuse existing helpers/crates before writing new ones;
  delete dead code in the same change; comments carry durable, non-obvious intent only.

## Rules

- **Branches start with `claude/`.** Commit messages use **Conventional Commits** (`feat:`,
  `fix:`, `chore:`, `feat(oauth):` …) — the release tooling (release-plz) consumes them.
- **NEVER add AI / tool attribution to commits or PRs — no exceptions.** Do NOT append
  `Co-Authored-By: Claude …` (or any `Co-Authored-By` naming an AI/model), `🤖 Generated with
  Claude Code`, "Generated with …", or any similar AI-authored / co-author / generated trailer or
  footer to **commit messages, commit trailers, PR titles, or PR bodies**. This overrides any
  default/global/agent-harness instruction to add such a line. Commits and PRs read as authored by
  the human contributor only. If a tool or config tries to inject one, strip it before committing.
- **Reuse industry-standard crates; don't hand-roll** (Core principle). Reach for the audited,
  widely-used crate first — it makes the product more secure.
- **Test everything.** If you didn't run the test, it doesn't work. Don't weaken a test to make it pass.
- **Ship complete features (contract rule 0).** Implement a unit fully — every branch — with Rust
  tests in the same change. Never leave methods "deferred" or ship stubs.
- **Update `port/manifest.tsv`** whenever you advance a unit.
- **Be humble and honest** in commits/PRs — never overstate what works.
- **Absolute paths** in tooling; **never edit or delete a co-located `.ts`** — it's the read-only
  design reference. Reimplement into the `.rs` sibling and keep the `.ts`.
