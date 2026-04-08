# Branching Strategy

## Overview

This repository uses a two-branch model that matches active multi-stream
development in parallel worktrees:

- `main` is the stable release branch.
- `dev` is the active integration branch.

The key rule is cadence: `dev` is a short-horizon integration branch, not a
long-lived alternate product line. The model works only if release promotion is
frequent and every `main`-only fix is merged back quickly.

## Branches

| Branch                                 | Purpose                                                                        | Protection                                              |
| -------------------------------------- | ------------------------------------------------------------------------------ | ------------------------------------------------------- |
| `main`                                 | Stable branch for production releases and hotfixes. Always deployable.         | PRs only. Full release CI gate.                         |
| `dev`                                  | Active integration branch for day-to-day work from multiple streams.           | PRs required. Standard CI.                              |
| `release/x.y` or `release/x.y.z`       | Temporary release stabilisation branch cut from `dev`.                         | PRs or maintainer-only pushes during release hardening. |
| `feat/*`, `fix/*`, `docs/*`, `chore/*` | Short-lived work branches created from `dev`.                                  | Disposable.                                             |
| `hotfix/*`                             | Urgent production fix branch created from `main` or the active release branch. | Disposable.                                             |

## Workflow

```text
feat/*  ──PR──► dev ──PR──► main
fix/*   ──PR──► dev ──PR──► main
docs/*  ──PR──► dev ──PR──► main

dev ──cut──► release/x.y.z ──PR──► main ──merge back──► dev
main ──branch──► hotfix/* ──PR──► main ──merge back──► dev
```

## Normal Development

1. Create feature, fix, docs, and chore branches from `dev`.
2. Merge completed work into `dev` continuously.
3. Keep branches small and short-lived where possible.
4. Use APS plans and work item IDs for planning; branch structure should reflect
   code flow, not roadmap ownership.

## Release Flow

1. Promote from `dev` to `main` frequently.
2. For low-risk releases, open a direct `dev -> main` release PR.
3. For higher-risk releases, cut `release/x.y` or `release/x.y.z` from `dev`.
4. Allow only release hardening on `release/*`: bug fixes, packaging, docs,
   changelog, and versioning.
5. Merge `release/*` into `main`, tag the release, then merge the release branch
   back into `dev` immediately.

## Hotfix Flow

1. Branch `hotfix/*` from `main` or the active `release/*` branch.
2. Merge the fix into the release target first.
3. Tag the patch release if needed.
4. Merge the same fix back into `dev` on the same day.

## Cadence Rules

1. Promote `dev -> main` at least weekly.
2. During active development, prefer promotion every 2-3 days.
3. Do not allow `release/*` branches to live for weeks.
4. If the `dev -> main` PR feels too large to review comfortably, promotion is
   already overdue.
5. If a fix lands on `main`, it is not complete until `dev` has it too.

## Divergence Guardrails

1. `main` and `dev` must stay close enough that promotion remains routine.
2. Stop queuing new release work if `main...dev` grows beyond a small,
   reviewable change set.
3. Use the branch reconciliation runbook only for exceptional recovery, not as a
   normal release mechanism.
4. Avoid long-lived release-only changes on `main`.

## Branch Naming

- `feat/docsauth-github-oauth`
- `fix/rcli-038-cache-workspace-root`
- `docs/release-policy`
- `chore/config-cleanup`
- `release/0.3.0`
- `hotfix/auth-token-expiry`

## CI Tiers

### PRs to `dev` (lightweight)

- Lint and format
- Type check
- Unit tests (Linux, Node 20)
- Build (Linux, Node 20)
- E2E tests when relevant
- Security scans when code changes are detected

### PRs to `main` (release gate)

All of the above plus:

- Cross-platform smoke tests (macOS and Windows)

### Nightly (`ci-nightly.yml`)

- Cross-platform: macOS and Windows
- Multi-version: Node 22 and 24
- Runs at 02:00 UTC / 10:00 AM Perth

## Why this model

This repo regularly runs multiple active streams in parallel. `dev` provides a
safe integration branch before release, while `main` stays stable. The process
fails when promotion waits too long, because release fixes accumulate on `main`
and structural work continues on `dev`.

This strategy preserves the useful buffer of `dev` while preventing a repeat of
that drift.

## Related Docs

- [Release Runbook](release-runbook.md)
- [Worktree Policy](worktree-policy.md)
- [Branch Reconciliation Runbook](../runbooks/branch-reconciliation.md)
