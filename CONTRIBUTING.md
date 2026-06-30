# Contributing to better-auth-rs

Thanks for helping build a secure, idiomatic Rust reimplementation inspired by better-auth!

## Ground rules

- **The reference is a design guide, not a spec to copy.** For each module, the **co-located
  sibling `.ts`** (same folder as the `.rs`, both in `crates/*/src/`) shows what feature to build
  and how better-auth behaves; the baseline is pinned in `port/UPSTREAM_PORTED`. **Keep every `.ts`
  — never edit or delete it.** Reimplement behavior idiomatically and securely on mature, audited
  crates; don't copy upstream bugs or weaker choices — fix them and prefer modern best-in-class
  defaults. [AGENTS.md](./AGENTS.md) is the single source of truth for the full contract; this file
  is the quick start.
- **Toolchain is pinned** to Rust `1.96.0`, edition `2024` (`rust-toolchain.toml`).
- **Branches** start with `claude/` or a short descriptive prefix.
- **Commits** follow [Conventional Commits](https://www.conventionalcommits.org/) — the release
  tooling derives the changelog and version bumps from them.

## Local workflow

```bash
cargo check -p <crate>                  # fast inner loop
cargo fmt --all                         # never call rustfmt directly
cargo clippy --workspace --all-targets  # CI runs with -D warnings
cargo nextest run -p <crate> [filter]   # scope your tests
cargo xtask manifest                    # regenerate the porting manifest
```

DB-backed tests use the services in `docker-compose.yml`:

```bash
docker compose up -d postgres redis
```

## Porting a file

Follow `.claude/skills/porting-ts-to-rust` and the per-file loop in `.claude/phases/README.md`:
read the `.ts` → write the Rust sibling at its `rust_path` (from `port/manifest.tsv`) → make it
compile → port the matching test → run it → update the manifest row. Cover each externally
observable behavior with our own Rust behavior tests (`cargo nextest`).

## Pull requests

- Keep changes scoped; every behavioral change ships a test.
- Run `cargo fmt --all --check`, `cargo clippy`, and the relevant `cargo nextest` before pushing.
- Be honest in the PR description about what works and what doesn't.

## Code of conduct

This project follows the [Contributor Covenant](https://www.contributor-covenant.org/) — see
[CODE_OF_CONDUCT.md](./CODE_OF_CONDUCT.md).
