//! Upstream reference: types/helper.ts
//!
//! Pure compile-time TypeScript type utilities with **no runtime analog** — they all collapse in
//! Rust, so this module is intentionally empty (recorded for 1:1 traceability):
//!
//! - `Awaitable<T> = T | Promise<T>` → just `T` (futures are `.await`ed; there is no value-level
//!   "maybe-a-promise" type).
//! - `AwaitableFunction<T>` → a plain value or an `Fn`/`async fn` at the use site.
//! - `LiteralString` / `LiteralUnion<L, B>` → `String`/`&str`, or a closed `enum` where the set of
//!   literals is fixed, at the use site.
//! - `Primitive`, `Prettify<T>`, `UnionToIntersection<U>` → type-level only; nothing is emitted.
