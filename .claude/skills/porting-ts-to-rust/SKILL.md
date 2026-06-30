---
name: porting-ts-to-rust
description: How to translate a better-auth TypeScript source file into idiomatic async Rust for better-auth-rs. Use when porting any co-located .ts spec in crates/*/src/, or when unsure how a TS construct maps to Rust.
---

# Porting better-auth TS → idiomatic async Rust

The **co-located sibling `.ts`** (same folder as the `.rs` you're writing) is the **design
reference** — better-auth's source is vendored right next to where each `.rs` goes. Read it to
understand the feature and how better-auth behaves, then **reimplement it idiomatically and securely
in Rust**, built on mature crates (see the crates.io rule below). Match observable *feature
behavior*, not byte formats or internals; do **not** copy upstream bugs or weaker security choices —
do the secure, correct thing and note the deviation. Find the file's `rust_path` in
`port/manifest.tsv`. **Keep the `.ts` permanently — never edit or delete it** (it is the coverage
map of what's reimplemented, the re-sync reference, and the source of test cases worth porting; see
AGENTS.md).

## Per-file loop

read `.ts` → write the Rust sibling → `cargo check -p <crate>` → port the matching `*.test.ts`
→ `cargo nextest run -p <crate> <filter>` green → update the manifest row (status/confidence/upstream_sha).

## Construct → Rust mapping

| TypeScript (better-auth) | Rust |
|---|---|
| `async function` / `Promise<T>` | `async fn -> T` (tokio); `async-trait` only at `dyn` boundaries |
| `zod` schema + `.parse()` | a `serde` struct (`#[derive(Deserialize)]`) + `garde` validation derive |
| `throw new APIError(status, { code })` | `Err(ApiError { status, code, .. })` (typed `thiserror` enum) |
| `BetterAuthError`, `*_ERROR_CODES` | a Rust error enum mirroring the upstream code set |
| Kysely query builder | `sea-query` builder (dynamic `Where[]`→SQL), executed by `sqlx` |
| `DBAdapter` object | the `DatabaseAdapter` trait (`create/find_one/find_many/...`) |
| `ctx.json(...)`, endpoint handler | the endpoint/hook pipeline in `api/` (Phase 3) |
| `createAuthEndpoint(path, opts, handler)` | the Rust endpoint constructor (Phase 3) |
| object spread / optional fields | structs + `Option<T>`; builder for option bags |
| `Date` | `time::OffsetDateTime` (the workspace datetime stack) |
| `crypto.subtle` / node:crypto | RustCrypto crates (`hmac`,`sha2`,`scrypt`,`aes-gcm`,…) |
| JS `Record<string,unknown>` | `serde_json::Value` or a typed struct where the shape is known |

## Rules

- **Check crates.io first; don't reinvent the wheel.** When a TS construct maps to a standard
  primitive — HTTP types, cookies, URLs, dates/durations, encodings, crypto, JSON — reach for the
  mature ecosystem crate (`http`, `cookie`, `url`, `time`, RustCrypto, `serde_json`, …) instead of
  hand-rolling. Search crates.io and prefer the widely-used, maintained standard. Model external
  types (e.g. `better-call`'s `APIError` → `http::StatusCode`/`HeaderMap`; `CookieOptions` →
  `cookie::SameSite`) on those crates. Hand-roll only when nothing suitable exists; note why.
- No `unwrap`/`expect` on fallible IO/user input (clippy-`warn` here); use `?` + typed errors.
- Constant-time compare (`subtle`) for tokens/MACs; wrap secrets in `secrecy`.
- Crypto/storage may be idiomatic Rust (we're schema/API-compatible, not byte-identical) —
  adapt internal-format test assertions to behavior assertions.
- Reuse existing helpers (`crates/better-auth-rs-core`) before writing new ones.
- Never edit `reference/`. Never invent behavior that isn't in the `.ts`.
