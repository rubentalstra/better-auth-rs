# crates/better-auth-rs-core/src — Rust conventions

Local detail for the framework-agnostic core (port of `@better-auth/core`). See the root
`AGENTS.md` for build/test commands and the porting contract, and
`.claude/skills/porting-ts-to-rust` for the TS→Rust cheat-sheet. This file adds only the idioms
specific to *this* crate — it does not restate the contract.

## Layout (mirrors `@better-auth/core/src`)

- `error/` — `BetterAuthError`, `APIError`, `BASE_ERROR_CODES`.
- `types/` — `secret`, `cookie`, `plugin`, `context`, `init-options`, `helper` (the option/context
  vocabulary every consumer depends on).
- `db/` — the data model (`schema/*`), the dynamic value/field types (`type.ts → types.rs`), and
  the adapter machinery (`adapter/`, `get-tables`).
- `utils/` — `id`, `json`, `string`, `host`, `ip`, `url`, `redirect-uri`, `async`, `error-codes`.
- `env/` — logger + terminal color-depth + env-var access.
- `oauth2/` — the OAuth2 *protocol* (authorization-url, validate-code, refresh, verify) — no HTTP
  server surface.
- `context/`, `async_hooks/`, `instrumentation/`, `social_providers/`, `api/`.

## Idioms

- **Driver-light — this is the hard rule for core.** No web framework, no DB driver. Deps are
  only `serde, serde_json, time, thiserror, async-trait` (workspace-pinned). `http`/`cookie`/
  `url`/`secrecy` belong to the main crate; model `better-call` types (`APIError`,
  `CookieOptions`) as **local plain structs** here. Don't add a dep without it being genuinely
  required by the file you're porting.
- **TS type-level inference collapses to nothing at runtime.** `InferDBFields*`, `Prettify`,
  `UnionToIntersection`, `LiteralString`, `ValidateErrorCodes`, and `declare module …`
  plugin-registry augmentations have **no Rust analog**. Port the runtime value/struct/trait
  surface and **document each dropped type-level construct** in the module doc. This is the single
  thing most likely to be done wrong — when a `.ts` import looks like a dependency cycle
  (`db/type.ts ↔ types/init-options.ts`), check whether the import is type-level only; if so it
  drops and the cycle disappears.
- **String-literal unions → closed enums** (e.g. `DBFieldType`, error codes), not stringly-typed
  values. See utils' `ShaFamily` for the pattern.
- **Errors are typed `thiserror` enums/structs** mirroring the upstream code set
  (`BASE_ERROR_CODES` → a closed enum with `const fn code()`/`const fn message()`).
- `async fn` via `tokio`; `async-trait` only where a trait must be `dyn` (`SecondaryStorage`,
  `DatabaseAdapter`). `Awaitable<T>` → plain `T` in an `async fn`.
- Dates → `time::OffsetDateTime`. `Map<number,string>` → `BTreeMap<u32, String>` (deterministic).
- Secrets: no `secrecy` dep in core — use a plain field with a manual redacting `Debug` impl and a
  `// never log` note; document the deviation.

## Module docs & glue

- Every ported `.rs` names its origin: `//! Upstream source: <path>.ts`. The `type`→`types`
  keyword rename (`db/type.ts → db/types.rs`) must be called out in the module doc.
- Where **no `index.ts` exists** (`utils/`, `db/schema/`, and `types/`/`context/` until their
  `index.ts` is ported), the `mod.rs` is Rust-only aggregator glue →
  `//! Rust-only module aggregator (no upstream .ts).`. Where an `index.ts` *does* exist
  (`error/`, `db/`, `types/`, the crate root `index.ts → lib.rs`), `mod.rs`/`lib.rs` **is** the
  port of that barrel and gets an origin line.

## Tests

Co-located `<stem>.test.rs` child modules (so they reach private items), header
`#![allow(clippy::unwrap_used, clippy::expect_used)]`, wired from the source with
`#[cfg(test)] #[path = "<stem>.test.rs"] mod <stem>_tests;`. Every upstream `it(`/`test(` case
gets a Rust equivalent. Core has no HTTP surface, so the differential harness is deferred — prove
pure-logic files (`host`, `ip`, `url`, `string`, oauth2 helpers, provider configs) with **golden
input→output vectors harvested from the upstream TS test suite** (`reference/better-auth/test/`),
baked in as literal Rust assertions. The gold-standard idiom to copy is
`crates/better-auth-rs-utils/src/hex.rs` + `hex.test.rs`.
