# Dashboard Core Views

| ID       | Owner      | Status | Progress |
| -------- | ---------- | ------ | -------- |
| DASHCORE | @eddacraft | In Progress | 0/9      |

**Last reviewed:** 2026-07-18 — Wave 2 started with DASHCORE-001 after the
archived schema-source callouts were reconciled against the shipped Wave 1
Rust API boundary. DASHCORE-001 now owns honest current-state cards;
DASHCORE-002 owns the retained-history read model and all trend/sparkline work.

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
- an authoritative Rust retained-history read surface — required by
  DASHCORE-002 before trends, drift history, or suppression history can be
  claimed; no such dashboard capability exists yet

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

- **Status:** In Progress
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

- **Status:** Proposed — requires an authoritative Rust retained-history source;
  the latest-run `GateSnapshot` is intentionally insufficient.
- **Intent:** Add the authoritative historical health read model, then visualise
  codebase-health trajectory without fixture-invented points.
- **Expected Outcome:** The local Rust dashboard API exposes dated gate pass,
  warning, drift, and suppression series from their owning evidence stores.
  Overview cards and charts use every genuine retained point available: target
  at least 30 days, include a longer retained range when present, show a shorter
  actual range honestly, label the covered dates, and never pad missing days.
  Warning-count trend and gate-pass-rate views provide daily/weekly controls.
  Uses DASH-004 chart primitives.
- **Files:**
  - `crates/anvil-dashboard-server/src/api.rs`
  - `crates/anvil-dashboard-server/src/capabilities/`
  - `crates/anvil-dashboard-server/src/openapi.rs`
  - `apps/dashboard/src/api/generated/openapi.json`
  - `apps/dashboard/src/api/generated/openapi.d.ts`
  - `apps/dashboard/src/api/client.ts`
  - `apps/dashboard/src/modules/core/overview/trend-charts.tsx`
- **Dependencies:** DASH-004, DASH-006; retained-history source design and
  ownership validation
- **Validation:** Rust contract tests pin source attribution and date ordering;
  TypeScript tests prove actual-range labels, no padded samples, longer-than-30
  retention, shorter available history, missing periods, and time aggregation
- **Confidence:** low until the retained-history authority is selected

### DASHCORE-003: Overview — activity feed

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

- **Intent:** Browse all gate runs with sorting and filtering
- **Expected Outcome:** DataTable of gate runs: timestamp, status, score, checks
  (passed/total), trigger, duration, file count. Filters via useFilterParams.
  Click navigates to detail.
- **Files:**
  - `apps/dashboard/src/routes/gates.index.tsx`
  - `apps/dashboard/src/modules/core/gates/gate-history-table.tsx`
- **Dependencies:** DASH-003, DASH-006, DASH-008
- **Validation:** Table renders gate history; filters narrow results; row click
  navigates
- **Confidence:** high

### DASHCORE-005: Gate detail with check tree

- **Intent:** Deep dive into a single gate run's results
- **Expected Outcome:** Header (status, score, timestamp, trigger), expandable
  check tree (name, status, score, message, duration → detailed output on
  expand), evidence panel, provenance footer. Keyboard nav (j/k/Enter/n/N).
- **Files:**
  - `apps/dashboard/src/routes/gates.$id.tsx`
  - `apps/dashboard/src/modules/core/gates/check-tree.tsx`
  - `apps/dashboard/src/modules/core/gates/gate-detail-header.tsx`
  - `apps/dashboard/src/modules/core/gates/evidence-panel.tsx`
- **Dependencies:** DASHCORE-004, DASH-003
- **Validation:** Detail view renders full gate data; check tree expands;
  keyboard nav works
- **Confidence:** medium

### DASHCORE-006: Warning list with grouping/filtering

- **Intent:** Browse and investigate all active warnings
- **Expected Outcome:** DataTable: ID, severity badge, category, title, file,
  line, new-since-baseline, suppression status. Filters: severity, category,
  file glob, new-only, suppressed. Group-by: file, category, severity, pattern.
  Click opens detail panel.
- **Files:**
  - `apps/dashboard/src/routes/warnings.index.tsx`
  - `apps/dashboard/src/modules/core/warnings/warning-table.tsx`
- **Dependencies:** DASH-003, DASH-006, DASH-008
- **Validation:** Table renders warnings; filters narrow results; group-by
  reorganises data
- **Confidence:** high

### DASHCORE-007: Warning detail panel

- **Intent:** Understand a specific warning in full context
- **Expected Outcome:** shadcn/ui Sheet with full message, explanation, fix
  suggestion, code context (highlighted violation), suppression status, drift
  info. Opens on row click, closes on Escape.
- **Files:**
  - `apps/dashboard/src/modules/core/warnings/warning-detail-panel.tsx`
- **Dependencies:** DASHCORE-006
- **Validation:** Clicking a warning opens panel with full context; code
  rendering is readable
- **Confidence:** medium

### DASHCORE-008: Warning breakdown visualisations

- **Intent:** Understand warning distribution and identify hotspots
- **Expected Outcome:** Bar chart by pattern ID, hotspot file ranking, donut for
  severity, donut for category.
- **Files:**
  - `apps/dashboard/src/routes/warnings.breakdown.tsx`
  - `apps/dashboard/src/modules/core/warnings/warning-charts.tsx`
- **Dependencies:** DASH-004, DASH-006
- **Validation:** Charts render and reflect current warning data; hotspots are
  identifiable
- **Confidence:** medium

### DASHCORE-009: Anti-pattern registry reference

- **Intent:** Document all defined anti-patterns in an accessible reference
- **Expected Outcome:** Table of all patterns (AP-001..007): ID, name, category,
  severity, enabled, instance count, sparkline. Click expands inline docs.
- **Files:**
  - `apps/dashboard/src/routes/warnings.patterns.tsx`
  - `apps/dashboard/src/modules/core/warnings/pattern-registry.tsx`
- **Dependencies:** DASH-003, DASH-006
- **Validation:** All defined patterns appear; clicking opens documentation;
  instance counts are accurate
- **Confidence:** high
