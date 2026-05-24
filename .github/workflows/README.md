# GitHub Workflows

## Overview

This directory contains GitHub Actions workflows for CI/CD automation.

## Validation Contracts

Per the
[CI/CD and validation operating model](../../plans/specs/2026-05-10-ci-cd-validation-operating-model.md),
five contracts cover the entire target pipeline (the spec's MVP CI Operating
Model). A workflow may implement **one or more** of these contracts depending on
its trigger surface — `ci.yml`, for example, runs both PR validation
(`pull_request` event) and Integration push (`push` event) from the same file.
The Workflow Contract Map below lists each workflow with the contract(s) it
implements. A small set of explicit **auxiliary** workflows (labels, gates,
observability) sit outside the five contracts.

| #   | Contract              | Authoritative for                                                   |
| --- | --------------------- | ------------------------------------------------------------------- |
| 1   | **PR validation**     | Proving the proposed change shape (affected lint/typecheck/test).   |
| 2   | **Integration push**  | Proving the merged integration SHA — full workspace evidence.       |
| 3   | **Assurance**         | Scheduled full assurance — coverage, expanded matrices, deep scans. |
| 4   | **Release candidate** | Release readiness for an explicit SHA before tag publish.           |
| 5   | **Publish**           | Immutable tag-triggered build/publish + post-publish verification.  |

### Workflow Contract Map (CICD-010)

This map is the single source of truth for which workflow implements which
contract. Every workflow under `.github/workflows/` (excluding `*.example`) must
appear here, and every backtick-quoted workflow name in this file must
correspond to a file on disk. The `scripts/ci/workflow-contracts.test.sh`
fixture enforces both directions.

| Workflow                              | Contract              | Trigger surface                                                                                                                                                                                                                                           | Owner module |
| ------------------------------------- | --------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------ |
| `ci.yml`                              | PR + Integration      | `pull_request` to `main` / `push` to `main` — Node/TS lint, typecheck, test, build, e2e, metadata, platform-smoke                                                                                                                                         | CICD         |
| `rust.yml`                            | PR + Integration      | `pull_request` to `main` / `push` to `main`/`rust-*`/`release/*` (path-filtered) plus `workflow_dispatch`                                                                                                                                                 | CICD         |
| `security.yml`                        | PR + Integration      | `pull_request` to `main` / `push` to `main` plus weekly `schedule` and `workflow_dispatch`                                                                                                                                                                | CICD         |
| `codeql.yml`                          | PR + Integration      | `pull_request` to `main` / `push` to `main` plus weekly `schedule`                                                                                                                                                                                        | CICD         |
| `napi.yml`                            | Integration + Publish | `pull_request` / `push` to `main` (napi paths) plus `napi-v*` tags                                                                                                                                                                                        | CICD         |
| `infra.yml`                           | PR + Integration      | `pull_request` (any base, path-filtered) plus `push` to `main` and `workflow_dispatch` — Pulumi preview/apply                                                                                                                                             | CICD         |
| `release-harness.yml`                 | PR + Integration      | `pull_request` / `push` to `main` (release-script paths) plus `workflow_dispatch` — release-command contract                                                                                                                                              | RELORCH      |
| `bench.yml`                           | Integration           | `push` to `main` (release-gate) plus `workflow_dispatch`                                                                                                                                                                                                  | CICD         |
| `resource-budget.yml`                 | PR + Integration      | `pull_request` / `push` to `main` (crate and resource-budget policy paths) plus `workflow_dispatch`                                                                                                                                                       | ADOPT        |
| `editor-coexistence.yml`              | PR + Integration      | `pull_request` / `push` to `main` (anvil-cli/kernel/hook paths, harness, policy doc) plus `workflow_dispatch` — ADOPT-006 LSP/formatter coexistence gate                                                                                                  | ADOPT        |
| `bench-nightly.yml`                   | Assurance             | `schedule` plus `workflow_dispatch`                                                                                                                                                                                                                       | CICD         |
| `ci-nightly.yml`                      | Assurance             | `schedule` (daily 02:00 UTC) plus `workflow_dispatch` — coverage (TS + Rust), expanded matrices, multi-version Node                                                                                                                                       | CICD         |
| `ci-cost-report.yml`                  | Assurance             | weekly `schedule` plus `workflow_dispatch` — workflow / event / branch elapsed minutes, omitted-run diagnostics                                                                                                                                           | CICD         |
| `release-readiness.yml`               | Release candidate     | `workflow_dispatch` only — exact `sourceSha` validation, candidate metadata artefact, no publish credentials                                                                                                                                              | RELORCH      |
| `release.yml`                         | Publish               | `pull_request` (path-filtered) plus `push: tags: …` — cargo-dist build, publish, post-publish verification                                                                                                                                                | RELORCH      |
| `release-sign-artefacts.yml`          | Publish               | `release: published` plus `workflow_dispatch` — signs `*-installer.{sh,ps1}` with minisign, uploads `.minisig` back                                                                                                                                       | DISTRIB      |
| `publish-eddacraft-tui.yml`           | Publish               | `push: tags: ['eddacraft-tui-v*']` — validate against D-TUIR-007 publish-side gates, `cargo publish` to crates.io, propagate the tag (append-only) to `eddacraft/eddacraft-tui` mirror, then `gh release create` on anvil-001                             | TUIR         |
| `homebrew-bump.yml`                   | Publish               | `release: published` plus `workflow_dispatch` plus path-filtered `pull_request` — dry-run contract on PR, manual republish to `eddacraft/homebrew-tap`, macOS arm64/x64 install smoke                                                                     | DISTRIB      |
| `labeler.yml`                         | Auxiliary (PR labels) | `pull_request` (any base) — `actions/labeler` path-based labels                                                                                                                                                                                           | CICD         |
| `mirror-acknowledgements-starter.yml` | Auxiliary (mirror)    | `push` to `main` (kit + workflow paths) plus `workflow_dispatch` — `git subtree split` + force-push to public `eddacraft/acknowledgements-starter` mirror                                                                                                 | ATTRIB       |
| `mirror-eddacraft-tui.yml`            | Auxiliary (mirror)    | `push` to `main` (crate + workflow paths) plus `workflow_dispatch` — `git subtree split` + force-push to public `eddacraft/eddacraft-tui` mirror                                                                                                          | TUIR         |
| `acknowledgements-kit.yml`            | PR + Integration      | `pull_request` / `push` to `main` (kit + `licences.toml` + `licences.node-allow.txt` paths) — kit self-tests (dispatcher schema + two-block + strict-license-field + drift-detector + Node driver preflight/render/strict) plus `expand-licences --check` | ATTRIB       |

### PR vs Integration push contract

The PR contract is intentionally distinct from the integration push contract.
See
[CICD-005](../../plans/archive/modules/ci-cd-validation.aps.md#cicd-005-integration-sha-validation-redesign).

- PR-only status fillers (`Lint & Format` skip, `Type Check` skip, `Unit Tests`
  skip) do not run on push — they exist to satisfy required-check status on
  docs-only / pure-Rust PRs.
- `Dependency Audit (PR)` (`ci.yml`) is PR-only. `security.yml`'s
  `Dependency Audit` job owns the equivalent check on push.
- `Security Summary` (`security.yml`) is PR-only — there is no PR to comment on
  for a push event.
- `Integration Readiness` (`ci.yml`) is push-only and aggregates the **Node /
  TypeScript** side of the integration push (`docs-lint`, `metadata-validation`,
  `platform-smoke`, `lint`, `typecheck`, `test`, `build`, `e2e-harness`). It
  emits a single step summary identifying the SHA, the ref, the run link, and
  the validating job results, and fails the workflow if any required Node-side
  job reports `failure` / `cancelled`. `APS Drift Check` is treated as
  warning-only evidence per
  [CICD-011](../../plans/archive/modules/ci-cd-validation.aps.md#cicd-011-apsreporelease-drift-checks-in-ci).
- **Rust validation is intentionally not aggregated by `Integration Readiness`**
  — `rust.yml` (`Check`, `Test`, `Clippy`, `Format`, `Hakari verify`,
  `cargo-deny`, `Acknowledgements freshness`, `Cross (target)`) is the
  authoritative Rust integration gate. A Rust-only push will show every row of
  the Integration Readiness summary as `skipped`; the merged-SHA evidence in
  that case lives in the Rust workflow's job statuses, not in the readiness
  summary. If `Integration Readiness` is later elevated to a required branch
  protection check, the Rust workflow's job statuses must be required alongside
  it; the readiness aggregate is not a substitute.

### Authority audit (no duplicated gates)

The following surfaces were potential duplicate-authority risks. Each was either
eliminated, gated, or justified:

- `Dependency Audit (PR)` (`ci.yml`) vs `Dependency Audit` (`security.yml`) —
  CICD-005 gated `ci.yml`'s job to `pull_request` events; `security.yml` owns
  the push and scheduled audit.
- Semgrep (`security.yml`) vs CodeQL (`codeql.yml`) — distinct static analysis
  tools with distinct rule packs and authority. Both run on PR + push +
  schedule. Not duplicate authority.
- `metadata-validation` (`ci.yml`) `infra-static-check` step vs `infra.yml`
  Pulumi preview/apply — `metadata-validation` runs `pnpm lint:check` for static
  infra config validation; `infra.yml` runs the Pulumi engine for preview/apply.
  Distinct contracts, not duplicate.
- `Integration Readiness` aggregate (`ci.yml`) vs per-job statuses — the
  aggregate fails on any non-success/skipped required job so the contract status
  survives even when the granular jobs are skipped by path filters.

## Matrix Targeting

Platform matrices (macOS, Windows, Rust cross-compile, NAPI binding) are
expensive — macOS runners cost 10x, Windows 2x. Per
[CICD-008](../../plans/archive/modules/ci-cd-validation.aps.md#cicd-008-matrix-and-platform-execution-targeting),
they run only when platform evidence is required:

| Matrix                            | Runs on                                                                                                                                                                                                                                                                                          | Skipped on                                                                         |
| --------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------- |
| `ci.yml` `Release Gate` (Node)    | PR to `main` (release-gate) or push to `main`, **only when `source-changed`**                                                                                                                                                                                                                    | Docs-only release PRs; routine PRs to `dev`; integration push to `dev`             |
| `rust.yml` `Cross (target)`       | PR to `main`, push to `main`/`release/*`, or `workflow_dispatch` — gated on `rust-changed` (dispatch ignores the rust-changed guard). **Cost note:** one `workflow_dispatch` run bills ~390 weighted runner-minutes (6-target matrix = 2x ubuntu + 2x macOS@10x + 2x windows@2x; use sparingly). | Push to `dev`; routine PRs to `dev`; JS-only diffs that admit the workflow paths   |
| `napi.yml` `Build`/`Test`         | PR/push touching `crates/anvil-checks-napi/**`, `crates/anvil-checks/src/**`, manifests, toolchain, or tags `napi-v*`                                                                                                                                                                            | Anything outside the napi binding's compile surface                                |
| `bench.yml` `Criterion`/`Midedit` | Push to `main` (release-gate) or `workflow_dispatch`                                                                                                                                                                                                                                             | All PRs; push to `dev`                                                             |
| `ci-nightly.yml` `Unit Tests`     | `schedule` (daily 02:00 UTC) or `workflow_dispatch`                                                                                                                                                                                                                                              | Routine PR and integration push events — nightly assurance owns scheduled evidence |

Operators can force any of the gated matrices via the workflow's `Run workflow`
button (`workflow_dispatch`) when an out-of-band verification run is needed.

## Workflows (per-file detail)

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
releases. Tag (`napi-v*`) push additionally publishes per-platform npm packages.

### `infra.yml` — Infrastructure

Pulumi preview on PR, Pulumi apply on push to `main` and `workflow_dispatch`.

### `release-harness.yml` — Release command contract

PR/push gate scoped to `scripts/release/**` — runs the release-command contract
tests (assess, preflight, prepare, promote) on Ubuntu + macOS.

### `bench.yml` / `bench-nightly.yml` — Benchmarks

Rust criterion + stress + midedit benchmarks. `bench.yml` is push-to-`main`
(release-gate) and dispatch; `bench-nightly.yml` covers scheduled assurance.

### `resource-budget.yml` — Resource Budget

ADOPT-002 resource-budget gate for `anvil watch`. Builds the release CLI, runs
`cargo bench -p anvil-bench --bench watch_resource_budget`, uploads the JSON
budget verdict, and fails when steady-state CPU or peak RSS exceeds the pinned
budget in `docs/policies/resource-budget.md`.

### `editor-coexistence.yml` — Editor Coexistence

ADOPT-006 LSP / formatter coexistence gate. Builds the release CLI, installs
`rust-analyzer`, `tsserver`, `pyright`, `ruff`, `prettier`, and `eslint`, then
runs `tools/test-harness/editor-coexistence/run-harness.sh` against minimal
language fixtures while `anvil watch` runs concurrently. Uploads the JSON
verdict and fails when a runner exits non-zero, lock-contention markers appear
in either log, or the required-targets floor in
`tools/test-harness/editor-coexistence/required-targets.txt` is not met. Matrix
and verdict shape are documented in `docs/policies/editor-coexistence.md`.

### `ci-nightly.yml` — Scheduled assurance

Coverage (TS + Rust), expanded cross-platform matrices, multi-version Node
tests, and broader audits that do not belong on routine PR or integration push.

### `ci-cost-report.yml` — CI Cost Report

Weekly cron + manual dispatch. Writes workflow / event / branch elapsed minutes
plus optional job timing and omitted-run diagnostics to the GitHub Actions step
summary (CICD-001).

### `release.yml` / `release-readiness.yml`

Release-candidate readiness (`release-readiness.yml`, dispatch-only with exact
SHA validation, no publish credentials) and immutable tag publishing
(`release.yml`, cargo-dist build + publish + post-publish verify). See
`plans/modules/release-orchestration.aps.md`.

### `release-sign-artefacts.yml`

Signs every `*-installer.{sh,ps1}` asset published by `release.yml` with the
release minisign key and uploads the detached `.minisig` files back to the
GitHub Release. Triggers on `release: published` (also dispatchable). Refuses to
run when `vars.ANVIL_MINISIGN_PUBLIC_KEY` is empty or still equals the committed
dev fallback. See ADR-045 and DISTRIB-001.

### `publish-eddacraft-tui.yml`

TUIR-005. Publishes the `eddacraft-tui` crate to crates.io from canonical source
at `crates/eddacraft-tui/`, then propagates the release tag (append-only) to the
public mirror at `eddacraft/eddacraft-tui` and cuts a GitHub Release on
anvil-001. Triggers on
`push: tags: ['eddacraft-tui-v[0-9]+.[0-9]+.[0-9]+', 'eddacraft-tui-v[0-9]+.[0-9]+.[0-9]+-*']`
(prefixed semver per D-TUIR-002 — the Anvil monorepo ships multiple crates with
independent semver and unprefixed `v…` would collide with Anvil product tags). A
ref-reachability guard refuses tags pointing at commits not on `main`. The full
D-TUIR-007 publish-side gate matrix runs as the authoritative pre-publish check
(`fmt --check`, `clippy -D warnings`, all-features + no-default-features test,
`doc --no-deps`, `cargo-deny`, `publish --dry-run`, and a byte-diff of
`cargo package --list` against the TUIR-001 baseline at
`plans/specs/2026-05-22-tui-reintegration-baseline/package-list.txt`).
Authenticates via three secrets: `CRATES_IO_EDDACRAFT_TUI_TOKEN` (crates.io
publish-only, scoped to the `eddacraft-tui` crate) plus
`EDDACRAFT_MIRROR_BOT_APP_ID` + `EDDACRAFT_MIRROR_BOT_PRIVATE_KEY` (the
`eddacraft-mirror-bot` GitHub App, same App used by `mirror-eddacraft-tui.yml` —
minted into a short-lived installation token per run). Tag push to the mirror
uses `http.extraheader` Basic auth with a single tag refspec — no `--force`, no
`--mirror`, so existing tags on the mirror (including pre-cutover `v0.x.y` tags
per D-TUIR-011) are never overwritten (D-TUIR-009). See
`docs/runbooks/eddacraft-tui-release.md` for the operator cut procedure,
verification, rollback, and App-key rotation steps.

### `homebrew-bump.yml`

Three-job workflow that backs the Homebrew distribution path: `dry-run` runs the
`scripts/release/bump-homebrew.sh` unit tests and a synthetic-formula dry-run on
every PR that touches the script or workflow; `republish` provides a
`workflow_dispatch` recovery surface that re-patches and re-publishes
`Formula/anvil.rb` to `eddacraft/homebrew-tap` for a given tag; `smoke` runs
`brew install eddacraft/tap/anvil` on macOS arm64 (`macos-14`) and x64
(`macos-13`) after every release. See `docs/runbooks/homebrew-publish.md` and
DISTRIB-003.

### `labeler.yml`

Auto-labels PRs based on path filters.

### `mirror-acknowledgements-starter.yml`

ATTRIB-011. Mirrors `tools/starters/acknowledgements/` to its public sibling
repo `eddacraft/acknowledgements-starter`. Triggers on `push` to `main` when the
kit directory or this workflow file changes, plus `workflow_dispatch` for manual
force-resync. A ref-guard step refuses to run on anything other than
`refs/heads/main`. Authenticates via `MIRROR_PUSH_TOKEN` (fine-grained PAT
scoped to the mirror repo, `Contents: Read and write`); deploy keys are disabled
at the eddacraft org level. The mirror repo is read-only by policy — canonical
edits land here, the workflow force-pushes the subtree split. See
`plans/modules/attribution-pipeline-v3.aps.md` (ATTRIB-011) and
`plans/execution/ATTRIB-011.steps.md`.

### `mirror-eddacraft-tui.yml`

TUIR-004. Mirrors `crates/eddacraft-tui/` to its public sibling repo
`eddacraft/eddacraft-tui` (the historical canonical home for the crate). Same
shape as `mirror-acknowledgements-starter.yml`: `push` to `main` with crate +
workflow path filters, plus `workflow_dispatch` for manual force-resync, a
ref-guard refusing anything other than `refs/heads/main`, and pre-push
prepending of `MIRROR-README.md` onto `README.md` (D-TUIR-012). Authenticates
via the `eddacraft-mirror-bot` GitHub App (org-owned, installed on
`eddacraft/eddacraft-tui` only, permissions `Contents: Read and write` +
`Metadata: Read`) — the workflow mints a short-lived installation token at
runtime via `actions/create-github-app-token` using the
`EDDACRAFT_MIRROR_BOT_APP_ID` + `EDDACRAFT_MIRROR_BOT_PRIVATE_KEY` repo secrets
and uses it as the password in an `http.extraheader` Basic auth header.
URL-embedded credentials fail as `CURLE_URL_MALFORMAT` on a single stray byte.
The mirror is read-only by policy and force-pushes `main` only — release tags
(`eddacraft-tui-v*`) are pushed by the separate publish workflow
(`publish-eddacraft-tui.yml`), so existing tags on the mirror (including
pre-cutover unprefixed `v0.x.y` tags per D-TUIR-011) are never overwritten by
the mirror job (D-TUIR-009). See `plans/modules/tui-reintegration.aps.md`
(D-TUIR-004, D-TUIR-009, D-TUIR-012) and
`docs/policies/eddacraft-tui-mirror.md`.

## Local testing

```bash
# Lock the fast PR contract.
pnpm test:ci-fast-pr

# Lock the integration push contract.
pnpm test:ci-integration

# Lock the matrix-targeting contract.
pnpm test:ci-matrix-targeting

# Lock the workflow contract map (every file appears in the README).
pnpm test:ci-workflow-contracts

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
- [`plans/archive/modules/ci-cd-validation.aps.md`](../../plans/archive/modules/ci-cd-validation.aps.md)
- [GitHub Actions Documentation](https://docs.github.com/en/actions)
