# Dashboard Core Views

| ID       | Owner      | Status | Progress |
| -------- | ---------- | ------ | -------- |
| DASHCORE | @eddacraft | In Progress | 8/9      |

**Last reviewed:** 2026-07-27 — DASHCORE-003..009 reconciled to Merged; their
Wave 2 product routes landed on `main` via PR #3379 (2026-07-24) but the item
statuses were never flipped, so the module read 1/9 while eight items were
terminal. DASHCORE-001 Merged via PR #3363. Only **DASHCORE-002** (retained
history and trends) remains: Ready since 2026-07-26 with an approved design
spec. The module stays In Progress until it lands.

## Purpose

Implement the three most-used dashboard pages — Overview (home), Gates, and
Warnings — plus their sub-views. These pages cover the daily developer workflow:
see project health, drill into gate results, and investigate warnings. This is
the minimum viable dashboard.

## In Scope

- Overview page: metric cards, trend charts, recent activity feed
- Gates page: run history list, gate detail view with check tree
- Warnings page: warning list with filtering/grouping, warning detail panel,
  warning breakdown charts, anti-pattern registry reference
- Empty states and loading skeletons for all views

## Out of Scope

- Architecture visualisation (see DASHARCH)
- Drift tracking (see DASHARCH)
- Suppression management (see DASHARCH)
- AI dashboard builder (see DASHAI)
- Audit trail, plans, configuration (see DASHOPS)

## Interfaces

**Depends on:**

- `dashboard-foundation` — App shell, routing, component catalogue, data hooks,
  theme, deep linking, dashboard server, and OpenAPI client seam
- `crates/anvil-dashboard-server` — authoritative dashboard capability adapter;
  Rust response types flow through OpenAPI into the generated TypeScript client
- `crates/anvil-kernel-types/src/gate_snapshot.rs` — canonical persisted latest
  gate summary; deliberately excludes retained history and diagnostic detail
- `patterns/compiled/registry.json`, loaded by
  `crates/anvil-checks/src/antipattern/registry_loader.rs` — canonical
  anti-pattern catalogue source for DASHCORE-009
- an authoritative Rust retained-history read surface — DASHCORE-002 design
  approved (`plans/specs/2026-07-26-dashcore-002-retained-history.md`); v1 is
  gate-derived series via `.anvil/gate-history.ndjson` +
  `GET /api/v1/protection/history` (drift/suppression series remain gaps)

**Exposes:**

- Overview page at dashboard root
- Gates pages at `/gates`, `/gates/:id`
- Warnings pages at `/warnings`, `/warnings/breakdown`, `/warnings/patterns`
- Domain components: `GateResultCard`, `WarningList`, `GateCheckTree`

## Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| Overview page feels empty with sparse data | medium | Show meaningful empty states; provide sample/demo data mode |
| Warning list is too slow with large codebases | medium | Paginate server-side; virtual scrolling for long lists |
| Gate detail check tree is complex to navigate | low | Keyboard navigation (j/k/n/N) matching CLI Gate Explorer |

## Ready Checklist

Change status to **Ready** when:

- [x] Purpose and scope are clear
- [x] Dependencies identified
- [x] At least one task defined
- [x] DASH foundation tasks are Ready

## Wave

**Wave 2** — Can begin after Wave 1 (DASH) completes. Runs in parallel with
DASHARCH.

## Work Items

### DASHCORE-001: Overview — metric cards row

- **Status:** Merged 2026-07-19 via PR #3363
- **Pull Request:** [#3363](https://github.com/eddacraft/anvil-001/pull/3363)
- **Intent:** Show the current project-health facts the shipped local dashboard
  API can prove, without manufacturing retained history.
- **Expected Outcome:** A responsive MetricCard row derived only from the typed
  `ProtectionOverview` resource: save-time protection state, latest gate
  score/result, active-warning count or explicit partial/unavailable state,
  workspace assurance coverage/state, and evidence freshness. Cards remain
  readable without colour or hover. No sparkline, drift history, suppression
  total, or 30-day claim renders until DASHCORE-002 supplies genuine retained
  points through the Rust API.
- **Files:**
  - `apps/dashboard/src/components/primitives/metric-card.tsx`
  - `apps/dashboard/src/modules/core/overview/current-health-cards.tsx`
  - `apps/dashboard/src/modules/core/overview/current-health-cards.test.tsx`
  - `apps/dashboard/src/modules/protection/protection-overview.tsx`
  - `apps/dashboard/src/modules/protection/protection-overview.test.tsx`
  - `apps/dashboard/src/styles.css`
- **Dependencies:** DASH-003, DASH-006, DASH-010
- **Validation:** Typed component tests prove complete, partial, and unavailable
  resources never collapse into false zeroes; dashboard test, lint, typecheck,
  and build targets pass
- **Evidence:** 43 dashboard tests, dashboard lint/typecheck/build,
  `validate:changed`, APS/docs/format checks, and desktop/mobile visual QA pass;
  Council session `council-aca76a2d` converged with one minor finding fixed and
  no open findings
- **Confidence:** high

### DASHCORE-002: Overview — retained history and trend charts

- **Status:** In Progress
- **Pull Request:** [#3436](https://github.com/eddacraft/anvil-001/pull/3436)
- **Design:** [2026-07-26-dashcore-002-retained-history.md](../specs/2026-07-26-dashcore-002-retained-history.md)
  (approved 2026-07-26)
- **Intent:** Add the authoritative historical health read model from a
  gate-writer NDJSON store, then visualise gate-score and warning-count
  trajectory without fixture-invented points.
- **Expected Outcome:** Gate persist best-effort appends
  `.anvil/gate-history.ndjson` (90-day + ~500 line retention). Dashboard
  `GET /api/v1/protection/history` returns ordered raw points + actual range +
  honest `data_state`/gaps (drift/suppression unavailable in v1). Overview
  charts/sparklines consume that resource; browser aggregates daily/weekly in
  UTC; Wave 2 actual-range rules (no padding). Gate write never fails on
  history I/O errors.
- **Files:**
  - `crates/anvil-cli/src/commands/gate.rs`
  - `crates/anvil-dashboard-server/src/api.rs`
  - `crates/anvil-dashboard-server/src/capabilities/history.rs`
  - `crates/anvil-dashboard-server/src/capabilities/mod.rs`
  - `crates/anvil-dashboard-server/src/lib.rs`
  - `crates/anvil-dashboard-server/src/openapi.rs`
  - `crates/anvil-dashboard-server/src/server.rs`
  - `crates/anvil-dashboard-server/tests/openapi_snapshot.rs`
  - `crates/anvil-dashboard-server/tests/protection_history.rs`
  - `crates/anvil-dashboard-server/tests/server_smoke.rs`
  - `apps/dashboard/src/api/generated/openapi.json`
  - `apps/dashboard/src/api/generated/openapi.d.ts`
  - `apps/dashboard/src/api/client.test.ts`
  - `apps/dashboard/src/api/client.ts`
  - `apps/dashboard/src/api/query-client.tsx`
  - `apps/dashboard/src/api/query-keys.ts`
  - `apps/dashboard/src/api/query-layer.test.tsx`
  - `apps/dashboard/src/hooks/use-protection-history.ts`
  - `apps/dashboard/src/modules/core/overview/history-aggregation.test.ts`
  - `apps/dashboard/src/modules/core/overview/history-aggregation.ts`
  - `apps/dashboard/src/modules/core/overview/trend-charts.test.tsx`
  - `apps/dashboard/src/modules/core/overview/trend-charts.tsx`
  - `apps/dashboard/src/modules/protection/protection-overview.test.tsx`
  - `apps/dashboard/src/modules/protection/protection-overview.tsx`
  - `apps/dashboard/src/styles.css`
- **Dependencies:** DASH foundation Merged; design approved; DASHCORE-001 Merged
- **Validation:**
  - `cargo test -p eddacraft-anvil-dashboard-server`
  - `cargo test -p eddacraft-anvil -- gate` (or package filter covering
    `commands::gate` history append unit tests)
  - `pnpm nx run dashboard:test`
  - `pnpm nx run dashboard:typecheck`
  - `pnpm nx run dashboard:lint`
  - `pnpm nx run dashboard:build`
  - `pnpm --filter @eddacraft/anvil-dashboard generate:api` when OpenAPI changes
    (or repo equivalent), then `check:api`
- **Confidence:** high (design approved; write site known:
  `persist_gate_snapshot` in `gate.rs`)
- **Evidence:** CLI gate/history tests, dashboard-server tests, dashboard
  test/typecheck/lint/build and generated-API parity, Rust fmt/clippy, APS/docs
  checks, Council `council-62254fb8` (12 findings fixed; zero open), and fresh
  independent verify-loop `pass-with-advisories`; root evidence gates are green
  on the feature worktree.

### DASHCORE-003: Overview — activity feed

- **Status:** Merged 2026-07-24 via PR #3379 — activity feed over provenance events with detail navigation

- **Intent:** Show recent events and provide navigation shortcuts
- **Expected Outcome:** Last 20 events from provenance: gate runs, new warnings,
  suppressions. Timestamp, type badge, summary, actor. Click navigates to detail.
- **Files:**
  - `apps/dashboard/src/modules/core/overview/activity-feed.tsx`
- **Dependencies:** DASH-003, DASH-006
- **Validation:** Activity feed shows latest events; clicking entries navigates
  to detail views
- **Confidence:** high

### DASHCORE-004: Gate history list

- **Status:** Merged 2026-07-24 via PR #3379 — gate-run DataTable with sorting and `useFilterParams` filtering

- **Intent:** Browse all gate runs with sorting and filtering
- **Expected Outcome:** DataTable of gate runs: timestamp, status, score, checks
  (passed/total), trigger, duration, file count. Filters via useFilterParams.
  Click navigates to detail.
- **Files:**
  - `apps/dashboard/src/routes/gates.tsx`
  - `apps/dashboard/src/modules/core/gates/gate-history-table.tsx`
  - `crates/anvil-dashboard-server/src/capabilities/protection.rs`
- **Dependencies:** DASH-003, DASH-006, DASH-008
- **Validation:** Table renders latest-gate recent_runs; empty state is honest;
  row open navigates to detail
- **Confidence:** high

### DASHCORE-005: Gate detail with check tree

- **Status:** Merged 2026-07-24 via PR #3379 — gate detail header plus expandable check tree

- **Intent:** Deep dive into a single gate run's results
- **Expected Outcome:** Header (status, score, timestamp, trigger), expandable
  check tree (name, status, score, message, duration → detailed output on
  expand), evidence panel, provenance footer. Keyboard nav (j/k/Enter/n/N).
- **Files:**
  - `apps/dashboard/src/routes/gates.tsx`
  - `apps/dashboard/src/modules/core/gates/gate-detail-page.tsx`
  - `apps/dashboard/src/modules/core/gates/check-tree.tsx`
  - `apps/dashboard/src/modules/core/gates/gate-detail-header.tsx`
  - `crates/anvil-dashboard-server/src/api.rs`
- **Dependencies:** DASHCORE-004, DASH-003
- **Validation:** Detail view renders full gate data; check tree expands;
  keyboard nav works
- **Confidence:** medium

### DASHCORE-006: Warning list with grouping/filtering

- **Status:** Merged 2026-07-24 via PR #3379 — warning DataTable with grouping, severity/category filtering

- **Intent:** Browse and investigate all active warnings
- **Expected Outcome:** DataTable: ID, severity badge, category, title, file,
  line, new-since-baseline, suppression status. Filters: severity, category,
  file glob, new-only, suppressed. Group-by: file, category, severity, pattern.
  Click opens detail panel.
- **Files:**
  - `apps/dashboard/src/routes/warnings.tsx`
  - `apps/dashboard/src/modules/core/warnings/warning-table.tsx`
  - `crates/anvil-dashboard-server/src/capabilities/protection.rs`
- **Dependencies:** DASH-003, DASH-006, DASH-008
- **Validation:** Table renders warnings; filters narrow results; group-by
  reorganises data
- **Confidence:** high

### DASHCORE-007: Warning detail panel

- **Status:** Merged 2026-07-24 via PR #3379 — warning detail panel

- **Intent:** Understand a specific warning in full context
- **Expected Outcome:** shadcn/ui Sheet with full message, explanation, fix
  suggestion, code context (highlighted violation), suppression status, drift
  info. Opens on row click, closes on Escape.
- **Files:**
  - `apps/dashboard/src/modules/core/warnings/warning-detail-panel.tsx`
  - `apps/dashboard/src/routes/warnings.tsx`
- **Dependencies:** DASHCORE-006
- **Validation:** Clicking a warning opens panel with full context; code
  rendering is readable
- **Confidence:** medium

### DASHCORE-008: Warning breakdown visualisations

- **Status:** Merged 2026-07-24 via PR #3379 — warning breakdown charts

- **Intent:** Understand warning distribution and identify hotspots
- **Expected Outcome:** Bar chart by pattern ID, hotspot file ranking, donut for
  severity, donut for category.
- **Files:**
  - `apps/dashboard/src/routes/warnings.tsx`
  - `apps/dashboard/src/modules/core/warnings/warning-charts.tsx`
- **Dependencies:** DASH-004, DASH-006
- **Validation:** Charts render and reflect current warning data; hotspots are
  identifiable
- **Confidence:** medium

### DASHCORE-009: Anti-pattern registry reference

- **Status:** Merged 2026-07-24 via PR #3379 — anti-pattern registry reference

- **Intent:** Document all defined anti-patterns in an accessible reference
- **Expected Outcome:** Table of all patterns (AP-001..007): ID, name, category,
  severity, enabled, instance count, sparkline. Click expands inline docs.
- **Files:**
  - `apps/dashboard/src/routes/warnings.tsx`
  - `apps/dashboard/src/modules/core/warnings/pattern-registry.tsx`
  - `crates/anvil-dashboard-server/src/capabilities/patterns.rs`
- **Dependencies:** DASH-003, DASH-006
- **Validation:** All defined patterns appear; clicking opens documentation;
  instance counts are accurate
- **Confidence:** high
