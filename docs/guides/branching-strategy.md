# Branching Strategy

## Overview

`main` is the only permanent product branch. Normal work branches from `main`
with Worktrunk (`wt`), and normal PRs target `main`. `dev` is retired as a dated
compatibility branch following the OPMODEL-012 cutover on 2026-05-11.

See
[`plans/specs/2026-05-09-plan-build-release-operating-model.md`](../../plans/specs/2026-05-09-plan-build-release-operating-model.md)
for the operating-model context.

## Branches

| Branch                                 | Purpose                                                                                                                             | Protection                                                        |
| -------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------- |
| `main`                                 | The only permanent product branch; continuously releasable.                                                                         | PRs only. Required CI and release-readiness evidence for release. |
| `feat/*`, `fix/*`, `docs/*`, `chore/*` | Short-lived normal work branches created from `main`.                                                                               | Disposable.                                                       |
| `release/*`                            | Exceptional, short-lived release stabilisation branch when `main` cannot be tagged directly.                                        | Explicit expiry; release hardening only.                          |
| `hotfix/*`                             | Urgent production repair branch from `main` or latest good tag when `main` is unreleasable.                                         | Disposable; incident follow-up required if bypassing normal flow. |
| `dev`                                  | Retired dated compatibility branch (tag `dev-retired-2026-05-11`). Scheduled for deletion on or after 2026-07-10 — see issue #1419. | Locked: no creation, update, deletion, or non-FF.                 |

## Normal flow

```text
feat/*  --PR--> main --release when useful
fix/*   --PR--> main --release when useful
docs/*  --PR--> main
chore/* --PR--> main
```

1. Start from the permanent `main` worktree or an equivalent clean `main` clone.
2. Create the task branch and worktree with `wt switch --create <branch>`.
3. Do all task work in that Worktrunk-managed worktree, not in the permanent
   `main` worktree.
4. Open the PR against `main`.
5. Required CI checks must pass; review threads must resolve.
6. Merge — squash, rebase, or merge are all allowed by the `main` ruleset.
7. After PR open, merge, abandon, or long pause, offer `wt remove` to clean up
   the Worktrunk worktree and local branch.

Example:

```bash
git fetch origin --prune
wt switch --create docs/release-plan-refresh
```

If a task starts in the wrong place, prefer moving the work into a disposable
Worktrunk worktree before it grows. Do not reset, delete, or move uncommitted
work without explicit user approval.

## Cleanup flow

Branch and worktree cleanup is part of finishing a task, but it is still a user
choice when the work is not safely merged or preserved remotely.

Offer cleanup when any of these happens:

1. A PR is opened and the user is done with local iteration.
2. A PR is merged.
3. A branch is abandoned or superseded.
4. A stream is paused with no near-term next action.

Before removing anything, verify:

1. `git status` is clean in the worktree being removed.
2. The branch is pushed if the PR is still open.
3. The branch is merged, or the user explicitly approves deleting unmerged local
   state.

Safe cleanup command for a merged branch:

```bash
wt remove
```

Worktrunk removes the current worktree and deletes the local branch when it is
safe. Use raw `git branch -D` only when the user explicitly approves deleting an
unmerged branch. Do not delete remote branches unless the user asks or the
repository's PR workflow has already closed them.

## Release flow

1. Select a green `main` SHA.
2. Run release readiness for that SHA.
3. Build candidate artefacts when required.
4. Tag the exact green `main` SHA.
5. Publish and verify release artefacts.
6. Emit the release record.
7. Reconcile APS shipped state from the release record.

`release/*` branches are exceptional. Cut one only when `main` cannot be tagged
directly (e.g. a release window needs to freeze a SHA while routine merges
continue). The branch carries an explicit expiry and is restricted to release
hardening — versioning, packaging, docs, changelog. Merge to `main` and tag from
`main`, then delete the `release/*` branch.

## Hotfix flow

1. Branch `hotfix/*` from `main` (or the latest good tag if `main` is
   unreleasable).
2. Merge the fix to `main` after targeted review and CI.
3. Tag a patch release from the green `main` SHA.
4. If `hotfix/*` was cut from a tag because `main` was unreleasable, reconcile
   back to `main` immediately as an incident response; see
   [`docs/runbooks/emergency-hotfix.md`](../runbooks/emergency-hotfix.md).

## Branch naming

- `feat/docsauth-github-oauth`
- `fix/rcli-038-cache-workspace-root`
- `docs/release-policy`
- `chore/config-cleanup`
- `release/0.3.0`
- `hotfix/auth-token-expiry`

## CI gates

CI selects validation by risk and changed paths rather than branch tiering. The
`main` ruleset enforces the always-running subset as required checks:

- APS Drift Check
- Docs Lint
- Lint & Format
- Type Check
- Unit Tests (Node 22.x, ubuntu-latest)
- Security Summary
- Detect Changes

Path-filtered and risk-targeted workflows (Build, E2E Harness, Release Gate,
SAST, Secret Scan, Dependency Audit, Platform Smoke, License Compliance) run
when their triggers fire but are not required for merge — they would otherwise
block PRs whose paths don't trigger them.

Release readiness and drift checks are defined in OPMODEL-005 (release readiness
workflow) and OPMODEL-010 (warning-mode drift). The CICD module owns the
validation contract that selects checks per risk class.

## Why this is the model

The previous `dev`-as-integration model created release-day branch
reconciliation, let `main` drift from active product truth, and required
back-merges that frequently slipped. The main-first model keeps one product line
and moves validation authority to CI results for commit SHAs, tags, GitHub
Release assets, and release records.

## Related docs

- [Worktree Policy](worktree-policy.md)
- [Release Runbook](../runbooks/release-runbook.md)
- [Main-First Cutover Runbook](../runbooks/main-first-cutover.md) (historical
  evidence of the 2026-05-11 cutover; not for re-execution)
- [Branch Reconciliation Runbook](../runbooks/branch-reconciliation.md)
  (historical evidence of the pre-cutover divergence recovery)
- [Operating Model Spec](../../plans/specs/2026-05-09-plan-build-release-operating-model.md)

## Archive — pre-OPMODEL-012 compatibility model

Retained for historical reference. The `dev -> main` promotion model described
here is **no longer in force** as of the 2026-05-11 cutover.

Before cutover:

- normal work branched from `dev`
- normal PRs targeted `dev`
- `main` was the stable release branch promoted from `dev`
- `release/*` branches were cut from `dev` and merged to `main`, then
  back-merged to `dev`
- `hotfix/*` branches were cut from `main` or `release/*`, merged to the release
  target first, then back-merged to `dev` the same day

Cutover evidence:

- Phase 0 audit:
  [`plans/audits/2026-05-11-opmodel-012-workflow-audit.md`](../../plans/audits/2026-05-11-opmodel-012-workflow-audit.md)
- Phase 2 playbook:
  [`docs/runbooks/main-first-cutover.md`](../runbooks/main-first-cutover.md)
- Cutover SHA: `b6f236e90dbc03338f17767202acf93f1449f8d2`
- `pr-base-guard.yml` retirement: PR #1417 (merged
  `62d85777c03ffe9a196befc9390a7d0a18ff0ee8`)
- `dev` retirement issue: #1419 (deletion on or after 2026-07-10)
