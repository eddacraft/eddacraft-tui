# Test Infrastructure Fix

| ID   | Owner      | Status   |
| ---- | ---------- | -------- |
| TFIX | @eddacraft | Complete |

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
- **Status:** Resolved — CI workflows pin OPA to `v0.60.0` to match
  `DEFAULT_OPA_VERSION` (`0.60.0` as of 2026-04-17 in
  `packages/anvil/policy/src/opa-binary-manager.ts`).

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
  - `apps/e2e/src/helpers/cli-runner.ts` — retarget at Rust binary, graceful
    skip when absent
  - `apps/e2e/src/helpers/fixtures.ts` — APS/SpecKit fixtures updated to match
    current detection indicators; anti-pattern fixture switched from
    non-existent secret detection to real pattern triggers
  - `apps/e2e/src/smoke/smoke.e2e.test.ts` — allow `/health` 200-or-503 and
    swap obsolete TS-CLI check for Rust-binary presence probe
  - `apps/e2e/src/adapters/format-roundtrip.e2e.test.ts` — no change (drove
    fixture updates)
  - `apps/e2e/src/core/drift-detection.e2e.test.ts` — match new
    `validateSnapshot` return shape (`{ success, data?, error? }`)
  - `apps/e2e/src/contracts/schema-compat.e2e.test.ts` — EvidenceEntrySchema
    status enum (`passed|failed|skipped|warning`)
  - `apps/e2e/src/mcp/server-tools.e2e.test.ts` — `generateMcpConfig(target)`
    positional signature
  - `apps/e2e/src/cli/commands.e2e.test.ts`,
    `apps/e2e/src/cli/gate-workflow.e2e.test.ts` — `describe.skip` when
    Rust CLI binary absent
  - `apps/e2e/package.json` — `pretest*` hooks build
    `@eddacraft/transactional` so anvil-api dist import resolves
  - `apps/e2e/vitest.config.ts` — flatten vitest 4 `poolOptions`, add
    `@eddacraft/anvil-policy` alias, add `retry: 1` (shared with TFIX-009)
- **Dependencies:** —
- **Validation:** `pnpm --filter @eddacraft/anvil-e2e test` — 67 passed,
  13 skipped (CLI suites, Rust binary absent in TS-only env).
- **Confidence:** medium — fixture drift scope unknown until investigated
- **Status:** Complete

#### TFIX-002: re-add E2E harness job in CI

- **Intent:** The `e2e-harness` job was fully removed from `ci.yml` (see the
  trailing `NOTE:` block at end of the `jobs:` section) rather than toggled
  with `if: false`. Re-add the job and wire it to the `code-changed` /
  `e2e-changed` path filter exposed by `.github/actions/detect-changes`.
- **Expected Outcome:** The E2E harness runs automatically on PRs that touch
  CLI, core, runtime, or adapter code; docs-only PRs still skip.
- **Files:**
  - `.github/workflows/ci.yml` — new `e2e-harness` job gated on
    `source-changed || e2e-changed`
  - `.github/actions/detect-changes/` (verified — `source-changed` covers
    apps/packages/tools/infra; `e2e-changed` covers `apps/e2e/**`)
- **Dependencies:** TFIX-001
- **Validation:** Open a PR touching `packages/anvil/core/` — E2E harness job
  appears and passes in the Actions tab.
- **Confidence:** high
- **Status:** Complete

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
- **Status:** Complete — `ci.yml` test job installs OPA pinned to
  `v0.60.0` (DEFAULT_OPA_VERSION).

#### TFIX-004: install OPA binary in rust.yml

- **Intent:** Make the `opa` binary available for Rust policy crate tests.
- **Expected Outcome:** `crates/anvil-policy/` tests that invoke OPA run in CI.
- **Files:**
  - `.github/workflows/rust.yml`
- **Dependencies:** —
- **Validation:** `cargo test -p eddacraft-anvil-policy` in CI runs OPA-dependent tests.
- **Confidence:** high
- **Status:** Complete — `rust.yml` uses `open-policy-agent/setup-opa@v2.3.0`
  with OPA pinned to `v0.60.0`.

#### TFIX-005: cache OPA binary in CI

- **Intent:** Avoid re-downloading OPA on every CI run.
- **Expected Outcome:** OPA install step is fast and idempotent on subsequent runs.
- **Files:**
  - `.github/workflows/ci.yml`
  - `.github/workflows/rust.yml`
- **Dependencies:** TFIX-003, TFIX-004
- **Validation:** Second CI run is fast in the OPA install step.
- **Confidence:** high
- **Status:** Complete via `open-policy-agent/setup-opa` action, which handles
  binary caching natively (no separate `actions/cache` step required). Cache
  key becomes deterministic once D-TFIX-002 version pinning is applied.

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
- **Status:** Complete — `Unit Tests` job emits TypeScript Coverage table.

#### TFIX-007: add cargo-llvm-cov to rust.yml

- **Intent:** Generate Rust coverage reports in CI using `cargo-llvm-cov`.
- **Expected Outcome:** Rust `test` job produces coverage output and includes
  per-crate percentages in the job summary.
- **Files:**
  - `.github/workflows/rust.yml`
- **Dependencies:** —
- **Validation:** Rust CI job summary shows per-crate line coverage table.
- **Confidence:** high
- **Status:** Complete — `rust.yml` installs cargo-llvm-cov (cached) and emits
  a Rust Coverage summary.

#### TFIX-008: combined coverage summary step

- **Intent:** Add a final CI step that merges TS and Rust coverage into a single
  summary block, making the total picture visible at a glance.
- **Expected Outcome:** Job summary includes both stacks in one table with
  monorepo totals.
- **Files:**
  - `.github/workflows/ci.yml` (or a shared composite action, or a new
    `coverage-aggregate.yml` workflow triggered by `workflow_run`)
- **Dependencies:** TFIX-006, TFIX-007
- **Validation:** PR summary shows combined TS + Rust coverage table.
- **Confidence:** medium — may need a lightweight script to merge formats
- **Status:** Deferred — TFIX-006 and TFIX-007 now emit separate summaries in
  their own workflows (ci.yml test job for TS, rust.yml test job for Rust).
  Both are visible in the PR's Checks tab. A true combined table requires
  either a `workflow_run`-triggered aggregator that downloads both artifacts
  and posts a PR comment, or moving Rust coverage into ci.yml (which conflicts
  with the separate rust.yml path-filter design). Revisit once PR comment
  bot or shared composite action is justified.

#### TFIX-009: add E2E retry logic for flaky tests

- **Intent:** Mitigate intermittent E2E failures with a 2-attempt retry strategy
  at the test runner level. Track which tests are retried for future cleanup.
- **Expected Outcome:** Flaky tests get one retry before failing the job.
  Retried tests are logged for visibility.
- **Files:**
  - `apps/e2e/vitest.config.ts` — `retry: 1` top-level
- **Dependencies:** TFIX-002
- **Validation:** Retried tests appear with `(retry x1)` in vitest output; real
  failures still fail the job.
- **Confidence:** high
- **Status:** Complete

#### TFIX-011: Regal lint + `opa test` for Rego fixtures in rust.yml

- **Intent:** Lint and test the fixture Rego policies in CI (previously only
  reachable via `opa test` locally). Added opportunistically alongside TFIX-004.
- **Expected Outcome:** `opa test --verbose policies/fixtures/` and
  `regal lint --format github policies/fixtures/` run on every Rust CI run.
- **Files:**
  - `.github/workflows/rust.yml` (lines 53-62)
  - `policies/fixtures/*.rego`
- **Dependencies:** TFIX-004
- **Validation:** Rust CI job contains passing "Run OPA policy tests" and
  "Lint policies with Regal" steps.
- **Confidence:** high
- **Status:** Complete — already shipped in `rust.yml`. **Overlap:** partially
  subsumes TCOV-011 for the Rust-side fixtures; TCOV-011 may narrow to the TS
  gate-runner invocation path or mark this task as sufficient for CI coverage.

#### TFIX-010: document test infrastructure in AGENTS.md

- **Intent:** Update AGENTS.md with the test infrastructure topology — which
  tests run where, how to run coverage locally, where reports land.
- **Expected Outcome:** New contributor can find and run any test category from
  the documentation alone.
- **Files:**
  - `AGENTS.md` — new **Test Infrastructure** section covering stack matrix,
    local commands, coverage output, E2E conventions, and OPA/Regal pinning.
- **Dependencies:** TFIX-001 through TFIX-008, TFIX-011
- **Validation:** Manual review — a developer unfamiliar with the repo can
  follow the docs to run all test categories.
- **Confidence:** high
- **Status:** Complete
