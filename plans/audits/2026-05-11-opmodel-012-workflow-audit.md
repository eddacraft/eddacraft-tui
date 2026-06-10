# OPMODEL-012 — Workflow Audit

> **Date:** 2026-05-11
> **Owner:** OPMODEL-012 (action plan:
> [`plans/archive/execution/opmodel-012.steps.md`](../archive/execution/opmodel-012.steps.md))
> **Purpose:** Inventory every `.github/workflows/*.yml` for branch triggers
> that reference `dev` or `main`, identify which workflows must change before
> the main-first cutover, and which can be cleaned up after.

## Method

Trigger blocks (`on:`) inspected for each workflow under `.github/workflows/`.
A workflow is **cutover-blocking** if leaving it unchanged would break normal
work or release flow once `dev` is no longer the integration target.

## Inventory

| Workflow | Triggers | Cutover impact | Action |
|---|---|---|---|
| `bench-nightly.yml` | `schedule`, `workflow_dispatch` | None | No change |
| `bench.yml` | `push: [main]`, `workflow_dispatch` | None — already main-only | No change |
| `ci-cost-report.yml` | `schedule`, `workflow_dispatch` | None | No change |
| `ci-nightly.yml` | `schedule`, `workflow_dispatch` | None | No change |
| `ci.yml` | `push: [main, dev]`, `pull_request: [main, dev]` | Triggers on both; PRs against `main` will continue to run | **Post-cutover cleanup** — drop `dev` from both lists |
| `codeql.yml` | `push: [main, dev]`, `pull_request: [main, dev]`, `schedule` | Triggers on both | **Post-cutover cleanup** — drop `dev` |
| `infra.yml` | `pull_request` (any base, path-filtered), `push: [main]`, `workflow_dispatch` | None — already main-only on push. (`PULUMI_STACK_PREVIEW: dev` at line 33 is a Pulumi stack name, not a branch ref — do not change.) | No change |
| `labeler.yml` | `pull_request` (any base) | None | No change |
| `napi.yml` | `push: [main, dev]` (path-filtered + tags), `pull_request` (any base, path-filtered) | Triggers on both | **Post-cutover cleanup** — drop `dev` |
| **`pr-base-guard.yml`** | `pull_request: [main]` | **Actively rejects** feat/fix/docs/chore branches targeting `main`. After cutover, normal work targets `main`; this guard would reject every such PR. | **Cutover-blocking** — delete (or invert) as part of the cutover |
| `release-harness.yml` | `pull_request: [main, dev]` (path-filtered), `push: [main, dev]` (path-filtered), `workflow_dispatch` | Triggers on both | **Post-cutover cleanup** — drop `dev` |
| `release-readiness.yml` | `workflow_dispatch` only | None | No change |
| `release.yml` | `pull_request` (path-filtered), `push: tags: ...` | None | No change |
| `rust.yml` | `push: [main, dev, rust-*, release/*]` (path-filtered), `pull_request` (any base, path-filtered) | Triggers on both | **Post-cutover cleanup** — drop `dev` |
| `security.yml` | `push: [main, dev]`, `pull_request: [main, dev]` | Triggers on both | **Post-cutover cleanup** — drop `dev` |

## Summary

- **Cutover-blocking workflows: 1** — `pr-base-guard.yml`. Must be deleted (or
  inverted) in the same change window as the cutover, otherwise every normal
  post-cutover PR targeting `main` fails the guard.
- **Post-cutover cleanup workflows: 6** — `ci`, `codeql`, `napi`,
  `release-harness`, `rust`, `security`. None of these break after cutover —
  they continue to trigger on `main` work. The `dev` entries become dead
  triggers; remove them in the Phase 3 docs-flip PR or a separate cleanup PR.
- **No-change workflows: 8** — already main-only, schedule-only, or
  base-agnostic.

## Adjacent surfaces (not workflows but cutover-relevant)

These are not `.github/workflows/*.yml` files, but the audit found them while
sweeping for `dev` references. Phase 3 (docs flip + cleanup PR) must address
each. They are listed here so they are not forgotten between Phase 0 and
Phase 3.

| Surface | File | Issue | Phase 3 action |
|---|---|---|---|
| Dependabot | `.github/dependabot.yml` | No explicit `target-branch`; tracks repo default. Will silently start targeting `main` the moment Step 6 flips the default. | Decide whether to pin `target-branch: main` explicitly (defensive, prevents future-default-branch surprises) or leave the implicit default. Document the decision. |
| Emergency-hotfix runbook | `docs/runbooks/emergency-hotfix.md` lines 170, 174 | Compatibility-mode back-merge commands branch from and PR into `dev`. Post-cutover the back-merge step is unnecessary (main is the only target) but the runbook still says "do this". | Remove the compatibility-mode back-merge section in Phase 3 once OPMODEL-012 is verified complete. Update the mode notes to say compatibility mode is retired. |
| `origin/HEAD` on local clones | n/a (per-contributor) | Each existing clone has `origin/HEAD -> refs/remotes/origin/dev`. `gh pr create` resolves the default base from this; it will keep proposing `dev` until each contributor runs `git remote set-head origin --auto`. | Phase 2 announcement includes the snippet (already in playbook Step 8). |

## Cutover-blocking detail: `pr-base-guard.yml`

The guard intentionally enforces the *current* compatibility model:

```text
Allowed head patterns for base 'main':
  - dev          (the standard dev -> main release sync)
  - release/*    (release branches cut from dev)
  - hotfix/*     (production fixes)
```

After cutover, the allowed head patterns become **everything that branches
from `main`** — `feat/*`, `fix/*`, `docs/*`, `chore/*`, plus the existing
`hotfix/*`. At that point the guard's allow-list is the wrong shape and the
guard rejects every normal PR.

Three options at cutover:

1. **Delete the workflow.** Simplest. The guard's job (keep `main` clean) is
   then taken over by branch protection rules — required CI checks, required
   PR review.
2. **Invert the allow-list** to forbid only specific patterns (e.g. raw
   pushes to `main`). Adds complexity for no clear benefit once protection
   rules exist.
3. **Repurpose** the workflow to enforce a different rule
   (e.g. "no PR opened to `dev`" during the retirement window). Possible but
   out of scope for OPMODEL-012.

**Recommendation:** Option 1 — delete `pr-base-guard.yml` as part of the
cutover commit. Branch protection on `main` (added in Phase 2) takes over
gate enforcement.

## Required CI checks for `main` branch protection

Captured from the most recent merged PR (#1407, commit `2800473c`). The
operator uses this list when adding branch protection in Phase 2; reconcile
against the current set at protection-add time, since this list is a
snapshot, not authority.

Checks observed on #1407:

- `APS Drift Check`
- `Analyze (javascript-typescript)` (CodeQL)
- `Auto-label`
- `Detect Changes`
- `Docs Lint`
- `Lint & Format`
- `Security Summary`
- `Type Check`
- `Unit Tests (Node 22.x, ubuntu-latest)`

Checks observed in **skipping** state on #1407 (path-filtered out for a
docs-only PR; would run on code PRs):

- `Build (Node 22.x)`
- `CI Metadata Checks`
- `Dependency Audit`
- `Dependency Audit (PR)`
- `E2E Harness (Node 22.x)`
- `License Compliance`
- `Platform Smoke`
- `Release Gate (Node 22.x, ${{ matrix.os }})`
- `SAST (Semgrep)`
- `Secret Scan`

The operator must decide at Phase 2 whether each of the "skipping" checks is
also required, since they only run when paths trigger them. If a check is
required but only sometimes runs, branch protection may block merges
indefinitely. Two safe defaults:

- Require only the checks that **always run** for the relevant change shape.
- Or: require nothing, and rely on PR review + the workflows running
  organically. Riskier; operator's call.

Reference: `gh pr checks <recent-merged-PR-number>` on a code PR before
protection-add will give a more representative snapshot.

### `Integration Readiness` is push-only and Node-only

**Do not add `Integration Readiness` to the PR-required checks list.** The
job (`ci.yml`, added by CICD-005) only runs on `push` events to the
integration branch, so it never reports a status on a PR — requiring it on
PRs would block every merge indefinitely.

It is also intentionally **Node / TypeScript-scoped**: it aggregates
`docs-lint`, `metadata-validation`, `platform-smoke`, `aps-drift`, `lint`,
`typecheck`, `test`, `build`, and `e2e-harness`. Rust integration evidence
lives in `rust.yml`'s job statuses (`Detect Rust Changes`, `nxrust Smoke`,
`Check`, `Test`, `Clippy`, `Format`, `Hakari verify`, `cargo-deny`,
`Acknowledgements freshness`, and the matrix-expanded `Cross (…)` legs —
`Cross (x86_64-unknown-linux-gnu)`, `Cross (aarch64-unknown-linux-gnu)`,
`Cross (x86_64-apple-darwin)`, `Cross (aarch64-apple-darwin)`,
`Cross (x86_64-pc-windows-msvc)`, `Cross (aarch64-pc-windows-msvc)`,
the names GitHub surfaces at branch-protection time). These are **not**
aggregated by Integration Readiness — a Rust-only push will show every
Integration Readiness row as `skipped`.

If `Integration Readiness` is ever elevated to a required *push* check
(e.g., for tag-protection or release-branch flows), the `rust.yml` job
statuses must be required alongside it; the aggregate is not a substitute
for the Rust workflow's gates. See `.github/workflows/README.md` § "PR vs
Integration push contract" for the full statement.

## Open PRs targeting `dev` at audit time

Snapshot 2026-05-11. The cutover playbook re-runs this query at Phase 2.

| PR | Title | Status |
|---|---|---|
| #1406 | fix(release): align RELORCH contract checks | Needs retarget or merge-before |
| #1408 | feat(cicd): move coverage to nightly assurance | Needs retarget or merge-before |
| #1333 | chore: bump the production-dependencies group across 1 directory with 19 updates | Dependabot — close-and-reopen or retarget |

## Branch divergence

- `dev` is 64 commits ahead of `main`.
- `main` has 0 commits not in `dev`.
- Fast-forward `main` → `dev`'s HEAD is mechanically clean.

## Phase fit

This audit produces the inputs for Phase 0 and Phase 2:

- **Phase 0** (this PR): inventory checked in; `pr-base-guard.yml` flagged
  cutover-blocking; required-check list captured for the playbook.
- **Phase 2** (operator window): playbook references this audit and the
  required-check list; `pr-base-guard.yml` deletion happens in the same
  commit window as the protection add and the fast-forward.
- **Phase 3** (docs flip + cleanup PR): drop `dev` triggers from the 6
  post-cutover cleanup workflows; sweep stale `dev` references.
