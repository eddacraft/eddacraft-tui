# GitHub Workflows

## Overview

This directory contains GitHub Actions workflows for CI/CD automation.

## Validation Contracts

Per the
[CI/CD and validation operating model](../../plans/specs/2026-05-10-ci-cd-validation-operating-model.md),
each workflow has a single validation contract. Two contracts are particularly
important to keep separate:

| Contract                       | Triggered by                                                                                             | Purpose                                                                                              |
| ------------------------------ | -------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------- |
| **Fast PR validation**         | `pull_request` against `main`/`dev`                                                                      | Prove the proposed change shape — affected lint/typecheck/test/metadata.                             |
| **Integration SHA validation** | `push` to `main`/`dev` (the integration branch — `dev` during the migration, `main` after `OPMODEL-012`) | Prove the merged SHA — full workspace lint/typecheck/test/build/e2e plus a single readiness summary. |

The integration push contract is intentionally distinct from the PR contract.
See
[CICD-005](../../plans/modules/ci-cd-validation.aps.md#cicd-005-integration-sha-validation-redesign).
Specifically:

- PR-only status fillers (`Lint & Format` skip, `Type Check` skip, `Unit Tests`
  skip) do not run on push — they exist to satisfy required-check status on
  docs-only / pure-Rust PRs.
- `Dependency Audit (PR)` (`ci.yml`) is PR-only. `security.yml`'s
  `Dependency Audit` job owns the equivalent check on push.
- `Security Summary` (`security.yml`) is PR-only — there is no PR to comment on
  for a push event.
- `Integration Readiness` (`ci.yml`) is push-only — it emits a single step
  summary identifying the SHA, the ref, and the validating job results. It fails
  the workflow if any required integration job failed; `APS Drift Check` is
  treated as warning-only evidence.

## Matrix Targeting

Platform matrices (macOS, Windows, Rust cross-compile, NAPI binding) are
expensive — macOS runners cost 10x, Windows 2x. Per
[CICD-008](../../plans/modules/ci-cd-validation.aps.md#cicd-008-matrix-and-platform-execution-targeting),
they run only when platform evidence is required:

| Matrix                            | Runs on                                                                                                                              | Skipped on                                                                         |
| --------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------- |
| `ci.yml` `Release Gate` (Node)    | PR to `main` (release-gate) or push to `main`, **only when `source-changed`**                                                        | Docs-only release PRs; routine PRs to `dev`; integration push to `dev`             |
| `rust.yml` `Cross (target)`       | PR to `main`, push to `main`/`release/*`, or `workflow_dispatch` — gated on `rust-changed` (dispatch ignores the rust-changed guard) | Push to `dev`; routine PRs to `dev`; JS-only diffs that admit the workflow paths   |
| `napi.yml` `Build`/`Test`         | PR/push touching `crates/anvil-checks-napi/**`, `crates/anvil-checks/src/**`, manifests, toolchain, or tags `napi-v*`                | Anything outside the napi binding's compile surface                                |
| `bench.yml` `Criterion`/`Midedit` | Push to `main` (release-gate) or `workflow_dispatch`                                                                                 | All PRs; push to `dev`                                                             |
| `ci-nightly.yml` `Unit Tests`     | `schedule` (daily 02:00 UTC) or `workflow_dispatch`                                                                                  | Routine PR and integration push events — nightly assurance owns scheduled evidence |

Operators can force any of the gated matrices via the workflow's `Run workflow`
button (`workflow_dispatch`) when an out-of-band verification run is needed.

## Workflows

### `ci.yml` — Continuous Integration

Owns fast PR validation and integration SHA validation for the Node / TypeScript
surface plus shared metadata, platform smoke, APS drift, and docs lint.

Path-based change detection (`.github/actions/detect-changes`) and the shared
classifier (`scripts/ci/classify-changes.sh`) decide which jobs run. Coverage
moved to `ci-nightly.yml` per CICD-006.

### `rust.yml` — Rust

Owns Rust validation for both PR (affected) and integration push (full
workspace). Includes Hakari verification, `cargo-deny`, acknowledgements
freshness, and a cross-compile matrix gated on `rust-changed` and the
release-gate condition (PR to `main`, push to `main`/`release/*`, or
`workflow_dispatch`). Push to `dev` no longer triggers the matrix per CICD-008 —
`dev` is the integration branch during migration but is not a release gate.

### `security.yml` — Security

Owns Semgrep, Trivy dependency audit, TruffleHog secret scan, and license
compliance. PR-only `Security Summary` posts a single sticky comment per
CICD-007. Weekly Monday 06:15 UTC schedule runs a full assurance sweep.

### `codeql.yml` — CodeQL

Owns CodeQL analysis for JavaScript/TypeScript and Rust on PR, push, and weekly
schedule.

### `napi.yml` — NAPI

Cross-platform NAPI binding builds for path-sensitive changes plus tagged
releases.

### `infra.yml` — Infrastructure

Pulumi preview/apply gates for infra changes.

### `bench.yml` / `bench-nightly.yml` — Benchmarks

Rust stress-test scenarios. Push-to-`main` and scheduled.

### `ci-nightly.yml` — Scheduled assurance

Coverage (TS + Rust), expanded matrices, and broader audits that do not belong
on routine PR or integration push.

### `ci-cost-report.yml` — CI Cost Report

Weekly cron + manual dispatch. Writes workflow / event / branch elapsed minutes
plus optional job timing and omitted-run diagnostics to the GitHub Actions step
summary (CICD-001).

### `release.yml` / `release-readiness.yml` / `release-harness.yml`

Release-candidate readiness, immutable tag publishing, and release verification.
See `plans/modules/release-orchestration.aps.md`.

### `pr-base-guard.yml`

Compatibility-mode guard rejecting `feat/*` / `fix/*` / `docs/*` / `chore/*` PRs
against `main`. **Cutover-blocking** — to be deleted at `OPMODEL-012` Phase 2.
See
[`plans/audits/2026-05-11-opmodel-012-workflow-audit.md`](../../plans/audits/2026-05-11-opmodel-012-workflow-audit.md).

### `labeler.yml`

Auto-labels PRs based on path filters.

## Local testing

```bash
# Lock the fast PR contract.
pnpm test:ci-fast-pr

# Lock the integration push contract.
pnpm test:ci-integration

# Lock the matrix-targeting contract.
pnpm test:ci-matrix-targeting

# Lock the APS drift CI wiring.
pnpm test:ci-drift-integration

# Lock the security workflow gating.
pnpm test:ci-security-targeting

# Lock the classifier and cost-report outputs.
pnpm test:ci-classify
pnpm test:ci-cost
```

## References

- [`plans/specs/2026-05-10-ci-cd-validation-operating-model.md`](../../plans/specs/2026-05-10-ci-cd-validation-operating-model.md)
- [`plans/modules/ci-cd-validation.aps.md`](../../plans/modules/ci-cd-validation.aps.md)
- [GitHub Actions Documentation](https://docs.github.com/en/actions)
