---
description: Diff a new better-auth release against the pinned baseline and open/work the re-port checklist
allowed-tools: Bash(cargo:*), Bash(git:*), Bash(gh:*), Read, Edit, Write, Grep, Glob
---

# /sync-upstream

Pull a new better-auth version into the Rust port. Argument: the target tag/sha (default: the
latest better-auth release).

Steps:

1. `cargo xtask sync --to <tag>` to diff every manifest file's `.ts` between its recorded
   `upstream_sha` and the target ref. Classify: unchanged / changed / new / deleted.
2. If drift exists and running in CI, open/update a `Sync v<x.y.z>` issue with a per-file
   checklist (idempotent via an HTML marker).
3. Work the checklist using the `syncing-upstream` skill: triage each diff by relevance, then
   fold the worthwhile feature/behavior/security changes into the Rust reimplementation (fan out
   one agent per file with the upstream diff + current Rust source), build out new features the
   release introduces, retire features it removes; adversarially review each. Skip changes that
   don't apply to an idiomatic Rust design.
4. Verify with `cargo nextest run` (our own Rust behavior tests) + `cargo clippy` for the touched files.
5. Advance each touched row's `upstream_sha` and bump `port/UPSTREAM_PORTED`. Commit on
   `claude/sync-<version>`.
