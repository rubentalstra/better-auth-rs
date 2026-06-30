# Phase 1 — Core model + adapters

**Goal:** data model + storage abstraction + first backend, with our own conformance battery as the spec.
**Preconditions:** Phase 0.

## Scope (design reference → target)

- `packages/core/src/db/{schema,adapter}` → `crates/better-auth-rs-core/src/db`
- `packages/better-auth/src/db`, `packages/better-auth/src/adapters` →
  `crates/better-auth-rs/src/{db, adapters/{sqlx_postgres, memory}}`
- Postgres migrations.

## Design reference reading

db schema (`user`/`session`/`account`/`verification`), `DBAdapter` types, `internal-adapter`,
`with-hooks`, the TS ORM adapters (`prisma-adapter`/`drizzle-adapter`/`kysely-adapter` — whose Rust
analogues are Diesel / SeaORM / SQLx, see below), `field-converter`/`get-migration` — read for the
feature shape and behavior, then reimplement idiomatically in Rust.

## What to build

- `DatabaseAdapter` trait: `create / find_one / find_many / count / update / update_many /
  delete / delete_many / transaction`, with `Where { field, value, operator, connector, mode }`
  and `WhereOperator` (eq/ne/lt/lte/gt/gte/in/not_in/contains/starts_with/ends_with).
- Storage adapters are **separate crates**, each implementing the `CustomAdapter` contract:
  - `better-auth-rs-memory-adapter` — in-memory, no DB (tests/dev).
  - `better-auth-rs-sqlx-adapter` — SQLx (Postgres first); dynamic SQL via `sea-query` +
    `sea-query-binder`.
  - `better-auth-rs-diesel-adapter` — the [Diesel](https://diesel.rs) ORM.
  - `better-auth-rs-seaorm-adapter` — the [SeaORM](https://www.sea-ql.org/SeaORM/) ORM.
  - `better-auth-rs-redis-storage` — `SecondaryStorage` (sessions / rate-limit), not a full adapter.

  These are the Rust analogues of better-auth's TS ORM adapters: **Prisma / Drizzle / Kysely →
  Diesel / SeaORM / SQLx**; MongoDB → a future `better-auth-rs-mongodb-adapter`. We reuse mature,
  industry-standard ORMs rather than reimplementing a query layer.

## Gates

Build our own Rust conformance battery (informed by better-auth's `test-utils` suites): normal
CRUD / where-ops / transforms, `authFlow`, joins, uuid, number-id, case-insensitive, transactions;
run against `memory` + `sqlx-postgres` (docker-compose Postgres). These are our behavior tests
(`cargo nextest`), not a differential harness against the TS server.

## Exit criteria

Conformance battery green on both backends; clippy `-D warnings` and fmt clean; migrations create
exactly `[user, session, account, verification]`.
