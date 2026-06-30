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
`with-hooks`, `kysely-adapter` (the sea-query analogue), `field-converter`/`get-migration` —
read for the feature shape and behavior, then reimplement idiomatically in Rust.

## What to build

- `DatabaseAdapter` trait: `create / find_one / find_many / count / update / update_many /
  delete / delete_many / transaction`, with `Where { field, value, operator, connector, mode }`
  and `WhereOperator` (eq/ne/lt/lte/gt/gte/in/not_in/contains/starts_with/ends_with).
- `memory-adapter` (no DB) and `sqlx-postgres` adapter (sea-query builds the dynamic SQL,
  sqlx executes; `sea-query-binder` bridges values).

## Gates

Build our own Rust conformance battery (informed by better-auth's `test-utils` suites): normal
CRUD / where-ops / transforms, `authFlow`, joins, uuid, number-id, case-insensitive, transactions;
run against `memory` + `sqlx-postgres` (docker-compose Postgres). These are our behavior tests
(`cargo nextest`), not a differential harness against the TS server.

## Exit criteria

Conformance battery green on both backends; clippy `-D warnings` and fmt clean; migrations create
exactly `[user, session, account, verification]`.
