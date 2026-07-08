# Dashboard Core Views

| ID       | Owner      | Status | Progress |
| -------- | ---------- | ------ | -------- |
| DASHCORE | @eddacraft | Ready  | 0/9      |

**Last reviewed:** 2026-07-09

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
- `contracts` — `WarningSchema`, `EvidenceEntrySchema`, `ProvenanceRecordSchema` (see `schema-contracts` module)
- `drift-reporting` — Drift score for Overview metric card [REVIEW: archived module — drift artefacts now produced by Rust kernel; verify schema source]
- `antipattern-library` — Pattern definitions for registry page [REVIEW: archived module — pattern definitions now live in `crates/anvil-checks/`; verify registry source]

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

- **Intent:** Show at-a-glance project health via key metrics
- **Expected Outcome:** MetricCards for gate pass rate (sparkline), active
  warnings by severity, drift score with trend arrow, suppression count, last
  gate run with badge. Each links to detail page.
- **Files:**
  - `apps/dashboard/src/routes/index.tsx`
  - `apps/dashboard/src/modules/core/overview/metric-cards.tsx`
- **Dependencies:** DASH-003, DASH-006
- **Validation:** Cards render with real data from API; sparklines show 30-day
  trend; clicking navigates to detail
- **Confidence:** high

### DASHCORE-002: Overview — trend charts

- **Intent:** Visualise codebase health trajectory over time
- **Expected Outcome:** Warning count trend (line, 30/60/90 day toggle) and gate
  pass rate (area, daily/weekly). Uses DASH-004 charts.
- **Files:**
  - `apps/dashboard/src/modules/core/overview/trend-charts.tsx`
- **Dependencies:** DASH-004, DASH-006
- **Validation:** Charts render with historical data; time range toggle works
- **Confidence:** medium

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
