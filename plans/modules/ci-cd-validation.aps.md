<!--
APS Module: CI/CD Validation Operating Model
============================================
Implements the CI/CD, validation, and pipeline-efficiency layer for the target
Plan / Build / Release operating model. See: plans/aps-rules.md
-->

# CI/CD Validation Operating Model

| ID   | Owner | Status   | Progress |
| ---- | ----- | -------- | -------- |
| CICD | —     | In Progress | 3/12     |

**Spec:** [2026-05-10 CI/CD And Validation Operating Model](../specs/2026-05-10-ci-cd-validation-operating-model.md)
**Operating model:** [2026-05-09 Plan / Build / Release Operating Model](../specs/2026-05-09-plan-build-release-operating-model.md)
**Execution architecture:** [2026-05-09 Agentic Execution Ecosystem Architecture](../specs/2026-05-09-agentic-execution-ecosystem-architecture.md)
**Review architecture:** [2026-05-09 Council Agent And Skill Change Proposal](../specs/2026-05-09-council-agent-skill-change-proposal.md)
**Council review:** [2026-05-10 CICD Validation Council Review](../reviews/2026-05-10-cicd-validation-council.md) — converged, no open findings.

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
[`plans/aps-rules.md#cross-cutting-modules`](../aps-rules.md#cross-cutting-modules).
CICD owns CI/CD cost, validation layering, shared classification, local-first
validation commands, and workflow decomposition. It coordinates with specialist
modules rather than absorbing their authority.

## Migration Boundary

The target operating model is `main`-first, but this repository remains
`dev`-first until `OPMODEL-012` completes.

Therefore:

- Normal PR validation continues to target `dev` until `OPMODEL-012`.
- `main` validation remains release/hotfix/current compatibility work until
  cutover.
- Any target-state `main` workflow changes must be migration-safe and clearly
  labelled.
- This module must not retire `dev`, change branch protections, or alter normal
  branch authority; OPMODEL owns that transition.

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

- **Status:** Proposed
- **Intent:** Make every PR receive cheap, deterministic, path-targeted validation
  without routine coverage, broad matrices, or unrelated security work.
- **Expected Outcome:** PR validation runs format/lint/typecheck/affected tests
  and metadata checks selected by the classifier.
- **Validation:** Representative docs-only, TS-only, Rust-only, and mixed PRs run
  the expected jobs and skip unrelated expensive jobs.
- **Files:** `.github/workflows/ci.yml`, `.github/workflows/rust.yml`,
  `.github/actions/`
- **Coordinates with:** OPMODEL migration boundary; normal PR target remains `dev`
  until OPMODEL-012
- **Confidence:** medium

---

### CICD-005: Integration SHA validation redesign

- **Status:** Proposed
- **Intent:** Separate merged-SHA validation from PR feedback and reduce duplicate
  execution on pushes to the integration branch.
- **Expected Outcome:** Push validation proves the `dev` integration SHA during
  migration, then `main` after cutover, with a distinct readiness contract.
- **Validation:** Push to integration branch runs integration readiness checks and
  does not duplicate PR-only commentary or coverage work.
- **Files:** `.github/workflows/ci.yml`, `.github/workflows/rust.yml`,
  `.github/workflows/security.yml`
- **Coordinates with:** OPMODEL-012
- **Confidence:** medium

---

### CICD-006: Coverage and artefact retention cost controls

- **Status:** Proposed
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
- **Confidence:** high

---

### CICD-007: Security and dependency assurance targeting

- **Status:** Proposed
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
- **Confidence:** medium

---

### CICD-008: Matrix and platform execution targeting

- **Status:** Proposed
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
- **Confidence:** medium

---

### CICD-009: Release candidate readiness workflow

- **Status:** Proposed
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
- **Confidence:** medium

---

### CICD-010: Workflow decomposition and consolidation

- **Status:** Proposed
- **Intent:** Migrate from overlapping tool-centric workflows to validation
  contracts: PR, integration, assurance, candidate, publish.
- **Expected Outcome:** Existing workflows either consolidate or clearly map to
  one of the target contracts without duplicated setup or duplicated authority.
- **Validation:** Workflow README and job summaries identify each workflow's
  contract; duplicate execution paths are removed or justified.
- **Files:** `.github/workflows/`, `.github/workflows/README.md`
- **Coordinates with:** DOCGOV closeout, OPMODEL-012
- **Confidence:** medium

---

### CICD-011: APS/repo/release drift checks in CI

- **Status:** Proposed
- **Intent:** Add warning-mode deterministic drift checks that connect APS intent,
  changed files, PR metadata, release candidates, and release records.
- **Expected Outcome:** CI reports missing APS references, inconsistent module
  counts, stale validation metadata, release-note gaps, and shipped-state drift.
- **Validation:** Fixture tests cover each drift class; warning output appears on a
  controlled PR.
- **Files:** `scripts/ci/`, `plans/`, `.github/workflows/`
- **Coordinates with:** OPMODEL-010, DOCGOV-005, RELORCH
- **Confidence:** medium

---

### CICD-012: Main-first cutover readiness for validation workflows

- **Status:** Proposed
- **Intent:** Prepare validation workflows so `OPMODEL-012` can retarget normal
  work from `dev` to `main` without redesigning CI/CD.
- **Expected Outcome:** Workflow triggers, branch guards, candidate readiness, and
  documentation distinguish compatibility `dev` mode from target `main` mode.
- **Validation:** Dry-run or review evidence shows the same contracts can operate
  with `dev` today and `main` after cutover.
- **Files:** `.github/workflows/`, `.github/actions/`, `.github/PULL_REQUEST_TEMPLATE.md`,
  `docs/guides/branching-strategy.md`, `docs/guides/worktree-policy.md`
- **Blocks on:** OPMODEL-001, OPMODEL-002, OPMODEL-005, OPMODEL-007
- **Coordinates with:** OPMODEL-012
- **Confidence:** medium
