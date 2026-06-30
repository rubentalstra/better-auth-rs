---
name: db-adapter-conformance
description: The executable spec a better-auth-rs DatabaseAdapter must satisfy — our own conformance battery (informed by better-auth's test-utils adapter suites). Use when implementing or testing a storage backend (memory, sqlx/postgres, future mysql/sqlite/mongo).
---

# DB adapter conformance

Upstream `packages/test-utils/src/adapter` is the reference for the behavioral contract every
adapter must satisfy. We implement our own Rust trait-based conformance battery (in
`better-auth-rs-test-utils`) and run it against each backend.

## Suites to port (each is a behavior the adapter must satisfy)

- **normal** — CRUD; every `WhereOperator` (eq/ne/lt/lte/gt/gte/in/not_in/contains/starts_with/
  ends_with); `AND`/`OR` connectors; value transforms in/out per backend capability (JSON/dates/bools).
- **authFlow** — the auth-level operations exercised through the adapter (the behavior-bearing one).
- **joins** (+ plural joins) — relational reads.
- **uuid** / **number-id** — id strategies.
- **case-insensitive** — email case folding (`mode: insensitive`).
- **transactions** — commit/rollback (disable for `memory`).

## How to run

Run the `better-auth-rs-test-utils` battery against each adapter crate —
`cargo nextest run -p <adapter-crate> conformance`:

- `better-auth-rs-memory-adapter` (in-memory, no DB),
- `better-auth-rs-sqlx-adapter` (SQLx/Postgres, docker-compose),
- `better-auth-rs-diesel-adapter` ([Diesel](https://diesel.rs)),
- `better-auth-rs-seaorm-adapter` ([SeaORM](https://www.sea-ql.org/SeaORM/)).

Diesel + SeaORM are our Rust analogues of better-auth's Prisma/Drizzle/Kysely TS ORM adapters; the
same battery proves them all.

## Notes

- One battery, every backend: each adapter shares the same executable spec, so they behave
  consistently.
- Migrations must produce exactly `[user, session, account, verification]` (+ plugin tables
  when their features are on).
