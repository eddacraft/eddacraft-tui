<!--
APS Module: CI/CD Validation Operating Model
============================================
Implements the CI/CD, validation, and pipeline-efficiency layer for the target
Plan / Build / Release operating model. See: plans/aps-rules.md
-->

# CI/CD Validation Operating Model

| ID   | Owner | Status   | Progress |
| ---- | ----- | -------- | -------- |
| CICD | —     | Complete    | 12/12    |

**Spec:** [2026-05-10 CI/CD And Validation Operating Model](../../specs/2026-05-10-ci-cd-validation-operating-model.md)
**Operating model:** [2026-05-09 Plan / Build / Release Operating Model](../../specs/2026-05-09-plan-build-release-operating-model.md)
**Execution architecture:** [2026-05-09 Agentic Execution Ecosystem Architecture](../../specs/2026-05-09-agentic-execution-ecosystem-architecture.md)
**Review architecture:** [2026-05-09 Council Agent And Skill Change Proposal](../../specs/2026-05-09-council-agent-skill-change-proposal.md)
**Council review:** [2026-05-10 CICD Validation Council Review](../../reviews/2026-05-10-cicd-validation-council.md) — converged, no open findings.

## Purpose

Reduce CI/CD cost and increase validation confidence by moving deterministic
feedback earlier, making CI path/risk targeted, and separating fast PR checks,
integration readiness, release candidate readiness, tag publishing, and
scheduled assurance.

Target flow:

```text
APS intent -> local deterministic validation -> targeted review -> fast PR CI
  -> integration SHA validation -> release candidate readiness -> tag publish
  -> post-publish verification -> release record
```

## Cross-Cutting Convention

This is a cross-cutting module and follows
[`plans/aps-rules.md#module-types-vertical-and-conductor`](../../aps-rules.md#module-types-vertical-and-conductor).
CICD owns CI/CD cost, validation layering, shared classification, local-first
validation commands, and workflow decomposition. It coordinates with specialist
modules rather than absorbing their authority.

## Migration Boundary

`OPMODEL-012` completed the main-first cutover on 2026-05-11; `main` is the
single integration target and normal PRs target `main`. Earlier CICD items
(CICD-001..-009, CICD-012) shipped with explicit migration vs target wording.
Remaining CICD work (CICD-005, -008, -010, -011) now designs against the
post-cutover surface directly:

- Normal PR validation targets `main`.
- `release/*` and `hotfix/*` are the release-class head patterns; the
  cross-platform and cross-compile matrices fire on those PRs plus pushes to
  `main`.
- `dev` is retired as a dated compatibility branch (`dev-retired-2026-05-11`;
  deletion follow-up issue #1419). No CICD work should add new `dev`
  triggers; the 6 cleanup workflows that previously triggered on `dev`
  (`ci.yml`, `codeql.yml`, `napi.yml`, `release-harness.yml`, `rust.yml`,
  `security.yml`) had their `dev` entries removed in PR #1420.

## In Scope

- Shared path/risk classifier for hooks, scripts, agents, and CI.
- Fast PR, full PR, integration, scheduled assurance, candidate, tag, and
  post-publish validation contracts.
- CI cost observability and run-reason summaries.
- Local-first deterministic validation scripts.
- Coverage, security, dependency, matrix, and artefact-retention cost controls.
- Workflow consolidation or behaviour migration where safe.
- APS/repo/release drift-check touchpoints in CI, coordinated with OPMODEL and
  DOCGOV.

## Out of Scope

- Changing the branch cutover itself — owned by OPMODEL-012.
- Implementing release command internals — owned by RELORCH.
- Implementing documentation metadata/index validators — owned by DOCGOV.
- Changing release artefact topology or cargo-dist asset shape.
- Moving review judgement into CI.
- Building a general-purpose orchestration platform.

## Interfaces

**Depends on:**

- `plans/specs/2026-05-10-ci-cd-validation-operating-model.md`
- `plans/specs/2026-05-09-plan-build-release-operating-model.md`
- `plans/specs/2026-05-09-agentic-execution-ecosystem-architecture.md`
- `plans/specs/2026-05-09-council-agent-skill-change-proposal.md`
- Current workflows under `.github/workflows/`
- Current shared actions under `.github/actions/`
- `package.json`, `nx.json`, `Cargo.toml`, and `Cargo.lock`

**Coordinates with:**

- OPMODEL — lifecycle vocabulary, migration sequencing, branch cutover, drift
  checks.
- RELORCH — release-readiness, candidate artefacts, tag/publish verification,
  release records.
- DOCGOV — docs validation and generated-index checks.
- CGBDG and council work — review evidence routing and deterministic guidance.
- Security and attribution modules — dependency, licence, acknowledgement, and
  security assurance surfaces.

**Exposes:**

- CI/CD validation architecture and rollout plan.
- Shared classifier contract for CI, hooks, scripts, and agents.
- Local validation command surface.
- Workflow cost observability requirements.
- Migration-safe workflow changes.

## Closure Criteria

This module is Complete when:

1. Routine PR validation is fast, path/risk targeted, and does not run coverage
   or broad matrices by default.
2. Integration-branch validation proves the merged SHA without blindly duplicating
   PR validation.
3. Release candidate readiness is available for an explicit SHA and coordinates
   with RELORCH.
4. Tag publishing and post-publish verification have clear evidence boundaries.
5. A shared classifier is consumed by CI, hooks/scripts, and agent guidance.
6. CI cost reporting exposes workflow/job minutes, cancellation waste, matrix
   spend, coverage spend, and run reasons.
7. APS/repo/release drift checks exist at least in warning mode.
8. `OPMODEL-012` can retarget normal validation from `dev` to `main` without
   redesigning the validation model.

## Tasks

### CICD-001: Baseline CI cost and run-reason observability

- **Status:** Complete
- **Intent:** Make CI waste visible before changing pipeline shape.
- **Expected Outcome:** A baseline report shows workflow, event, and branch
  elapsed minutes plus optional job timing and omitted-run diagnostics. Full
  runner-cost attribution, path/risk classes, matrix spend, coverage spend, and
  security spend are target-state dimensions for later CICD work.
- **Validation:** Cost report generated from recent Actions history; workflow
  summaries include elapsed timing and run-shape evidence for representative PR
  and push events.
- **Files:** `.github/workflows/ci-cost-report.yml`, `scripts/ci/cost-report.sh`,
  `package.json`
- **Coordinates with:** OPMODEL-009, OPMODEL-010
- **Completed:** 2026-05-10 — Added `pnpm ci:cost` backed by
  `scripts/ci/cost-report.sh` plus a weekly/manual `CI Cost Report` workflow that
  writes workflow, event, branch, optional job timing summaries, and omitted-run
  diagnostics to the GitHub Actions step summary. The report explicitly labels
  elapsed timing versus full runner-cost attribution.
- **Validation Run:** `pnpm ci:cost -- --limit 2 --jobs`,
  `bash scripts/ci/cost-report.sh --limit 3 --json`
- **Confidence:** high

---

### CICD-002: Shared path and risk classifier contract

- **Status:** Complete
- **Intent:** Replace per-workflow path heuristics with one deterministic
  classifier consumed by hooks, agents, scripts, and CI.
- **Expected Outcome:** Classifier emits JSON with path classes, risk classes,
  required checks, required reviews, and warnings for staged, branch, PR, and
  push contexts.
- **Validation:** Fixture tests cover docs-only, TS, Rust, policy, release,
  workflow, infra, NAPI, lockfile, and mixed changes.
- **Files:** `scripts/ci/classify-changes.sh`,
  `scripts/ci/classify-changes.test.sh`, `package.json`
- **Coordinates with:** OPMODEL-007, council/review specs
- **Completed:** 2026-05-10 — Added `pnpm ci:classify` backed by
  `scripts/ci/classify-changes.sh`. The classifier emits JSON with
  `pathClasses`, `riskClasses`, `requiredChecks`, `requiredReviews`, and
  `warnings` for staged, branch, PR, and push contexts. Added fixture coverage
  for docs-only, TypeScript, Rust, policy, release, workflow, infra, NAPI,
  lockfile, and mixed changes via `pnpm test:ci-classify`.
- **Validation Run:** `pnpm test:ci-classify`, per-script shell syntax loop for
  `scripts/ci/classify-changes.sh` and `scripts/ci/classify-changes.test.sh`
- **Confidence:** medium

---

### CICD-003: Local deterministic validation command surface

- **Status:** Complete
- **Intent:** Move deterministic feedback earlier so CI is not the first broad
  validator humans and agents rely on.
- **Expected Outcome:** `validate:staged`, `validate:changed`, and
  `validate:full` commands run the same changed-area checks that fast PR CI will
  enforce where practical.
- **Validation:** Commands succeed on a clean workspace and fail on controlled
  fixture drift.
- **Files:** `package.json`, `scripts/validate/local.sh`,
  `scripts/validate/local.test.sh`
- **Coordinates with:** OPMODEL-007, DOCGOV-005
- **Completed:** 2026-05-10 — Added `pnpm validate:staged`,
  `pnpm validate:changed`, and `pnpm validate:full` backed by
  `scripts/validate/local.sh`. The staged and changed modes use the shared
  classifier to build a deterministic command plan; full mode runs the full local
  deterministic suite. Added fixture coverage via `pnpm test:validate-local`.
- **Validation Run:** `pnpm test:validate-local`,
  `pnpm validate:changed -- --dry-run --json`,
  `pnpm validate:full -- --dry-run --json`
- **Confidence:** high

---

### CICD-004: Fast PR validation redesign

- **Status:** Complete
- **Intent:** Make every PR receive cheap, deterministic, path-targeted validation
  without routine coverage, broad matrices, or unrelated security work.
- **Expected Outcome:** PR validation runs format/lint/typecheck/affected tests
  and metadata checks selected by the classifier.
- **Validation:** Representative docs-only, TS-only, Rust-only, and mixed PRs run
  the expected jobs and skip unrelated expensive jobs.
- **Files:** `.github/workflows/ci.yml`, `.github/workflows/rust.yml`,
  `.github/actions/`
- **Coordinates with:** OPMODEL (cutover completed 2026-05-11; normal PR
  target is now `main`)
- **Completed:** 2026-05-10 — Fast PR validation now consumes classifier-required
  checks for Node, docs, workflow, shell, policy, dependency, infra, release, and
  platform surfaces. Routine PR unit tests no longer collect/upload coverage,
  broad build and E2E work is no longer triggered for unrelated PR changes, and
  dev-target Rust PRs skip the cross-compile matrix unless they are release-gate
  PRs to `main`. Added `pnpm test:ci-fast-pr` as a workflow contract fixture.
- **Validation Run:** `pnpm test:ci-classify`, `pnpm test:ci-cost`,
  `pnpm test:ci-fast-pr`, `pnpm test:validate-local`, `pnpm format:check`,
  `pnpm lint:md`, `git diff --check`, Council convergence review.
- **Residual Risk:** Full `pnpm typecheck` currently stops on pre-existing Nx
  workspace sync drift before typechecking; PR CI remains the executable workflow
  authority for the changed CI paths.
- **Confidence:** medium

---

### CICD-005: Integration SHA validation redesign

- **Status:** Complete
- **Intent:** Separate merged-SHA validation from PR feedback and reduce duplicate
  execution on pushes to the integration branch.
- **Expected Outcome:** Push validation proves the `dev` integration SHA during
  migration, then `main` after cutover, with a distinct readiness contract.
- **Validation:** Push to integration branch runs integration readiness checks and
  does not duplicate PR-only commentary or coverage work.
- **Files:** `.github/workflows/ci.yml`, `.github/workflows/rust.yml`,
  `.github/workflows/security.yml`
- **Coordinates with:** OPMODEL-012
- **Completed:** 2026-05-11 — `ci.yml` now gates the `*-skip` required-check
  fillers (`lint-skip`, `typecheck-skip`, `test-skip`) and the PR-named
  `dependency-audit` Trivy job to `github.event_name == 'pull_request'`, so the
  integration push no longer runs status-fillers or duplicates `security.yml`'s
  `dependency-audit` on the merged SHA. A new push-only `integration-readiness`
  job (`if: always() && github.event_name == 'push'`) depends on the full set of
  integration-validating jobs, emits a single readiness summary naming the SHA
  and ref, and fails the workflow if any required integration job (lint,
  typecheck, test, build, e2e, docs-lint, metadata, platform-smoke) reports a
  non-`success`/`skipped` result; `aps-drift` remains warning-only per
  CICD-011. `rust.yml` and `security.yml` already differentiated push from PR
  (full workspace on push; PR-only summary comment), so no behavioural changes
  there. `scripts/ci/integration-validation.test.sh` locks the contract via
  `pnpm test:ci-integration`, and the metadata-validation job runs the new
  fixture alongside the existing CI fixtures. `.github/workflows/README.md`
  now documents the fast-PR / integration-push contracts and the explicit
  exclusions on each side.
- **Validation Run:** `pnpm test:ci-integration`, `pnpm test:ci-fast-pr`,
  `pnpm test:ci-security-targeting`, `pnpm test:ci-classify`, `pnpm format:check`,
  `pnpm lint:md`, `node -e '... yaml.parse ...'` on `.github/workflows/ci.yml`.
- **Confidence:** medium

---

### CICD-006: Coverage and artefact retention cost controls

- **Status:** Complete
- **Intent:** Stop paying coverage and artefact-upload cost on routine validation
  where it provides marginal confidence.
- **Expected Outcome:** TypeScript and Rust coverage move to scheduled assurance
  or release candidate readiness; routine PR/integration validation uses plain
  tests.
- **Validation:** PR and integration runs no longer invoke coverage flags or upload
  coverage artefacts; scheduled/candidate runs still produce coverage evidence.
- **Files:** `.github/workflows/ci.yml`, `.github/workflows/rust.yml`,
  `.github/workflows/ci-nightly.yml`
- **Coordinates with:** TCOV, OPMODEL-005
- **Completed:** 2026-05-11 — `.github/workflows/ci.yml` push test step no longer
  passes `--coverage --coverage.reporter=json-summary --coverage.reporter=text`
  and no longer runs the TypeScript coverage summary or `coverage-report-22.x`
  upload. `.github/workflows/rust.yml` push events drop the cargo-llvm-cov cache,
  install, instrumentation pass, coverage summary, and `coverage-report-rust`
  upload; the strict `cargo nextest run --workspace --no-fail-fast` gate plus
  doctests remain. Equivalent `coverage-typescript` and `coverage-rust` jobs were
  added to `.github/workflows/ci-nightly.yml` so scheduled assurance still
  publishes the same artefact names and step-summary tables; nightly uploads use
  `retention-days: 14`. `CARGO_LLVM_COV_VERSION` and `llvm-tools-preview` move
  with the coverage work into the nightly file.
- **Confidence:** high

---

### CICD-007: Security and dependency assurance targeting

- **Status:** Complete
- **Intent:** Keep security assurance strong while avoiding broad per-PR scans for
  unrelated changes.
- **Expected Outcome:** Semgrep, CodeQL, Trivy, TruffleHog, cargo-deny, licence,
  and acknowledgement checks run by path/risk plus scheduled assurance.
- **Validation:** Lockfile/manifests trigger dependency checks; docs-only and
  unrelated source changes skip unrelated expensive security jobs; scheduled run
  still performs full assurance.
- **Files:** `.github/workflows/security.yml`, `.github/workflows/codeql.yml`,
  `.github/workflows/rust.yml`, `scripts/license-check.sh`
- **Coordinates with:** SEC, ATTRIB, DOCGOV
- **Completed:** 2026-05-11 — `.github/workflows/security.yml` now triggers on
  push/PR/schedule/workflow_dispatch; the weekly Monday 06:15 UTC cron and the
  manual dispatch path skip `detect-changes` so every job runs as a full
  assurance sweep. Per-PR jobs gate on classifier signals: Semgrep on
  `source-changed` or `rust-changed`, dependency-audit (Trivy) and
  license-check on `dependency-audit-required` (lockfile/manifest moves), and
  the secret scan on the broader `code-changed` signal. `rust.yml` adds a new
  `rust-deps-changed` `dorny/paths-filter` output covering manifests, lockfile,
  toolchain, and the workflow file itself; cargo-deny and the acknowledgements
  freshness gate consume that signal so pure Rust source edits no longer trigger
  the dependency-graph jobs. CodeQL already gated `analyze-js`/`analyze-rust` on
  source-changed/rust-changed plus a weekly schedule and did not need targeting
  changes. `pnpm test:ci-security-targeting` locks the new contract via
  `scripts/ci/security-targeting.test.sh`.
- **Confidence:** medium

---

### CICD-008: Matrix and platform execution targeting

- **Status:** Complete
- **Intent:** Reserve macOS, Windows, cross-compile, NAPI, and benchmark matrices
  for changes that need platform evidence.
- **Expected Outcome:** Platform matrices run for platform-sensitive paths,
  release candidate/tag workflows, nightly assurance, or explicit dispatch.
- **Validation:** Non-platform PRs do not start macOS/Windows matrix jobs;
  NAPI/release/platform changes still trigger the required matrix.
- **Files:** `.github/workflows/ci.yml`, `.github/workflows/rust.yml`,
  `.github/workflows/napi.yml`, `.github/workflows/bench.yml`,
  `.github/workflows/ci-nightly.yml`
- **Coordinates with:** RELORCH, DIST surfaces
- **Completed:** 2026-05-11 — `rust.yml` `cross-compile` no longer fires on
  push to `dev`. The new gate is `workflow_dispatch` OR
  ((push to `main` OR push to `release/*`) OR PR to `main`) AND `rust-changed`,
  with `workflow_dispatch` intentionally bypassing the rust-changed guard so
  operators can force a verification run on any ref. `rust.yml` gains a
  `workflow_dispatch: {}` trigger. `ci.yml` `test-release-gate` (macOS +
  Windows Node tests) now gates on `source-changed` so docs-only release PRs
  and docs-only release-sync pushes skip the 10x/2x matrix. `napi.yml`,
  `bench.yml`, and `ci-nightly.yml` already match the spec — path-gated NAPI
  matrix, release-gated bench, schedule-only nightly cross-platform Node
  tests — and are locked by the fixture instead of being modified.
  `scripts/ci/matrix-targeting.test.sh` (`pnpm test:ci-matrix-targeting`)
  asserts each matrix's gating, including the explicit `refs/heads/dev`
  prohibition in the rust cross-compile if-gate. `.github/workflows/README.md`
  now documents the matrix-targeting contract per workflow and surfaces the
  `workflow_dispatch` operator path.
- **Validation Run:** `pnpm test:ci-matrix-targeting`,
  `pnpm test:ci-integration`, `pnpm test:ci-fast-pr`, `pnpm test:ci-classify`,
  `pnpm format:check`, `pnpm lint:md`,
  `node -e '... yaml.parse ...'` on `ci.yml` and `rust.yml`.
- **Confidence:** medium

---

### CICD-009: Release candidate readiness workflow

- **Status:** Complete
- **Intent:** Create a CI readiness gate for an explicit SHA before release tags
  are pushed.
- **Expected Outcome:** Candidate workflow records readiness evidence, candidate
  metadata, optional non-publishing artefacts, and failure diagnostics for a
  selected SHA.
- **Validation:** Manual or API-triggered candidate run emits candidate evidence
  and refuses ambiguous branch inputs.
- **Files:** `.github/workflows/`, `scripts/release/` when RELORCH command surface
  exists
- **Coordinates with:** OPMODEL-005, RELORCH-001, RELORCH-002
- **Completed:** 2026-05-10 — `.github/workflows/release-readiness.yml` shipped
  via PR #1398. The workflow is `workflow_dispatch`-only, enforces exact
  `sourceSha` checkout, validates reachability from `main` (or `migration-dev`
  for explicit compatibility probes), runs the required readiness checks
  (`pnpm format:check`, `pnpm lint:md`, guidance and classifier fixtures), and
  emits candidate metadata as a versioned artefact with bounded retention. It
  carries `permissions: contents: read` only — no tag, registry, OIDC, or
  deployment credentials are available, matching the OPMODEL-005 spec.
- **Confidence:** medium

---

### CICD-010: Workflow decomposition and consolidation

- **Status:** Complete
- **Intent:** Migrate from overlapping tool-centric workflows to validation
  contracts: PR, integration, assurance, candidate, publish.
- **Expected Outcome:** Existing workflows either consolidate or clearly map to
  one of the target contracts without duplicated setup or duplicated authority.
- **Validation:** Workflow README and job summaries identify each workflow's
  contract; duplicate execution paths are removed or justified.
- **Files:** `.github/workflows/README.md`,
  `scripts/ci/workflow-contracts.test.sh`, `.github/workflows/ci.yml`,
  `package.json`
- **Coordinates with:** DOCGOV closeout, OPMODEL-012
- **Completed:** 2026-05-11 — `.github/workflows/README.md` now opens with a
  five-contract validation model (PR validation, Integration push, Assurance,
  Release candidate, Publish) and a Workflow Contract Map table that lists
  every file under `.github/workflows/` (excluding `*.example`) with its
  contract, trigger surface, and owner module. An Authority Audit subsection
  enumerates each previously-overlapping surface — `Dependency Audit (PR)` vs
  `Dependency Audit` (resolved by CICD-005), Semgrep vs CodeQL (distinct
  tools), `metadata-validation` infra-static vs `infra.yml` Pulumi (distinct
  contracts), `Integration Readiness` aggregate vs per-job statuses (the
  aggregate fails on any non-success/skipped required job) — confirming that
  no duplicate authority remains. Filenames are intentionally not consolidated
  yet (the spec permits gradual migration and CICD-012 owns the post-cutover
  cleanup of stale `dev` triggers). `scripts/ci/workflow-contracts.test.sh`
  (`pnpm test:ci-workflow-contracts`) enforces the map: every YAML workflow
  must appear backtick-quoted in the README, the contract names must all be
  present, and the CICD-005 authority-audit credit must remain so a regression
  that re-introduces the duplicated dependency-audit gate is caught.
- **Validation Run:** `pnpm test:ci-workflow-contracts`,
  `pnpm test:ci-integration`, `pnpm test:ci-matrix-targeting`,
  `pnpm test:ci-drift-integration`, `pnpm test:ci-fast-pr`,
  `pnpm format:check`, `pnpm lint:md`,
  `node -e '... yaml.parse ...'` on `.github/workflows/ci.yml`.
- **Confidence:** medium

---

### CICD-011: APS/repo/release drift checks in CI

- **Status:** Complete
- **Intent:** Add warning-mode deterministic drift checks that connect APS intent,
  changed files, PR metadata, release candidates, and release records.
- **Expected Outcome:** CI reports missing APS references, inconsistent module
  counts, stale validation metadata, release-note gaps, and shipped-state drift.
- **Validation:** Fixture tests cover each drift class; warning output appears on a
  controlled PR.
- **Files:** `scripts/aps/drift-check.mjs`, `scripts/aps/_test/drift-check.test.sh`,
  `scripts/ci/drift-check-integration.test.sh`, `.github/workflows/ci.yml`,
  `package.json`
- **Coordinates with:** OPMODEL-010, DOCGOV-005, RELORCH
- **Completed:** 2026-05-11 — `scripts/aps/drift-check.mjs` gains two new
  flags (`--pr-title`, `--pr-body-file`) and two new finding codes
  (`pr-missing-aps-reference`, `pr-aps-reference-unknown`). `ci.yml`'s
  `aps-drift` job now captures `${{ github.event.pull_request.body }}` to a
  file and invokes drift-check.mjs with PR metadata on `pull_request` events;
  push events keep the changed-files-only invocation. The job remains
  `continue-on-error: true` — drift is warning-mode evidence, not a gate.
  Findings opt out cleanly via an `Unplanned-work:` line in the PR body,
  matching the operating-model rule for unplanned work. The existing
  `scripts/aps/_test/drift-check.test.sh` fixture (`pnpm test:aps-drift`) now
  covers the four PR-metadata cases — missing reference, known reference,
  unknown reference, `Unplanned-work:` opt-out — alongside the existing drift
  classes from OPMODEL-010 (progress mismatch, index mismatch, complete
  without validation evidence, changed file without APS reference, candidate
  missing merged item, shipped without release record, release/tag mismatch,
  package/tag mismatch, artifact missing integrity).
  `scripts/ci/drift-check-integration.test.sh` (`pnpm test:ci-drift-integration`)
  locks the workflow wiring: `aps-drift` is warning-mode, push and PR each
  invoke drift-check.mjs with the correct flags, and drift-check.mjs still
  exposes the PR-metadata surface the workflow depends on. The
  metadata-validation job runs the new fixture alongside the existing CI
  fixtures.
- **Validation Run:** `pnpm test:aps-drift`, `pnpm test:ci-drift-integration`,
  `pnpm test:ci-integration`, `pnpm test:ci-fast-pr`,
  `pnpm test:ci-matrix-targeting`, `pnpm format:check`, `pnpm lint:md`,
  `node -e '... yaml.parse ...'` on `.github/workflows/ci.yml`.
- **Confidence:** medium

---

### CICD-012: Main-first cutover readiness for validation workflows

- **Status:** Complete
- **Intent:** Prepare validation workflows so `OPMODEL-012` can retarget normal
  work from `dev` to `main` without redesigning CI/CD.
- **Expected Outcome:** Workflow triggers, branch guards, candidate readiness, and
  documentation distinguish compatibility `dev` mode from target `main` mode.
- **Validation:** Dry-run or review evidence shows the same contracts can operate
  with `dev` today and `main` after cutover.
- **Files:** `.github/workflows/`, `.github/PULL_REQUEST_TEMPLATE.md`,
  `docs/guides/branching-strategy.md`, `docs/guides/worktree-policy.md`
- **Blocks on:** OPMODEL-001, OPMODEL-002, OPMODEL-005, OPMODEL-007
- **Coordinates with:** OPMODEL-012
- **Completed:** 2026-05-11 — `.github/workflows/ci.yml` and
  `.github/workflows/rust.yml` now gate the cross-platform release matrix on
  the head pattern AND on `head.repo.full_name == github.repository`, so
  normal `feat/*` PRs and fork PRs do not fire the expensive matrix even
  after the cutover retargets normal work at `main`; release/hotfix PRs from
  the canonical repo and pushes to `main` still do. The fork-reject clause
  makes both gates self-defending, so retiring `pr-base-guard.yml` under
  OPMODEL-012 does not create a fork-trust gap.
  `.github/workflows/pr-base-guard.yml` carries a `MIGRATION-MODE GUARD`
  header spelling out the retirement sequence: verify the new gates'
  `head.repo.full_name` clause is present first, then delete (or rewrite to a
  label-based gate). `.github/workflows/release-readiness.yml` carries an
  inline CICD-012/OPMODEL-012 comment naming the `migration-dev` option as
  the artefact to retire post-cutover. The PR template references both modes
  so contributors choose the right base branch.
  `docs/guides/branching-strategy.md` gains a "Cutover-aware CI gates"
  subsection mapping every dual-mode gate to its migration and target
  triggers, plus a per-group audit naming the other workflows (already
  dual-mode, tag/schedule/dispatch-driven, intentionally `main`-only
  post-merge). `scripts/ci/cutover-readiness.test.sh` (wired as
  `pnpm test:ci-cutover-readiness`) locks the dual-mode invariants:
  integration workflows fire on both `dev` and `main`, release-class gates
  use the head allowlist AND the `head.repo.full_name` fork-reject clause,
  `pr-base-guard.yml` self-identifies as migration-only and keeps its
  fork-reject behaviour until the gate hardening lands, and
  `release-readiness.yml` already speaks the `main` / `migration-dev`
  vocabulary.
  **Remaining sweep owned by OPMODEL-012** (not blocking this work item):
  remove the hard-coded `dev` branch entries in `ci.yml` push/PR base lists,
  `rust.yml` push branch list + `cross-compile` push condition, and
  `security.yml` push/PR base lists once `dev` is retired; delete or rewrite
  `pr-base-guard.yml` per its in-file retirement sequence; remove the
  `migration-dev` option from `release-readiness.yml`. **Known forward gap:**
  the gates do not declare a `merge_group` event — merge-queue commits would
  not fire the cross-platform / cross-compile matrices. Merge queue is not
  currently enabled; adding `merge_group: {}` support is a separate
  follow-up if/when merge queue lands.
- **Confidence:** medium
