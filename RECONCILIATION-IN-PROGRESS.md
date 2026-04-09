# ⚠️ Reconciliation In Progress — Soft Freeze

`main` and `dev` are in a **soft freeze** while branch reconciliation runs.

## What this means

- **Do not** merge pull requests into `main` or `dev`.
- **Do not** force-push `main` or `dev`.
- **Do not** rename release-critical refs or recreate branches.
- Work on topic branches only; hold merges until the freeze lifts.
- Coordinate with the BRECON owner before landing anything on `main`/`dev`.

## Why

`main` (66 commits ahead) and `dev` (889 commits ahead) have diverged over
the `0.3.0` Rust rewrite. Release-critical CI, auth, Rust CLI, and release
pipeline fixes live only on `main` and must be ported onto `dev` before a
branch-role cutover. A blind merge would resurrect deleted archived paths
and regress the newer `dev` topology.

## Tracking

- Runbook: `docs/runbooks/branch-reconciliation.md`
- APS module: `plans/modules/branch-reconciliation.aps.md` (BRECON)
- Recovery branch: `reconcile/dev-with-main-fixes` (will be created in Phase 1)
- Safety anchors: `backup/dev-before-reconcile`, `backup/main-before-reconcile`

## How the freeze lifts

Freeze ends when BRECON-014 (cutover) completes. At that point:

1. `main` will point at the reconciled `dev` lineage.
2. Old `main` will live on as `main-archive`.
3. This marker file will be deleted and the AGENTS.md notice removed.

If you have work blocked by the freeze, add it to a topic branch and
coordinate merge timing with the BRECON owner.
