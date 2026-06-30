# Phase ∞ — Upstream sync (full auto)

**Goal:** keep the Rust reimplementation current with new better-auth releases, folding in
relevant new features, behavior changes, and security fixes, automatically.
**Preconditions:** the reimplementation exists and `port/UPSTREAM_PORTED` is set.

## The loop

1. `cargo xtask sync --to <tag|sha>` — sparse-fetch upstream into a temp dir (nothing written
   to the repo), diff each `manifest.tsv` entry's `ts_path` between its recorded `upstream_sha`
   and the new ref, classify (unchanged / changed / new / deleted), and print a per-file
   fold-in checklist.
2. `.github/workflows/upstream-sync.yml` (scheduled + `workflow_dispatch`): on a new release
   with drift, auto-open/update a **"Sync v<x.y.z>"** issue containing the per-file checklist.
3. A Claude routine (or maintainer) **fans out one agent per changed file** — context =
   the upstream diff (read as a design reference) + the current Rust reimplementation + the
   `syncing-upstream` skill — folds the relevant change in idiomatically, then an
   **adversarial review** pass confirms it.
4. Run our own Rust behavior tests (`cargo nextest`) for the touched files; clippy + fmt clean.
5. On green, update each row's `upstream_sha` and bump `port/UPSTREAM_PORTED` to the new ref.

## Notes

- Docs MDX drift is tracked the same way (the docs sub-track shares the manifest discipline).
- New upstream files → write a fresh port and add the manifest row; deleted files → mark the
  row and remove the Rust sibling.
- This is the mechanism that lets us prune the bulk vendored `reference/` later: the sync
  fetches upstream on demand, so the repo stays lean while tracking still works.
