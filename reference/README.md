# `reference/` — vendored better-auth test references (read-only)

The bulk of the upstream **better-auth** source is **co-located inside the crates** — each
`.ts` sits next to where its `.rs` goes (e.g. `crates/better-auth-rs/src/crypto/password.ts`
beside `password.rs`): read-only, not compiled, excluded from the published crate
(`exclude = ["**/*.ts"]`), and **kept permanently — never deleted, even after a file is fully
ported** (the `.ts` set is the 1:1 coverage map, the re-sync diff target, and the
differential-harness fixture).

This `reference/better-auth/` directory holds only the upstream pieces that don't map to a
single `.rs` sibling and are kept as references for the test tiers:

- `test/` — shared upstream test setup/utilities.
- `e2e/` — the end-to-end suite (adapter conformance, integration, smoke) we port into the
  Rust test framework + differential harness.
- `LICENSE.md` — upstream MIT license (retained for attribution).

## Provenance & rules

- **Upstream:** https://github.com/better-auth/better-auth
- **Pinned version:** `v1.6.23` (commit `9dfceee14021fc15a2fb93023f39635f25b0b5ba`) — see `port/UPSTREAM_PORTED`.
- **License:** MIT — `better-auth-rs` is a derivative work and retains attribution.
- **Read-only.** Never edit *or delete* vendored `.ts` (here or co-located in crates); port into
  the `.rs` sibling and **keep the `.ts`** (it stays as the permanent 1:1 spec).
- Re-vendor / refresh with `cargo xtask vendor --from <upstream-clone>`; track porting in
  `port/manifest.tsv`; pull new upstream versions via `cargo xtask sync` (see
  `.claude/phases/phase-sync-upstream.md`).
