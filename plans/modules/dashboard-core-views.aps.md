# Dashboard Core Views

| ID | Owner | Status |
|----|-------|--------|
| DASHCORE | @eddacraft | Draft |

## Purpose

Implement the three most-used dashboard pages — Overview (home), Gates, and
Warnings — plus their sub-views. These pages cover the daily developer workflow:
see project health, drill into gate results, and investigate warnings. This is
the minimum viable dashboard.

## In Scope

- Overview page: metric cards, trend charts, recent activity feed, quick actions
- Gates page: run history list, gate detail view, gate trend analysis
- Warnings page: warning list with filtering/grouping, warning detail panel,
  warning breakdown charts, anti-pattern registry reference
- Empty states and loading skeletons for all views
- Data export per table/chart (JSON, CSV, Markdown)

## Out of Scope

- Architecture visualization (see DASHARCH)
- Drift tracking (see DASHARCH)
- Suppression management (see DASHARCH)
- AI dashboard builder (see DASHAI)
- Audit trail, plans, configuration (see DASHOPS)

## Interfaces

**Depends on:**

- `dashboard-foundation` — App shell, routing, component catalog, data hooks,
  theme, deep linking
- `contracts` — `WarningSchema`, `EvidenceEntrySchema`, `ProvenanceRecordSchema`
- `drift-reporting` — Drift score for Overview metric card
- `antipattern-library` — Pattern definitions for registry page

**Exposes:**

- Overview page at `/`
- Gates pages at `/gates`, `/gates/:id`, `/gates/trends`
- Warnings pages at `/warnings`, `/warnings/breakdown`, `/warnings/patterns`
- Anvil-specific components: `GateResultCard`, `WarningList`, `GateCheckTree`

## Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| Overview page feels empty with sparse data | medium | Show meaningful empty states; provide sample/demo data mode |
| Warning list is too slow with large codebases | medium | Paginate server-side; virtual scrolling for long lists |
| Gate detail check tree is complex to navigate | low | Keyboard navigation (j/k/n/N) matching CLI Gate Explorer |

## Ready Checklist

Change status to **Ready** when:

- [ ] Purpose and scope are clear
- [ ] Dependencies identified
- [ ] At least one task defined
- [ ] DASH foundation tasks are Ready or In Progress

## Tasks

### DASHCORE-001: Overview page — metric cards row

- **Intent:** Show at-a-glance project health via key metrics
- **Expected Outcome:** Top row of metric cards displaying: gate pass rate (last
  30 days) with sparkline, active warning count by severity, architecture drift
  score with trend arrow, pending suppression count, active plan count by
  status, and last gate run timestamp with pass/fail badge
- **Scope:** `apps/anvil-ui/src/pages/overview/`
- **Non-scope:** Trend charts, activity feed
- **Dependencies:** DASH-004, DASH-006
- **Validation:** Cards render with real data from API; sparklines show 30-day trend
- **Confidence:** high

### DASHCORE-002: Overview page — trend charts

- **Intent:** Visualise codebase health trajectory over time
- **Expected Outcome:** Mid-page section with three charts: warning trend (line
  chart, 30/60/90 day toggles, grouped by category), gate pass rate over time
  (area chart, daily/weekly granularity), and drift trajectory (line chart of
  key drift metrics across snapshots). All charts support hover tooltips and
  time range selection.
- **Scope:** `apps/anvil-ui/src/pages/overview/`
- **Non-scope:** Custom chart configurations
- **Dependencies:** DASH-004, DASH-006
- **Validation:** Charts render with historical data; time range toggle works
- **Confidence:** medium

### DASHCORE-003: Overview page — activity feed and quick actions

- **Intent:** Show recent events and provide navigation shortcuts
- **Expected Outcome:** Chronological activity feed showing recent gate runs,
  new warnings, suppression events, plan lifecycle events, and architecture
  changes. Each entry has timestamp, event type badge, summary text, and actor.
  Quick action buttons: "View latest warnings", "Compare drift". Feed is
  paginated and auto-refreshes.
- **Scope:** `apps/anvil-ui/src/pages/overview/`
- **Non-scope:** Real-time push updates
- **Dependencies:** DASH-004, DASH-006
- **Validation:** Activity feed shows latest events; clicking entries navigates to detail views
- **Confidence:** high

### DASHCORE-004: Gate history list with filtering

- **Intent:** Browse all gate runs with sorting and filtering
- **Expected Outcome:** Sortable table of all gate runs showing: timestamp,
  status badge (pass/fail), score, checks summary (passed/total), trigger type
  (manual/pre-commit/CI/watch), duration, scope, file count, actor. Filters:
  status, trigger type, date range, minimum score. Clicking a row navigates to
  gate detail.
- **Scope:** `apps/anvil-ui/src/pages/gates/`
- **Non-scope:** Gate detail view, trend analysis
- **Dependencies:** DASH-004, DASH-006, DASH-008
- **Validation:** Table renders gate history; filters narrow results; sort works; row click navigates
- **Confidence:** high

### DASHCORE-005: Gate detail view with check tree

- **Intent:** Deep dive into a single gate run's results
- **Expected Outcome:** Header with overall status, score, timestamp, duration,
  trigger, scope. Expandable check list showing each check's name, status,
  score, message, duration. Expanding a check reveals detailed output, issues
  found, and file references. Evidence panel showing raw evidence entries.
  Provenance footer with environment and git context. Keyboard navigation
  matching Gate Explorer TUI (j/k navigate, Enter expand, n/N jump to
  failures).
- **Scope:** `apps/anvil-ui/src/pages/gates/`
- **Non-scope:** Comparison between gate runs
- **Dependencies:** DASHCORE-004, DASH-004
- **Validation:** Detail view renders full gate data; check tree expands; keyboard nav works
- **Confidence:** medium

### DASHCORE-006: Gate trend analysis charts

- **Intent:** Identify patterns in gate results over time
- **Expected Outcome:** Charts page showing: pass rate by check type over time,
  average score trend, most common failure reasons (bar chart), and duration
  trends. Useful for understanding which checks are failing, whether quality is
  improving, and whether gates are getting slower.
- **Scope:** `apps/anvil-ui/src/pages/gates/`
- **Non-scope:** Custom chart building
- **Dependencies:** DASH-004, DASH-006
- **Validation:** Charts render with historical gate data; trends are visible
- **Confidence:** medium

### DASHCORE-007: Warning list with grouping and filtering

- **Intent:** Browse and investigate all active warnings
- **Expected Outcome:** Table of all warnings from latest analysis showing: ID,
  severity badge, category, title, file path, line number, new-since-baseline
  indicator, suppression status. Filters: severity, category, file path glob,
  new-only, suppressed/unsuppressed, confidence level. Group-by options: file,
  category, severity, pattern ID. Sorting on all columns.
- **Scope:** `apps/anvil-ui/src/pages/warnings/`
- **Non-scope:** Warning detail panel, breakdown charts
- **Dependencies:** DASH-004, DASH-006, DASH-008
- **Validation:** Table renders warnings; filters narrow results; group-by reorganises data
- **Confidence:** high

### DASHCORE-008: Warning detail panel with code context

- **Intent:** Understand a specific warning in full context
- **Expected Outcome:** Slide-out detail panel showing: full warning message and
  explanation, fix suggestion, code context (surrounding source lines with
  violation highlighted), suppression status (reason, author, expiry if
  suppressed), drift info (new vs. existing, instance count). Panel opens when
  clicking a warning row; closable with Escape.
- **Scope:** `apps/anvil-ui/src/pages/warnings/`
- **Non-scope:** Inline editing, auto-fix
- **Dependencies:** DASHCORE-007
- **Validation:** Clicking a warning opens panel with full context; code rendering is readable
- **Confidence:** medium

### DASHCORE-009: Warning breakdown visualisations

- **Intent:** Understand warning distribution and identify hotspots
- **Expected Outcome:** Visual summary page with: bar chart of warning counts
  per pattern ID, treemap or ranked list of warnings per file (hotspot
  identification), donut chart for severity split, donut chart for category
  split, stacked bar showing new vs. existing warnings (drift indicator).
- **Scope:** `apps/anvil-ui/src/pages/warnings/`
- **Non-scope:** Custom chart configurations
- **Dependencies:** DASH-004, DASH-006
- **Validation:** Charts render and reflect current warning data; hotspots are identifiable
- **Confidence:** medium

### DASHCORE-010: Anti-pattern registry reference page

- **Intent:** Document all defined anti-patterns in an accessible reference
- **Expected Outcome:** Table listing all anti-patterns (AP-001 through AP-007)
  with: ID, name, category, default severity, enabled status, opt-in flag,
  current instance count, sparkline trend. Clicking a pattern opens full
  documentation: explanation, detection configuration, suggestion text,
  allowlist details.
- **Scope:** `apps/anvil-ui/src/pages/warnings/`
- **Non-scope:** Pattern configuration editing
- **Dependencies:** DASH-004, DASH-006
- **Validation:** All defined patterns appear; clicking opens documentation; instance counts are accurate
- **Confidence:** high

## Execution

Steps: [../execution/DASHCORE.steps.md](../execution/DASHCORE.steps.md)
