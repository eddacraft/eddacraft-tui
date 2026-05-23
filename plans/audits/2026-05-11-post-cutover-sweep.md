# Post-Cutover Sweep — `dev` → `main` Reference Audit

> **Date:** 2026-05-11
> **Owner:** Post-OPMODEL-012 verification
> **Purpose:** Independent verification sweep after the OPMODEL-012 Phase 3
> docs flip (PR #1420) to catch stale `dev`-as-integration-target references
> the Phase 3 sweep missed, and to refresh the [`RELEASE-PLAN.md`](../../RELEASE-PLAN.md)
> for the new state.

## Method

Searched the live repository under `docs/`, `.claude/`, and `plans/` for
patterns that imply `dev` is still the integration target:

```text
dev branch | target dev | dev -> main | dev → main
PRs?\s+(target|to|against)\s+\`?dev\`?
branch from `dev` | git checkout dev | origin/dev (instructional)
until\s+OPMODEL-012 | before\s+OPMODEL-012
```

Hits were classified into three buckets:

1. **Update now** — live operational doc or active APS module where the
   stale wording would mislead a reader.
2. **Leave (historical)** — dated evidence, archived runbook, or
   migration-plan spec that documents the migration itself; the `dev`
   references are accurate as historical record.
3. **Already correct** — touched by the OPMODEL-012 Phase 3 PR.

## Findings updated in this PR

| File | Issue | Update |
|---|---|---|
| `docs/runbooks/rollback-bad-main.md` | Header described "Compatibility mode (`dev -> main`)" with target-mode notes flagged inline; Mode notes section split into compat-mode / target-mode | Header collapsed to single main-first scope; Mode notes section collapsed to a single Notes section |
| `docs/public/anvil/operations/troubleshooting.md` | Section titled "Windows CI regressions on `dev` branches are silent" with the gap framed around the dev → main release-sync | Renamed to "Windows CI regressions on feature branches are silent"; gap reframed around release/hotfix-class PRs and pushes to `main` |
| `docs/marketing/pitch-deck/README.md` | TUI Spec row annotated "(dev branch)" | Annotation removed |
| `plans/modules/ci-cd-validation.aps.md` (Migration Boundary) | Said "this repository remains `dev`-first until `OPMODEL-012` completes" | Rewritten to note OPMODEL-012 complete on 2026-05-11 and the `main` integration target |
| `plans/modules/ci-cd-validation.aps.md` (CICD-004 Coordinates) | "normal PR target remains `dev` until OPMODEL-012" | Updated to note cutover complete; normal PR target is `main` |
| `plans/specs/2026-05-10-release-readiness-workflow.md` | Paragraph claimed "Current executable work still branches from `dev` and PRs target `dev`" | Rewritten to note cutover complete; `migration-dev` remains as a transitional compatibility probe |
| `RELEASE-PLAN.md` | Whole file was pre-cutover: OPMODEL 6/12, RELORCH 0/11 Proposed, OPMODEL-012 in Wave 3 | Full refresh: post-cutover state, OPMODEL archived 12/12, RELORCH unblocked 3/12, CICD 8/12, remaining lanes + parallel-now table + revised `v0.6.2-beta` cut |

## Findings intentionally left

| File | Reason |
|---|---|
| `docs/runbooks/branch-reconciliation.md` | Self-described one-time divergence-recovery doc; the `dev` references are accurate for what it documents. |
| `docs/runbooks/intd-012-windows-evidence.md` | Dated evidence document for INTD-012; captures state as of when it was written. |
| `docs/archive/runbooks/v0.6.0-beta-release-runbook.md` | Historical release runbook for `v0.6.0-beta`. |
| `plans/specs/2026-05-04-launch-a1-execution.md` | Historical execution plan; describes how A1 shipped. |
| `plans/specs/2026-05-09-plan-build-release-operating-model.md` | The OPMODEL operating-model spec itself; `dev` references describe the migration plan that this spec drove. |
| `plans/specs/2026-05-10-ci-cd-validation-operating-model.md` | The CICD operating-model spec; describes migration vs target columns deliberately. |
| All files under `plans/archive/` and `plans/completed/` | Archived state; never rewrite. |
| `plans/modules/release-orchestration.aps.md` (RELORCH-006 Blocked block at line ~411 still references "the temporary `dev -> main` compatibility path") | RELORCH is actively in flight in a parallel worktree; not touched to avoid mid-stream merge conflicts. The agent owning RELORCH will refresh the pause/unblock language as it resumes work. |

## Already correct (verified)

These were updated in the OPMODEL-012 Phase 3 PR #1420 and re-verified
here:

- `docs/guides/branching-strategy.md`
- `docs/guides/worktree-policy.md`
- `docs/runbooks/release-runbook.md`
- `.claude/skills/release/SKILL.md`
- `docs/runbooks/emergency-hotfix.md`
- `.github/dependabot.yml`
- 6 cleanup workflows (`ci.yml`, `codeql.yml`, `napi.yml`,
  `release-harness.yml`, `rust.yml`, `security.yml`)

## Follow-ups

- The `migration-dev` `expectedReachableFrom` option in
  `.github/workflows/release-readiness.yml` is now a transitional compatibility
  probe. Remove in a follow-up sweep once no live SHA needs it. Tracked
  informally — file a CICD task if any live consumer is found.
- `plans/specs/2026-05-09-plan-build-release-operating-model.md` and
  `plans/specs/2026-05-10-ci-cd-validation-operating-model.md` describe the
  migration plan and remain authoritative design records. Consider archiving
  alongside the OPMODEL module once no remaining work item cites them as
  "design in progress."
