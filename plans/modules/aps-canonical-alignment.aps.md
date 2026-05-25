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
| APSCAN | —     | In Progress | 4/11     |

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
- `plans/aps-rules.md` — portable APS authoring and execution guidance.
- `plans/project-context.md` — Anvil-specific workflow, release, and closeout
  context.
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

- **Status:** Merged
- **Intent:** Let `plans/aps-rules.md` track canonical APS guidance while Anvil's
  Worktrunk, Council, release, and lifecycle extensions remain explicit local
  context.
- **Expected Outcome:** Portable APS rules are easy to refresh from canonical APS,
  and Anvil-specific rules live in a project-owned context surface linked from
  agent guidance.
- **Validation:** Manual diff against `anvil-plan-spec/scaffold/plans/aps-rules-v2.md`;
  `pnpm docs:check`; `pnpm aps:drift --json`
- **Files:** `plans/aps-rules.md`, `plans/project-context.md`, `AGENTS.md`
- **Closeout:** `plans/aps-rules.md` now keeps the portable APS guidance close
  to the canonical scaffold, `plans/project-context.md` owns Anvil-specific
  workflow/release/documentation context, and `AGENTS.md` links agents to both
  surfaces. Manual comparison against
  `anvil-plan-spec/scaffold/plans/aps-rules-v2.md` confirmed the portable
  sections retain the canonical shape while forwarding anchors preserve existing
  Anvil links. Validation passed locally with `pnpm docs:check` and `pnpm
  aps:drift --json`. Merged 2026-05-24 via PR
  [#1918](https://github.com/eddacraft/anvil-001/pull/1918) at
  `64403295dbc67173f4e4715bf9d2844d9aba95f2`.
- **Confidence:** medium

### APSCAN-003: Add canonical aliases to Anvil APS parser and validator

- **Status:** Done
- **Intent:** Allow active plans to move from legacy Anvil terms to canonical APS
  terms without breaking existing tooling mid-migration.
- **Expected Outcome:** Anvil tooling accepts `## Work Items`, `Expected Outcome`,
  `Outcome` as a temporary alias, `.actions.md`, and canonical completion stamps
  while still reading legacy `## Tasks` and `.steps.md` references.
- **Validation:** `pnpm -F @eddacraft/anvil-aps test`; targeted parser and
  validator fixtures covering legacy and canonical forms
- **Files:** `packages/aps/src/parser/**`, `packages/aps/src/validator/**`,
  `packages/aps/src/**/__fixtures__/**`
- **Closeout:** Parser and validator compatibility landed for canonical
  `## Work Items` sections and the temporary `Outcome:` field alias while
  preserving legacy `## Tasks` and `Expected Outcome:` support. Existing
  lifecycle completion aliases and active-lint `.actions.md` scope remain in
  place. Validation passed with `pnpm -F @eddacraft/anvil-aps test` and
  `pnpm -F @eddacraft/anvil-aps typecheck`.
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

- **Status:** Merged 2026-05-25 via PR [#1949](https://github.com/eddacraft/anvil-001/pull/1949)
- **Intent:** Adopt canonical `.actions.md` naming for active execution plans while
  preserving historical `.steps.md` references.
- **Expected Outcome:** New execution plans use `plans/execution/*.actions.md`,
  active live references are renamed when touched, and archived `.steps.md` files
  remain historical.
- **Validation:** `pnpm format:check`, `pnpm docs:check` (7/7),
  `pnpm aps:drift --json` (no APSCAN regressions). Active-lint infrastructure
  (`scripts/aps/active-lint.mjs`) already accepts `.actions.md` and excludes
  `.steps.md`; no script change needed.
- **Files:** `plans/aps-rules.md`, `plans/project-context.md`
- **Dependencies:** APSCAN-003
- **Closeout:** `plans/aps-rules.md` "When Asked to Execute" guidance now names
  the canonical `.actions.md` suffix and the rename-when-touched policy.
  `plans/project-context.md` adds an Execution Plans section codifying the four
  rules — new plans MUST use `.actions.md`; legacy plans rename with `git mv`
  when touched; no bulk-rename of historical plans; archived plans stay put.
  No file renames in this PR — the rename-when-touched policy means renames
  land alongside the work that re-opens each plan, preserving blame history.
- **Confidence:** medium

### APSCAN-006: Document status semantics and release metadata extensions

- **Status:** In Progress
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

- **Status:** Merged 2026-05-25 via PR [#1948](https://github.com/eddacraft/anvil-001/pull/1948)
- **Intent:** Support canonical `plans/issues.md` and `.aps/context/<ID>.md`
  surfaces without disrupting Anvil's existing review and release records.
- **Expected Outcome:** New planning-level discoveries can be logged as `ISS-NNN`
  or `Q-NNN`, and generated canonical context packages are ignored and safe to
  regenerate.
- **Validation:** `pnpm format:check`, `pnpm docs:check` (7/7),
  `pnpm aps:drift --json` (no APSCAN regressions); verified `.aps/context/`
  fixture is ignored by creating + checking + removing a TEST file.
- **Files:** `plans/issues.md`, `.gitignore`
- **Dependencies:** APSCAN-001
- **Closeout:** Empty-but-shaped `plans/issues.md` tracker landed with
  `ISS-NNN` / `Q-NNN` IDs, status vocabulary, promotion path, and explicit
  out-of-scope list. `.gitignore` ignores `.aps/` so canonical context
  packages stay regenerable cache output; the durable surface remains
  `plans/issues.md`.
- **Confidence:** medium

### APSCAN-008: Decide canonical CLI adoption boundary

- **Status:** Merged 2026-05-25 via PR [#1947](https://github.com/eddacraft/anvil-001/pull/1947)
- **Intent:** Choose whether Anvil consumes the canonical `aps` CLI directly or
  keeps its local `@eddacraft/anvil-aps` package as a compatibility layer.
- **Expected Outcome:** A short design note records the authority split between
  canonical CLI commands and Anvil-specific drift/release checks.
- **Validation:** Design note reviewed against `anvil-plan-spec/docs/usage.md` and
  current `packages/aps/**` consumers; `pnpm docs:check` (7/7 pass);
  `pnpm aps:drift --json` (no APSCAN regressions).
- **Files:** `plans/specs/2026-05-25-aps-cli-adoption-boundary.md`,
  `packages/aps/**`, `scripts/aps/**`
- **Spec:** `plans/specs/2026-05-25-aps-cli-adoption-boundary.md`
- **Closeout:** Hybrid adoption boundary recorded — canonical `aps` CLI is the
  source of truth for portable APS semantics; `@eddacraft/anvil-aps` stays as
  the local compatibility + extension layer; `scripts/aps/*` stays as the
  Anvil-only enforcement layer for drift, release evidence, progress counters,
  active-lint scope, and Anvil status extensions. No code changes — the
  existing surfaces already match the recorded split.
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

- **Status:** Merged via PR #1906 (`4c6e1e2a`)
- **Intent:** Provide a read-only terminal dashboard for active APS work so
  operators and agents can see in-progress modules, ready work, blocked work, and
  local reconciliation hints without manually reading the APS index and every
  module file.
- **Expected Outcome:** `anvil plan dashboard` builds a local-only
  `PlanStatusSnapshot`, renders active APS state in a Ratatui dashboard, flags
  APS-only consistency issues, and keeps an empty v1 enrichment seam for future
  GitHub/CI annotations.
- **Validation:** `cargo test -p eddacraft-anvil plan_dashboard --bin anvil`,
  `cargo test -p eddacraft-anvil-tui plan_dashboard --lib`, `cargo test -p
  eddacraft-anvil tui_snapshot --bin anvil`, `cargo fmt --check`, `pnpm
  format:check`, `pnpm docs:check`, `pnpm aps:drift --json`, and `cargo clippy
  --workspace --all-targets -- -D warnings`
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
- **Closeout:** PR #1906 merged 2026-05-24 at `4c6e1e2a`.
- **Confidence:** medium
