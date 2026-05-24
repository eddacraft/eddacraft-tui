<!--
APS Module: APS Canonical Alignment
===================================
Tracks migration from Anvil's original APS dialect to the canonical
anvil-plan-spec v0.3.0 surface while preserving Anvil-specific release and
operating-model extensions.
-->

# APS Canonical Alignment

| ID     | Owner | Status | Progress |
| ------ | ----- | ------ | -------- |
| APSCAN | —     | In Progress | 1/11     |

## Purpose

Bring Anvil's APS authoring conventions, validation, agent guidance, and active
plan files back into alignment with the canonical `anvil-plan-spec` repository
without rewriting historical archive content or losing Anvil-specific release
lifecycle semantics.

## In Scope

- Active APS module shape, field names, status semantics, and execution-plan
  naming.
- Local APS parser, validator, drift checks, docs checks, and agent guidance.
- Canonical CLI compatibility decisions for `aps lint`, `aps next`, `aps start`,
  `aps complete`, and `aps graph`.
- Migration path for active modules first, with archived legacy plans excluded
  from canonical enforcement unless touched.
- Project-specific extensions for release metadata and Anvil's
  Plan / Build / Release lifecycle.

## Out of Scope

- Bulk rewriting archived or legacy APS files solely for conformance.
- Removing Anvil-specific release lifecycle labels from historical prose.
- Making APS a user prerequisite for Anvil product value; planless-first remains
  the product posture.
- Changing canonical `anvil-plan-spec` upstream behaviour from this repo.

## Interfaces

**Depends on:**

- [`anvil-plan-spec`](https://github.com/eddacraft/anvil-plan-spec) — canonical
  APS repository, currently reviewed at `v0.3.0`.
- `plans/aps-rules.md` — current Anvil APS authoring and execution guidance.
- `plans/index.aps.md` and `plans/modules/*.aps.md` — active plan authority.
- `packages/aps/**` — Anvil's local APS parser, validator, state, and template
  implementation.
- `scripts/aps/drift-check.mjs` and `scripts/docs/check-aps.mjs` — existing
  Anvil APS drift/docs validation surfaces.
- `AGENTS.md` — repository agent lifecycle requirements.

**Exposes:**

- A tracked migration backlog for canonical APS alignment.
- Compatibility rules for active Anvil plans during migration.
- Validation boundaries that separate active APS enforcement from archive
  history.
- Documented local deviations where Anvil intentionally differs from canonical
  scaffolding.

## Local Deviations to Preserve

- Numeric module filename prefixes are useful when dependency order matters. New
  modules may use either stable kebab slugs or numeric-prefixed slugs; the index
  remains the dependency-order authority, but numeric prefixes are acceptable for
  readability in ordered migrations.
- Anvil may keep release metadata fields such as `changeType`, `releaseIntent`,
  `releaseScope`, `releaseNote`, and hold conditions as project-specific
  extensions.
- Anvil may keep release lifecycle prose labels such as `Merged` and
  `Released/Shipped` outside canonical work-item status fields.

## Migration Strategy

Migrate in waves:

1. Make tooling accept both legacy and canonical forms.
2. Switch new authored work to canonical APS terms and filenames.
3. Migrate active modules opportunistically, prioritising modules touched by
   current work.
4. Enforce canonical lint only on active APS surfaces.
5. Leave archived legacy files historical unless a future task explicitly
   reopens them.

## Work Items

### APSCAN-001: Define active APS lint scope

- **Status:** Done
- **Intent:** Prevent archived legacy plans from blocking canonical APS lint while
  making active APS conformance measurable.
- **Expected Outcome:** Canonical lint targets active APS files only and excludes
  `plans/archive/**`, legacy phase files, and historical review artefacts unless
  explicitly requested.
- **Validation:** `pnpm test:aps-active-lint`; `pnpm aps:active-lint --list-files`
  reports active-surface results without archive-only failures.
- **Files:** `scripts/aps/active-lint.mjs`,
  `scripts/aps/_test/active-lint.test.sh`, `package.json`
- **Closeout:** Validation passed with `pnpm test:aps-active-lint`,
  `node --check scripts/aps/active-lint.mjs`,
  `bash -n scripts/aps/_test/active-lint.test.sh`,
  `pnpm exec oxlint scripts/aps/active-lint.mjs`, and
  `pnpm aps:active-lint --list-files`. APS drift still reports the two
  pre-existing advisory findings for TUIR progress and ADOPT-005 release-record
  evidence.
- **Confidence:** high

### APSCAN-002: Split portable APS rules from Anvil project context

- **Status:** In Progress
- **Intent:** Let `plans/aps-rules.md` track canonical APS guidance while Anvil's
  Worktrunk, Council, release, and lifecycle extensions remain explicit local
  context.
- **Expected Outcome:** Portable APS rules are easy to refresh from canonical APS,
  and Anvil-specific rules live in a project-owned context surface linked from
  agent guidance.
- **Validation:** Manual diff against `anvil-plan-spec/scaffold/plans/aps-rules-v2.md`;
  `pnpm docs:check`
- **Files:** `plans/aps-rules.md`, `plans/project-context.md`, `AGENTS.md`
- **Confidence:** medium

### APSCAN-003: Add canonical aliases to Anvil APS parser and validator

- **Status:** In Progress
- **Intent:** Allow active plans to move from legacy Anvil terms to canonical APS
  terms without breaking existing tooling mid-migration.
- **Expected Outcome:** Anvil tooling accepts `## Work Items`, `Expected Outcome`,
  `Outcome` as a temporary alias, `.actions.md`, and canonical completion stamps
  while still reading legacy `## Tasks` and `.steps.md` references.
- **Validation:** `pnpm -F @eddacraft/anvil-aps test`; targeted parser and
  validator fixtures covering legacy and canonical forms
- **Files:** `packages/aps/src/parser/**`, `packages/aps/src/validator/**`,
  `packages/aps/src/**/__fixtures__/**`
- **Confidence:** medium

### APSCAN-004: Migrate active module headings and required fields

- **Status:** Ready
- **Intent:** Move active modules toward canonical `## Work Items` sections and
  required `Intent`, `Expected Outcome`, and `Validation` fields.
- **Expected Outcome:** Active modules touched during current work use canonical
  section names and field names; historical archive modules remain unchanged.
- **Validation:** Active-scope APS lint passes after each migrated batch.
- **Files:** `plans/modules/*.aps.md`, `plans/index.aps.md`
- **Dependencies:** APSCAN-001, APSCAN-003
- **Confidence:** medium

### APSCAN-005: Rename active execution plans to action plans

- **Status:** Ready
- **Intent:** Adopt canonical `.actions.md` naming for active execution plans while
  preserving historical `.steps.md` references.
- **Expected Outcome:** New execution plans use `plans/execution/*.actions.md`,
  active live references are renamed when touched, and archived `.steps.md` files
  remain historical.
- **Validation:** Link checks and `pnpm docs:check` pass for renamed active plans.
- **Files:** `plans/execution/**`, `plans/modules/*.aps.md`, `plans/aps-rules.md`
- **Dependencies:** APSCAN-003
- **Confidence:** medium

### APSCAN-006: Document status semantics and release metadata extensions

- **Status:** Ready
- **Intent:** Separate canonical work-item status from Anvil release lifecycle
  prose so tools and agents stop conflating execution state with release state.
- **Expected Outcome:** New work items use canonical statuses, while Anvil release
  labels and release metadata fields are documented as local extensions outside
  canonical status semantics.
- **Validation:** Manual review against `anvil-plan-spec/docs/usage.md` and
  Anvil release records; `pnpm docs:check`
- **Files:** `plans/aps-rules.md`, `plans/project-context.md`, `scripts/aps/drift-check.mjs`
- **Confidence:** medium

### APSCAN-007: Add canonical issues tracker and context package support

- **Status:** Ready
- **Intent:** Support canonical `plans/issues.md` and `.aps/context/<ID>.md`
  surfaces without disrupting Anvil's existing review and release records.
- **Expected Outcome:** New planning-level discoveries can be logged as `ISS-NNN`
  or `Q-NNN`, and generated canonical context packages are ignored and safe to
  regenerate.
- **Validation:** `aps start <ready-item>` in a disposable fixture or documented
  dry run creates ignored context output; `aps lint plans/issues.md` passes.
- **Files:** `plans/issues.md`, `.gitignore`, `.aps/**` ignore rules
- **Dependencies:** APSCAN-001
- **Confidence:** medium

### APSCAN-008: Decide canonical CLI adoption boundary

- **Status:** Ready
- **Intent:** Choose whether Anvil consumes the canonical `aps` CLI directly or
  keeps its local `@eddacraft/anvil-aps` package as a compatibility layer.
- **Expected Outcome:** A short design note records the authority split between
  canonical CLI commands and Anvil-specific drift/release checks.
- **Validation:** Design note reviewed against `anvil-plan-spec/docs/usage.md` and
  current `packages/aps/**` consumers.
- **Files:** `plans/specs/<date>-aps-cli-adoption-boundary.md`, `packages/aps/**`,
  `scripts/aps/**`
- **Confidence:** low

### APSCAN-009: Reconcile progress counters with canonical status

- **Status:** Ready
- **Intent:** Keep Anvil's useful progress summaries without making manual counts
  conflict with canonical status-driven tooling.
- **Expected Outcome:** New modules either avoid manual counters or have a clear
  drift rule; existing counter checks recognise canonical completion stamps.
- **Validation:** `pnpm aps:drift --json` reports no counter false positives for
  migrated modules.
- **Files:** `scripts/aps/drift-check.mjs`, `plans/modules/*.aps.md`,
  `plans/index.aps.md`
- **Dependencies:** APSCAN-003, APSCAN-006
- **Confidence:** medium

### APSCAN-010: Run active-module migration wave and closeout

- **Status:** Ready
- **Intent:** Apply the compatibility rules to the first active-module wave and
  prove the migration path before broader rollout.
- **Expected Outcome:** A representative batch of active modules validates under
  canonical active-scope lint, Anvil drift checks remain useful, and follow-up
  work is filed for any remaining incompatibilities.
- **Validation:** Active-scope APS lint; `pnpm aps:drift --json`; `pnpm docs:check`
- **Files:** `plans/modules/*.aps.md`, `plans/index.aps.md`, `scripts/docs/**`,
  `scripts/aps/**`
- **Dependencies:** APSCAN-001, APSCAN-002, APSCAN-003, APSCAN-006, APSCAN-009
- **Confidence:** medium

### APSCAN-011: Add APS TUI dashboard

- **Status:** In Progress
- **Intent:** Provide a read-only terminal dashboard for active APS work so
  operators and agents can see in-progress modules, ready work, blocked work, and
  local reconciliation hints without manually reading the APS index and every
  module file.
- **Expected Outcome:** `anvil plan dashboard` builds a local-only
  `PlanStatusSnapshot`, renders active APS state in a Ratatui dashboard, flags
  APS-only consistency issues, and keeps an empty v1 enrichment seam for future
  GitHub/CI annotations.
- **Validation:** `cargo test -p eddacraft-anvil plan_dashboard && cargo test -p
  eddacraft-anvil-tui plan_dashboard && pnpm format:check && pnpm docs:check`
- **Files:** `crates/anvil-cli/src/main.rs`,
  `crates/anvil-cli/src/commands/mod.rs`,
  `crates/anvil-cli/src/commands/plan.rs`,
  `crates/anvil-cli/src/plan_dashboard.rs`,
  `crates/anvil-tui/src/surfaces/mod.rs`,
  `crates/anvil-tui/src/surfaces/plan_dashboard/mod.rs`,
  `crates/anvil-tui/src/surfaces/plan_dashboard/render.rs`,
  `crates/anvil-tui/src/surfaces/plan_dashboard/event_adapter.rs`
- **Dependencies:** APSCAN-001
- **Spec:** `plans/specs/2026-05-24-aps-tui-dashboard.md`
- **Execution Plan:** `plans/execution/2026-05-24-aps-tui-dashboard.md`
- **Confidence:** medium
