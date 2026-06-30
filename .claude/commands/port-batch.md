---
description: Port the next batch of files from the manifest for a given phase/area
allowed-tools: Bash(cargo:*), Bash(git:*), Read, Edit, Write, Grep, Glob
---

# /port-batch

Drive a batch of files from `port/manifest.tsv` through the per-file porting loop.

Arguments: an optional path-prefix filter (e.g. `packages/core/src/db`) and/or a count
(default 10). With no args, take the next `todo` rows for the current phase's scope.

Steps:

1. Read `port/manifest.tsv`; select up to N rows with `status=todo` matching the filter,
   smallest-LOC first (warm up), but keep a dependency-sane order within a subsystem.
2. For each selected row, follow the per-file loop in `.claude/skills/porting-ts-to-rust`:
   read the `.ts` design reference → reimplement the feature idiomatically and securely in the
   Rust sibling at `rust_path`, built on mature audited crates (never hand-roll security
   primitives) → `cargo check -p <crate>` → write our own Rust behavior tests →
   `cargo nextest run -p <crate> <filter>` → update the row (`status`, `confidence`, `upstream_sha`).
3. Low-confidence ports: spawn an adversarial reviewer before marking `done`.
4. Commit on the current `claude/phase-*` branch with a Conventional-Commit message
   summarizing the batch (e.g. `feat(db): port adapter where-clause builder`).
5. Report: files done, confidence breakdown, anything left blocked.
