# Branching Strategy

## Overview

This repository is migrating from the current `dev` integration model to the
target `main`-first operating model defined in
[`plans/specs/2026-05-09-plan-build-release-operating-model.md`](../../plans/specs/2026-05-09-plan-build-release-operating-model.md).

Until `OPMODEL-012` completes the cutover, executable branch authority remains:

- normal work branches from `dev`
- normal PRs target `dev`
- `main` remains the stable release branch

After `OPMODEL-012`, executable branch authority changes to the target model:

- `main` is the only permanent product branch
- normal work branches from `main`
- normal PRs target `main`
- `dev` is retired, protected against normal work, or retained only as a dated
  compatibility branch

Do not mix these models. Target-state design language does not authorise
`main`-first execution before `OPMODEL-012`.

## Current Compatibility Model

Use this model until `OPMODEL-012` lands.

| Branch                                 | Purpose                                                                        | Protection                                              |
| -------------------------------------- | ------------------------------------------------------------------------------ | ------------------------------------------------------- |
| `main`                                 | Stable branch for production releases and hotfixes.                            | PRs only. Release gate.                                 |
| `dev`                                  | Active integration branch for day-to-day work from multiple streams.           | PRs required. Standard CI.                              |
| `release/x.y` or `release/x.y.z`       | Temporary release stabilisation branch cut from `dev`.                         | PRs or maintainer-only pushes during release hardening. |
| `feat/*`, `fix/*`, `docs/*`, `chore/*` | Short-lived work branches created from `dev`.                                  | Disposable.                                             |
| `hotfix/*`                             | Urgent production fix branch created from `main` or the active release branch. | Disposable.                                             |

Current flow:

```text
feat/*  --PR--> dev --PR--> main
fix/*   --PR--> dev --PR--> main
docs/*  --PR--> dev --PR--> main

dev --cut--> release/x.y.z --PR--> main --merge back--> dev
main --branch--> hotfix/* --PR--> main --merge back--> dev
```

Current normal development:

1. Create feature, fix, docs, and chore branches from `dev`.
2. Merge completed work into `dev` continuously.
3. Keep branches small and short-lived where possible.
4. Use APS plans and work item IDs for planning; branch structure should reflect
   code flow, not roadmap ownership.

Current release flow:

1. Promote from `dev` to `main` frequently.
2. For low-risk releases, open a direct `dev -> main` release PR.
3. For higher-risk releases, cut `release/x.y` or `release/x.y.z` from `dev`.
4. Allow only release hardening on `release/*`: bug fixes, packaging, docs,
   changelog, and versioning.
5. Merge `release/*` into `main`, tag the release, then merge the release branch
   back into `dev` immediately.

Current hotfix flow:

1. Branch `hotfix/*` from `main` or the active `release/*` branch.
2. Merge the fix into the release target first.
3. Tag the patch release if needed.
4. Merge the same fix back into `dev` on the same day.

## Target Model

Use this model only after `OPMODEL-012` completes the cutover.

| Branch                                 | Purpose                                                                                      | Protection                                                        |
| -------------------------------------- | -------------------------------------------------------------------------------------------- | ----------------------------------------------------------------- |
| `main`                                 | The only permanent product branch; continuously releasable.                                  | PRs only. Required CI and release-readiness evidence for release. |
| `feat/*`, `fix/*`, `docs/*`, `chore/*` | Short-lived normal work branches created from `main`.                                        | Disposable.                                                       |
| `release/*`                            | Exceptional, short-lived release stabilisation branch when `main` cannot be tagged directly. | Explicit expiry; release hardening only.                          |
| `hotfix/*`                             | Urgent production repair branch from `main` or latest good tag when `main` is unreleasable.  | Disposable; incident follow-up required if bypassing normal flow. |
| `dev`                                  | Retired, protected, or dated compatibility branch.                                           | No normal work.                                                   |

Target normal flow:

```text
feat/*  --PR--> main --release when useful
fix/*   --PR--> main --release when useful
docs/*  --PR--> main
chore/* --PR--> main
```

Target release flow:

1. Select a green `main` SHA.
2. Run release readiness for that SHA.
3. Build candidate artefacts when required.
4. Tag the exact green `main` SHA.
5. Publish and verify release artefacts.
6. Emit the release record.
7. Reconcile APS shipped state from the release record.

Target hotfix flow:

1. Branch `hotfix/*` from `main`.
2. Merge the fix to `main` after targeted review and CI.
3. Tag a patch release from the green `main` SHA.
4. If `main` is unreleasable, branch from the latest good tag only as an
   incident response and reconcile back to `main` immediately.

## Cutover Rules

`OPMODEL-012` is the only work item that changes executable branch authority.
Before that item completes:

1. Do not open normal PRs to `main`.
2. Do not branch normal work from `main`.
3. Do not remove `dev -> main` release guidance from runbooks.
4. Do not treat target-state examples as commands.

During cutover:

1. Prepare PR templates, branch protections, and guidance.
2. Freeze normal new PRs into `dev`.
3. Promote current `dev` to `main`.
4. Retarget normal work to `main` before reopening normal PR flow.
5. Protect, expire, or retire `dev`.

## Branch Naming

- `feat/docsauth-github-oauth`
- `fix/rcli-038-cache-workspace-root`
- `docs/release-policy`
- `chore/config-cleanup`
- `release/0.3.0`
- `hotfix/auth-token-expiry`

## CI Tiers

Current CI still reflects the compatibility model:

- PRs to `dev` run standard validation.
- PRs to `main` act as release promotion or hotfix gates.

Target CI is defined by risk and changed paths rather than branch tiering:

- every PR gets fast formatting, lint, typecheck, and affected tests
- risky paths select fuller validation
- release readiness is recorded for a commit SHA
- tag workflows publish immutable artefacts

OPMODEL-005 and OPMODEL-010 own the release-readiness and drift-check design;
this guide only describes the branching intent.

### Cutover-aware CI gates (CICD-012)

The following workflow gates are dual-mode by design and survive the
`OPMODEL-012` rename without re-tuning:

| Gate                     | Migration trigger                                                                                       | Target trigger                                                              |
| ------------------------ | ------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------- |
| `ci.yml` Release Gate    | PR with base `main` and head `dev` / `release/*` / `hotfix/*`; push to `main`.                          | Same — normal `feat/*` PRs to `main` do not fire the gate.                  |
| `rust.yml` Cross-compile | Push to `main` or `dev`; PR with base `main` and head `dev` / `release/*` / `hotfix/*`.                 | Same — push to `main` and release/hotfix PRs only.                          |
| `pr-base-guard.yml`      | Rejects fork or non-release heads targeting `main`.                                                     | Migration-only — workflow is retired or rewritten as part of `OPMODEL-012`. |
| `release-readiness.yml`  | `expectedReachableFrom: main` for canonical readiness; `migration-dev` opt-in for compatibility probes. | `expectedReachableFrom: main` only — `migration-dev` retired post-cutover.  |

Every other validation workflow already triggers on both `main` and `dev` (or
fires on tag pushes / scheduled assurance), so the cutover does not silently
change their meaning.

## Why This Is Changing

The current `dev` model provides a useful integration buffer, but it creates
release-day branch reconciliation and can let `main` drift from active product
truth. The target model keeps one product line and moves validation authority to
CI results for commit SHAs, tags, GitHub Release assets, and release records.

## Related Docs

- [Worktree Policy](worktree-policy.md)
- [Release Runbook](release-runbook.md)
- [Branch Reconciliation Runbook](../runbooks/branch-reconciliation.md)
- [Operating Model Spec](../../plans/specs/2026-05-09-plan-build-release-operating-model.md)
