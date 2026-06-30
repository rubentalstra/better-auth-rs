# Phase 0 — Scaffolding

**Goal:** stand up the dual workspace + all tooling so porting can begin and be measured.
**Preconditions:** none.

## Deliverables

- Cargo workspace (`crates/better-auth-rs-core`, `crates/better-auth-rs`, `xtask`) +
  `rust-toolchain.toml` (1.96.0, edition 2024).
- Vendored `reference/better-auth/` @ pinned tag (read-only design reference); `port/UPSTREAM_PORTED` baseline SHA.
- `cargo xtask manifest` → `port/manifest.tsv` (every portable source as `todo`).
- Full `.claude/`: canonical `AGENTS.md` + `CLAUDE.md` symlink, per-dir `CLAUDE.md`,
  `settings.json` + hooks, skills, commands, and these phase docs.
- `.github/` set up (templates, CODEOWNERS, security workflows, release-plz, ci/e2e skeleton).
- pnpm workspace scoped to `docs/` (Fumadocs site); `docker-compose.yml`; root meta docs
  (CONTRIBUTING/SECURITY/CODE_OF_CONDUCT/LICENSE, Rust-ified README); `lefthook.yml`; `examples/` stub.

## Gates

`cargo check --workspace` green; `cargo fmt --all --check` clean; `cargo clippy` clean; CI passes
on the empty port; `cargo xtask manifest` runs.

## Exit criteria

Workspace builds on 1.96.0; manifest lists all portable source files as `todo`; `.claude/`
complete; CI green.
