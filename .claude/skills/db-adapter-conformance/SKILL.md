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

Run the `better-auth-rs-test-utils` battery against each adapter crate: the in-memory adapter
(`better-auth-rs-memory-adapter`, no DB) and the SQLx/Postgres adapter
(`better-auth-rs-sqlx-adapter`) against the docker-compose Postgres service —
`cargo nextest run -p <adapter-crate> conformance`.

## Notes

- One battery, every backend: each adapter shares the same executable spec, so they behave
  consistently.
- Migrations must produce exactly `[user, session, account, verification]` (+ plugin tables
  when their features are on).
