# Dashboard Architecture Views

| ID       | Owner      | Status | Progress |
| -------- | ---------- | ------ | -------- |
| DASHARCH | @eddacraft | Ready  | 0/8      |

**Last reviewed:** 2026-07-09

## Purpose

Implement the dashboard pages for architecture visualisation, drift tracking,
and suppression management. These pages differentiate the web dashboard from the
CLI — graphical dependency graphs, side-by-side snapshot comparisons, and
suppression lifecycle views are not feasible in a terminal. Together they answer:
"Is the architecture healthy, is it getting better or worse, and where are we
making exceptions?"

## In Scope

- Architecture page: overview with layer diagram, boundary violation explorer,
  interactive dependency graph
- Drift page: snapshot timeline, snapshot detail, snapshot comparison (diff view)
- Suppressions page: active/expiring/expired views, suppression trend analysis
- Graphical visualisations that exceed CLI capabilities

## Out of Scope

- Architecture definition editing (managed via `.anvil/architecture.yml` in code)
- OPA policy authoring [REVIEW: original ref was `opa-architecture-integration` (archived); replacement owner TBD — possibly `opa-enhancements` or `opa-agent-orchestration`]
- Suppression creation or renewal through the UI — deferred to write API phase
- Overview page metrics (see DASHCORE)

## Interfaces

**Depends on:**

- `dashboard-foundation` — App shell, routing, component catalogue, data hooks,
  dashboard server, and OpenAPI client seam
- `architecture-safety` — Boundary rules, layer definitions, violation schemas [REVIEW: archived module — logic now in `crates/anvil-architecture/`; verify schemas/artefact format]
- `opa-architecture-integration` — Architecture YAML schema, template definitions [REVIEW: archived module — OPA hybrid covered by ADR-006; current ownership unclear]
- `drift-reporting` — Snapshot schema, comparison logic, trend calculation [REVIEW: archived module — drift artefacts now produced by Rust kernel; verify schema source]
- `suppressions` — Suppression record format, scope types, expiry rules [REVIEW: archived module — suppression parser is now Rust per ADR-029; verify schema source]

**Exposes:**

- Architecture pages at `/architecture`, `/architecture/violations`,
  `/architecture/graph`
- Drift pages at `/drift`, `/drift/:name`, `/drift/compare`
- Suppression pages at `/suppressions`, `/suppressions/trends`
- Domain components: `LayerDiagram`, `DependencyGraph`, `DriftIndicator`,
  `SuppressionLifecycleTable`

## Decisions

**D-DASHARCH-001:** Graph rendering library

- **Options:** (a) D3.js with custom React wrapper, (b) React Flow,
  (c) Cytoscape.js, (d) Dagre + SVG
- **Resolution:** Option (b) — React Flow. Purpose-built for node-edge graphs in
  React, supports zoom/pan/selection, good layout algorithms via dagre
  integration.
- **Status:** Resolved

## Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| Dependency graph overwhelms on large codebases | high | Default to module-level (not file-level) granularity; filter by layer |
| Snapshot comparison is noisy with many small changes | medium | Group changes by category; highlight net-new only by default |
| Suppression data is scattered across source files | medium | API aggregates from suppression parser; cache results |

## Ready Checklist

Change status to **Ready** when:

- [x] Purpose and scope are clear
- [x] Dependencies identified
- [x] At least one task defined
- [x] Graph library decision (D-DASHARCH-001) resolved
- [x] DASH foundation tasks are Ready

## Wave

**Wave 2** — Can begin after Wave 1 (DASH) completes. Runs in parallel with
DASHCORE.

## Work Items

### DASHARCH-001: Architecture overview with layer diagram

- **Intent:** Visualise the project's architecture definition at a glance
- **Expected Outcome:** SVG layer diagram from architecture definition, metric
  cards (modules, violations, new violations, orphans), template info.
- **Files:**
  - `apps/dashboard/src/routes/architecture.index.tsx`
  - `apps/dashboard/src/modules/architecture/layer-diagram.tsx`
  - `apps/dashboard/src/modules/architecture/arch-metric-cards.tsx`
- **Dependencies:** DASH-003, DASH-006
- **Validation:** Layer diagram renders from architecture YAML; metrics reflect
  current state
- **Confidence:** medium

### DASHARCH-002: Boundary violation explorer

- **Intent:** Browse and understand all architecture boundary violations
- **Expected Outcome:** DataTable: source/target file, layers, rule, severity,
  new flag, line. Filters: severity, layer pair, new-only, file. Click shows
  code context.
- **Files:**
  - `apps/dashboard/src/routes/architecture.violations.tsx`
  - `apps/dashboard/src/modules/architecture/violation-table.tsx`
- **Dependencies:** DASH-003, DASH-006, DASH-008
- **Validation:** Table shows all violations; filters work; layer pair filter
  narrows to specific boundaries
- **Confidence:** high

### DASHARCH-003: Interactive dependency graph

- **Intent:** Graphically visualise module dependencies and violations
- **Expected Outcome:** React Flow graph: nodes = modules (coloured by layer),
  edges = dependencies (green=allowed, red=violation). Click node for details,
  filter by layer, highlight cycles, pan/zoom. Lazy-loaded via a React lazy
  route/component boundary.
- **Files:**
  - `apps/dashboard/src/routes/architecture.graph.tsx`
  - `apps/dashboard/src/modules/architecture/dependency-graph.tsx`
- **Dependencies:** DASHARCH-001
- **Validation:** Graph renders modules with correct layer colours; violations
  are visually distinct; interaction works
- **Confidence:** low

### DASHARCH-004: Drift timeline and snapshot list

- **Intent:** Show how the codebase has changed over time via drift snapshots
- **Expected Outcome:** Chronological snapshot list (name, timestamp, metrics),
  trend line charts per metric, trend badge (improving/stable/degrading).
  Checkbox selection for comparison.
- **Files:**
  - `apps/dashboard/src/routes/drift.index.tsx`
  - `apps/dashboard/src/modules/drift/snapshot-list.tsx`
  - `apps/dashboard/src/modules/drift/drift-trend-charts.tsx`
- **Dependencies:** DASH-003, DASH-004, DASH-006
- **Validation:** Snapshot list renders; trend charts show historical data; trend
  badge reflects trajectory
- **Confidence:** high

### DASHARCH-005: Snapshot detail view

- **Intent:** Deep dive into a single drift snapshot's data
- **Expected Outcome:** Metrics panel, anti-pattern breakdown bar chart, hotspot
  ranking, full tables for violations/antipatterns/suppressions.
- **Files:**
  - `apps/dashboard/src/routes/drift.$name.tsx`
  - `apps/dashboard/src/modules/drift/snapshot-detail.tsx`
- **Dependencies:** DASHARCH-004
- **Validation:** Snapshot detail renders all metrics and tables; hotspots are
  correctly ranked
- **Confidence:** high

### DASHARCH-006: Snapshot comparison view

- **Intent:** Compare two drift snapshots side-by-side to understand changes
- **Expected Outcome:** Side-by-side metrics diff (before→after with delta
  arrows), new/resolved violations, per-pattern deltas. Snapshot names via query
  params.
- **Files:**
  - `apps/dashboard/src/routes/drift.compare.tsx`
  - `apps/dashboard/src/modules/drift/snapshot-comparison.tsx`
- **Dependencies:** DASHARCH-004, DASH-008
- **Validation:** Comparison renders with correct deltas; new/resolved items are
  correctly identified
- **Confidence:** medium

### DASHARCH-007: Suppression list with lifecycle views

- **Intent:** Make all suppressions visible with their lifecycle status
- **Expected Outcome:** Tabs (Active/Expiring Soon/Expired). Table: pattern,
  file, line, reason, scope, author, dates, status. Filters: pattern, scope,
  author, expiry window, file.
- **Files:**
  - `apps/dashboard/src/routes/suppressions.index.tsx`
  - `apps/dashboard/src/modules/suppressions/suppression-table.tsx`
- **Dependencies:** DASH-003, DASH-006, DASH-008
- **Validation:** All three views render; filtering works; expiry calculations
  are correct
- **Confidence:** high

### DASHARCH-008: Suppression trend analysis

- **Intent:** Understand suppression patterns and technical debt accumulation
- **Expected Outcome:** Charts: total suppressions over time, by pattern, by
  scope, average lifetime, expiry compliance rate.
- **Files:**
  - `apps/dashboard/src/routes/suppressions.trends.tsx`
  - `apps/dashboard/src/modules/suppressions/suppression-charts.tsx`
- **Dependencies:** DASH-004, DASH-006
- **Validation:** Charts render with suppression historical data; trends are
  visible
- **Confidence:** medium
