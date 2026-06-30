---
name: db-adapter-conformance
description: The executable spec a better-auth-rs DatabaseAdapter must satisfy, ported from better-auth's test-utils adapter suites. Use when implementing or testing a storage backend (sqlx-postgres, memory, future mysql/sqlite/mongo).
---

# DB adapter conformance

Upstream `packages/test-utils/src/adapter` defines the behavioral contract every adapter must
pass. Port it as a Rust trait-based conformance suite run against each backend.

## Suites to port (each is a behavior the adapter must satisfy)

- **normal** — CRUD; every `WhereOperator` (eq/ne/lt/lte/gt/gte/in/not_in/contains/starts_with/
  ends_with); `AND`/`OR` connectors; value transforms in/out per backend capability (JSON/dates/bools).
- **authFlow** — the auth-level operations exercised through the adapter (the behavior-bearing one).
- **joins** (+ plural joins) — relational reads.
- **uuid** / **number-id** — id strategies.
- **case-insensitive** — email case folding (`mode: insensitive`).
- **transactions** — commit/rollback (disable for `memory`).

## How to run

`cargo nextest run -p better-auth-rs --features memory-adapter conformance` (no DB) and
`... --features sqlx-postgres conformance` against the docker-compose Postgres service.

## Notes

- Drive the same scenarios in the differential harness so adapter behavior is also proven
  vs the TS server.
- Migrations must produce exactly `[user, session, account, verification]` (+ plugin tables
  when their features are on).
