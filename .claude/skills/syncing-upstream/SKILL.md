---
name: syncing-upstream
description: How to pull a new better-auth release into the Rust port — diff upstream against the pinned baseline, re-port changed files, and advance the markers. Use when bumping to a new better-auth version or resolving a "Sync vX.Y.Z" issue.
---

# Syncing a new better-auth release

The repo pins the ported baseline in `port/UPSTREAM_PORTED` (SHA + tag) and records the
`upstream_sha` per file in `port/manifest.tsv`.

## Steps

1. **Diff:** `cargo xtask sync --to <tag|sha>` — sparse-fetches upstream to a temp dir
   (nothing written to the repo) and prints, per manifest file, whether its `.ts` changed
   between the recorded `upstream_sha` and the new ref. Classes: `unchanged / changed / new / deleted`.
2. **Triage:** changed files → re-port; new files → write a fresh port + add a manifest row;
   deleted files → remove the Rust sibling + drop the row.
3. **Re-port (fan out):** for a large diff, run one agent per changed file with context =
   {the upstream diff, the current Rust port, the `porting-ts-to-rust` skill}. Keep the change
   minimal and faithful to the upstream diff.
4. **Adversarially review** each re-port (a second agent tries to find a behavior mismatch).
5. **Verify:** `cargo nextest run -p <crate> <filter>` + the differential vectors for the touched
   files.
6. **Advance markers:** set each touched row's `upstream_sha` to the new ref; bump
   `port/UPSTREAM_PORTED`. Conventional-commit on a `claude/sync-<version>` branch.

## Automation

`.github/workflows/upstream-sync.yml` runs step 1 on a schedule and opens/updates a
**"Sync vX.Y.Z"** issue with a per-file checklist when drift exists; a Claude routine works the
checklist via steps 3–6. See `.claude/phases/phase-sync-upstream.md`.
