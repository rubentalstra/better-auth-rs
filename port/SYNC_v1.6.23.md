# Upstream sync: better-auth v1.6.22 → v1.6.23

- **From:** `a90d061de7cdbd60e796230aadf5d1082add1fe2` (v1.6.22)
- **To:** `9dfceee14021fc15a2fb93023f39635f25b0b5ba` (v1.6.23)
- **Method:** full clone of `better-auth/better-auth@v1.6.23` + normalized mirror-integrity audit
  against every in-scope package's co-located `.ts` tree (directory segments normalized snake↔kebab,
  since the mirror snakes dir names to keep `.rs` module paths valid).
- **Date:** 2026-06-30

## Delta (in-scope packages only)

v1.6.23 is a tiny patch release. The complete in-scope `.ts` delta is **3 files**:

| Change   | File (co-located mirror path)                                              | `.rs` port today | Action |
|----------|-----------------------------------------------------------------------------|------------------|--------|
| **NEW**  | `crates/better-auth-rs/src/plugins/generic_oauth/providers/yandex.ts`       | absent (`todo`)  | vendored; add manifest row `yandex.ts → yandex.rs` |
| CHANGED  | `crates/better-auth-rs/src/plugins/generic_oauth/providers/index.ts`        | absent (`todo`)  | vendored (adds the `yandex` export) |
| CHANGED  | `crates/better-auth-rs-cli/src/generators/drizzle.ts`                       | absent (`todo`)  | vendored (`JSON.stringify` default-value fix) |

### Re-port debt: **none outstanding**
All three files belong to modules that are **not yet ported** (`generic_oauth` and the CLI generators
are `status = todo` in `port/manifest.tsv`). There is no existing `.rs` to re-port — the refreshed
`.ts` spec will be ported correctly when those phases run.

## Notes / non-findings
- **Directory naming:** the mirror stores upstream files under **snake_case directory names**
  (`two_factor/`, `email_otp/`, `generic_oauth/`, `social_providers/`, …) to keep the sibling `.rs`
  module paths legal; filenames keep their upstream (kebab/dotted) form. A naive path compare
  reports these as both "missing" and "extra" — they are the same files. The audit normalizes dir
  segments to see through this.
- **`core/src/async_hooks/`** uses an **underscore upstream** (not kebab). Its two files are
  byte-identical to the mirror; any "extra" flag for them is a normalization artifact, not a gap.
- All other in-scope packages (api-key, oauth-provider, passkey, redis-storage, scim, sso,
  telemetry, test-utils, memory-adapter, core) are **0-missing / 0-changed** vs v1.6.23.
- `reference/better-auth/{test,e2e}` and `LICENSE.md` refreshed to v1.6.23 (1 e2e file differed).

## Out of scope
The Rust port of the v1.6.23 deltas is tracked here for completeness but is not part of the
utils-port task; since no lagging `.rs` exists, nothing further is required now.
