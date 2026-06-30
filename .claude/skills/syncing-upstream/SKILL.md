---
name: syncing-upstream
description: How to review a new better-auth release and fold the relevant changes (new features, behavior changes, security fixes) into the Rust reimplementation. Use when bumping the tracked better-auth version or working a "Sync vX.Y.Z" issue.
---

# Tracking a new better-auth release

better-auth is our **design reference**, not an upstream we mirror byte-for-byte. When a new release
lands we review what changed and fold the relevant bits into our Rust implementation. The tracked
baseline is pinned in `port/UPSTREAM_PORTED` (SHA + tag); the manifest records per-file status.

## Steps

1. **Diff:** `cargo xtask sync --to <tag|sha>` — sparse-fetches upstream to a temp dir (nothing
   written to the repo) and prints, per manifest file, whether its reference `.ts` changed. Classes:
   `unchanged / changed / new / deleted`.
2. **Triage by relevance, not byte-diff:** changed files → if the change affects observable
   behavior, a feature, or fixes a security issue, update the Rust implementation; cosmetic / TS-only
   changes (types, internal refactors) need no action. New files → a new feature to consider
   building. Deleted files → consider removing the corresponding Rust + manifest row.
3. **Re-vendor the reference `.ts`** for changed/new files (it is our design reference) and update
   the manifest's `upstream_sha`.
4. **Implement & test** the relevant changes the idiomatic, secure Rust way (reuse crates — never
   hand-roll); cover with Rust behavior tests.
5. **Review** each change — a second pass / second agent confirms the behavior is correct and secure.
6. **Advance markers:** bump `port/UPSTREAM_PORTED`. Conventional-commit on a `claude/sync-<version>`
   branch.

## Automation

`.github/workflows/upstream-sync.yml` runs step 1 on a schedule and opens/updates a **"Sync
vX.Y.Z"** issue with a per-file checklist when drift exists. See
`.claude/phases/phase-sync-upstream.md`.
