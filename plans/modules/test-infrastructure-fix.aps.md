# Test Infrastructure Fix

| ID   | Owner      | Status |
| ---- | ---------- | ------ |
| TFIX | @eddacraft | Ready  |

## Purpose

The CI pipeline has critical blind spots: the E2E harness is hard-disabled
(`if: false`), neither CI workflow installs the OPA binary so all Rego tests are
mocked, and coverage numbers are only visible locally. Every other test
improvement in TCOV, TINT, and TEXT is unverifiable in CI until these gaps are
closed.

This module re-enables the existing test surfaces, installs missing test
dependencies, and adds advisory coverage reporting to PRs. It deliberately does
not write new tests or enforce thresholds — those concerns belong to downstream
modules.

## In Scope

- Re-enable the E2E harness job in `ci.yml`
- Stabilise adapter/fixture drift that caused the E2E disable
- Install OPA binary in both `ci.yml` and `rust.yml`
- Add `cargo-llvm-cov` to `rust.yml` for Rust coverage
- Advisory coverage reporting on PRs (TS via vitest, Rust via llvm-cov)
- Coverage output as PR comment or job summary — no blocking threshold

## Out of Scope

- Hard coverage gates or blocking thresholds (deferred to a future TFIX follow-up
  once baselines are stable)
- Writing new tests (TCOV)
- New E2E test cases (TCOV, TINT)
- External service test infrastructure (TEXT)

## Interfaces

**Depends on:**

- `.github/workflows/ci.yml` — existing CI pipeline
- `.github/workflows/rust.yml` — Rust CI pipeline
- `apps/e2e/` — existing E2E harness (disabled but functional)
- `packages/anvil/runtime/src/gate/__fixtures__/policies/` — Rego test fixtures

**Exposes:**

- Working E2E harness in CI
- OPA binary available in CI for downstream test modules
- Advisory coverage reports on every PR

## Decisions

**D-TFIX-001:** Coverage reporting format

- **Options:** (a) GitHub Actions job summary only, (b) PR comment via bot,
  (c) Third-party service (Codecov, Coveralls)
- **Resolution:** Option (a) — job summary. Zero external dependencies, no
  tokens to manage, visible in the Actions tab. PR comments can be added later
  if the summary proves insufficient.
- **Status:** Resolved

**D-TFIX-002:** OPA binary version pinning

- **Options:** (a) Pin to specific version, (b) Use `latest`, (c) Match the
  version in `opa-binary-manager.ts`
- **Resolution:** Option (c) — match the version the TS binary manager downloads.
  Avoids version skew between CI and local dev.
- **Status:** Resolved

**D-TFIX-003:** E2E harness trigger scope

- **Options:** (a) Run on every PR, (b) Run only when CLI/core/runtime files
  change, (c) Run on schedule (nightly)
- **Resolution:** Option (b) — affected-path filtering. Keeps CI fast for
  docs-only or frontend-only PRs. Use the existing `code-changed` detection
  pattern from the `test` job.
- **Status:** Resolved

## Risks

| Risk                                          | Impact | Mitigation                                              |
| --------------------------------------------- | ------ | ------------------------------------------------------- |
| E2E harness still flaky after fixture fixes   | medium | Add retry logic (2 attempts); track flaky test registry |
| OPA download slows CI by 10-15s               | low    | Cache the binary via actions/cache keyed on version     |
| Coverage reporting adds noise to PR summaries | low    | Collapsible `<details>` block in summary                |

## Ready Checklist

Change status to **Ready** when:

- [x] Purpose and scope are clear
- [x] Dependencies identified
- [x] All tasks defined
- [x] Technology decisions resolved
- [x] Risks catalogued with mitigations

## Tasks

### Phase 1 — CI Fixes

#### TFIX-001: diagnose and fix E2E harness fixture drift

- **Intent:** The E2E harness was disabled due to "adapter API drift, fixture
  schema drift." Identify which fixtures are stale and update them to match
  current schemas.
- **Expected Outcome:** All existing E2E tests in `apps/e2e/` pass locally
  against the current codebase.
- **Files:**
  - `apps/e2e/src/helpers/cli-runner.ts`
  - `apps/e2e/src/smoke/smoke.e2e.test.ts`
  - `apps/e2e/src/adapters/format-roundtrip.e2e.test.ts`
  - `apps/e2e/src/cli/commands.e2e.test.ts`
  - `apps/e2e/src/cli/gate-workflow.e2e.test.ts`
- **Dependencies:** —
- **Validation:** `pnpm --filter @eddacraft/anvil-e2e test` passes with zero
  failures.
- **Confidence:** medium — fixture drift scope unknown until investigated

#### TFIX-002: re-enable E2E harness job in CI

- **Intent:** Remove `if: false` from the `e2e-harness` job and wire it to the
  `code-changed` path filter.
- **Expected Outcome:** The E2E harness runs automatically on PRs that touch
  CLI, core, runtime, or adapter code.
- **Files:**
  - `.github/workflows/ci.yml`
- **Dependencies:** TFIX-001
- **Validation:** Open a PR touching `packages/anvil/core/` — E2E harness job
  appears and passes in the Actions tab.
- **Confidence:** high

#### TFIX-003: install OPA binary in ci.yml

- **Intent:** Make the `opa` binary available for TS policy tests in CI.
  Version-matched to `opa-binary-manager.ts`.
- **Expected Outcome:** `opa version` succeeds in the `test` job. Existing
  policy tests that skip when OPA is absent now run.
- **Files:**
  - `.github/workflows/ci.yml`
  - `packages/anvil/policy/src/opa-binary-manager.ts` (read version)
- **Dependencies:** —
- **Validation:** CI `test` job log shows OPA version output; any tests gated
  on OPA presence now execute.
- **Confidence:** high

#### TFIX-004: install OPA binary in rust.yml

- **Intent:** Make the `opa` binary available for Rust policy crate tests.
- **Expected Outcome:** `crates/anvil-policy/` tests that invoke OPA run in CI.
- **Files:**
  - `.github/workflows/rust.yml`
- **Dependencies:** —
- **Validation:** `cargo test -p eddacraft-anvil-policy` in CI runs OPA-dependent tests.
- **Confidence:** high

#### TFIX-005: cache OPA binary in CI

- **Intent:** Avoid re-downloading OPA on every CI run. Use `actions/cache`
  keyed on the pinned version.
- **Expected Outcome:** OPA download step shows cache hit on subsequent runs.
- **Files:**
  - `.github/workflows/ci.yml`
  - `.github/workflows/rust.yml`
- **Dependencies:** TFIX-003, TFIX-004
- **Validation:** Second CI run shows `Cache hit` in the OPA install step.
- **Confidence:** high

### Phase 2 — Coverage Reporting

#### TFIX-006: add TS coverage summary to CI job output

- **Intent:** Surface vitest coverage numbers in the GitHub Actions job summary
  so reviewers can see impact without running locally.
- **Expected Outcome:** The `test` job summary includes a collapsible table of
  per-project line and branch coverage.
- **Files:**
  - `.github/workflows/ci.yml`
- **Dependencies:** —
- **Validation:** PR Actions summary contains a "Test Coverage" section with
  per-project percentages.
- **Confidence:** high

#### TFIX-007: add cargo-llvm-cov to rust.yml

- **Intent:** Generate Rust coverage reports in CI using `cargo-llvm-cov`.
- **Expected Outcome:** Rust `test` job produces coverage output and includes
  per-crate percentages in the job summary.
- **Files:**
  - `.github/workflows/rust.yml`
- **Dependencies:** —
- **Validation:** Rust CI job summary shows per-crate line coverage table.
- **Confidence:** high

#### TFIX-008: combined coverage summary step

- **Intent:** Add a final CI step that merges TS and Rust coverage into a single
  summary block, making the total picture visible at a glance.
- **Expected Outcome:** Job summary includes both stacks in one table with
  monorepo totals.
- **Files:**
  - `.github/workflows/ci.yml` (or a shared composite action)
- **Dependencies:** TFIX-006, TFIX-007
- **Validation:** PR summary shows combined TS + Rust coverage table.
- **Confidence:** medium — may need a lightweight script to merge formats

#### TFIX-009: add E2E retry logic for flaky tests

- **Intent:** Mitigate intermittent E2E failures with a 2-attempt retry strategy
  at the test runner level. Track which tests are retried for future cleanup.
- **Expected Outcome:** Flaky tests get one retry before failing the job.
  Retried tests are logged for visibility.
- **Files:**
  - `apps/e2e/vitest.config.ts` (or playwright config)
  - `.github/workflows/ci.yml`
- **Dependencies:** TFIX-002
- **Validation:** A deliberately flaky test (skip on first run, pass on second)
  passes in CI with a retry log entry.
- **Confidence:** high

#### TFIX-010: document test infrastructure in AGENTS.md

- **Intent:** Update AGENTS.md with the test infrastructure topology — which
  tests run where, how to run coverage locally, where reports land.
- **Expected Outcome:** New contributor can find and run any test category from
  the documentation alone.
- **Files:**
  - `AGENTS.md`
- **Dependencies:** TFIX-001 through TFIX-008
- **Validation:** Manual review — a developer unfamiliar with the repo can
  follow the docs to run all test categories.
- **Confidence:** high
