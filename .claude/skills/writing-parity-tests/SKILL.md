---
name: writing-parity-tests
description: How to port a better-auth *.test.ts to a Rust test and add a differential vector so the port is proven behaviorally identical. Use when porting any file that has a sibling test, or when adding test coverage.
---

# Writing parity tests

Two layers prove every port: the ported unit/integration test (Rust) and a differential vector
(TS server vs Rust server).

## Port the unit test

- Translate the sibling `*.test.ts` into a Rust test **co-located with the module it covers**
  (extend that module's test section; don't scatter new files).
- A test must be able to fail for the right reason. No flaky/time-based waits — await the
  condition, not the clock.
- For internal byte-format assertions (e.g. exact scrypt hash bytes), assert **behavior**
  instead (hash then verify round-trips), since crypto/storage are idiomatic Rust.
- DB-touching tests use the docker-compose Postgres service or the `memory-adapter`.

## Add a differential vector

- A vector is a request (method, path, headers, body) + the normalization rules for dynamic
  fields (ids, timestamps, tokens, cookie values).
- `cargo xtask differential` boots the vendored TS server + the Rust server and asserts identical
  status / JSON body / `Set-Cookie` semantics for each vector.

## Win condition

The ported behavior test passes in Rust **and** its differential vector matches the TS server.
