# better-auth-rs

[![CI](https://github.com/rubentalstra/better-auth-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/rubentalstra/better-auth-rs/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/better-auth-rs.svg)](https://crates.io/crates/better-auth-rs)
[![docs.rs](https://img.shields.io/docsrs/better-auth-rs)](https://docs.rs/better-auth-rs)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE)

**better-auth-rs** is a comprehensive, framework-agnostic authentication and authorization
library for Rust — a faithful, file-by-file port of
[better-auth](https://github.com/better-auth/better-auth) (the TypeScript framework) to idiomatic
async Rust.

> **Status: early / under active porting.** The library is being built up subsystem-by-subsystem
> (see `.claude/phases/`). It currently tracks better-auth **v1.6.22**.

## Why

Auth in Rust usually means stitching together crates and writing a lot of glue. better-auth solved
this for TypeScript with a batteries-included core and a plugin ecosystem (2FA, organizations,
magic links, JWT/OIDC, API keys, and more). `better-auth-rs` brings that same design to the Rust
community — the same data model, the same HTTP API, the same plugin surface — implemented with
mature Rust crates.

## Design

- **Framework-agnostic core** that operates on `http` types and implements `tower::Service`, with
  thin per-framework adapters.
- **`axum`** integration and **PostgreSQL via SQLx** ship first, both behind Cargo features.
- **Plugins are opt-in features named exactly like their better-auth counterparts**
  (`two-factor`, `organization`, `admin`, `api-key`, `jwt`, `oidc-provider`, …).
- **Behavioral parity is proven**, not assumed: better-auth's test suite is ported to Rust and a
  differential harness replays identical requests against the TypeScript server and the Rust server.

## Install

```toml
[dependencies]
better-auth-rs = { version = "0.1", features = ["axum", "sqlx-postgres", "organization", "two-factor", "jwt"] }
```

`default = ["axum", "sqlx-postgres"]`. Add the plugin features you need — each is named after the
upstream better-auth plugin.

## Compatibility

- Same database schema and HTTP API as better-auth (so the concepts and docs transfer directly).
- Idiomatic Rust crypto/storage internally — this is not a drop-in binary replacement, but ported
  behavior is validated against the original.

## Project layout

| Path                         | What                                                          |
|------------------------------|---------------------------------------------------------------|
| `crates/better-auth-rs-core` | Framework-agnostic primitives (port of `@better-auth/core`)   |
| `crates/better-auth-rs`      | The published crate (api, plugins, adapters, axum)            |
| `reference/better-auth`      | Read-only vendored TS source — the porting spec (not shipped) |
| `port/`                      | Porting manifest + pinned upstream baseline                   |
| `.claude/`                   | Porting methodology: phases, skills, hooks                    |

## Toolchain

Rust **1.96.0**, edition **2024** (pinned in `rust-toolchain.toml`).

## Contributing

Contributions welcome — see [CONTRIBUTING.md](./CONTRIBUTING.md). Branches start with `claude/` or a
descriptive prefix; commits follow [Conventional Commits](https://www.conventionalcommits.org/).

## Security

Please report vulnerabilities privately — see [SECURITY.md](./SECURITY.md).

## License & attribution

MIT — see [LICENSE](./LICENSE). `better-auth-rs` is a derivative work of
[better-auth](https://github.com/better-auth/better-auth) (MIT, © better-auth authors); the upstream
license is retained at `reference/better-auth/LICENSE.md`.
