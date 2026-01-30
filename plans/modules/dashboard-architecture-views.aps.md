# Dashboard Architecture Views

| ID | Owner | Status |
|----|-------|--------|
| DASHARCH | @eddacraft | Draft |

## Purpose

Implement the dashboard pages for architecture visualization, drift tracking,
and suppression management. These pages differentiate the web dashboard from the
CLI — graphical dependency graphs, side-by-side snapshot comparisons, and
suppression lifecycle views are not feasible in a terminal. Together they answer:
"Is the architecture healthy, is it getting better or worse, and where are we
making exceptions?"

## In Scope

- Architecture page: overview with layer diagram, boundary violation explorer,
  interactive dependency graph, layer statistics, entry point listing
- Drift page: snapshot timeline, snapshot detail, snapshot comparison (diff view)
- Suppressions page: active/expiring/expired views, suppression trend analysis
- Graphical visualizations that exceed CLI capabilities

## Out of Scope

- Architecture definition editing (managed via `.anvil/architecture.yml` in code)
- OPA policy authoring (see opa-architecture-integration module)
- Suppression creation or renewal through the UI — deferred to write API phase
- Overview page metrics (see DASHCORE)

## Interfaces

**Depends on:**

- `dashboard-foundation` — App shell, routing, component catalog, data hooks
- `architecture-safety` — Boundary rules, layer definitions, violation schemas
- `opa-architecture-integration` — Architecture YAML schema, template definitions
- `drift-reporting` — Snapshot schema, comparison logic, trend calculation
- `suppressions` — Suppression record format, scope types, expiry rules

**Exposes:**

- Architecture pages at `/architecture`, `/architecture/violations`,
  `/architecture/graph`, `/architecture/layers`, `/architecture/entry-points`
- Drift pages at `/drift`, `/drift/:id`, `/drift/compare`
- Suppression pages at `/suppressions`, `/suppressions/expiring`,
  `/suppressions/expired`, `/suppressions/trends`
- Anvil-specific components: `DriftIndicator`, `FileViolationMap`,
  `SuppressionRequest`, `LayerDiagram`, `DependencyGraph`

## Decisions

**D-DASHARCH-001:** Graph rendering library

- **Options:** (a) D3.js with custom React wrapper, (b) React Flow,
  (c) Cytoscape.js, (d) Dagre + SVG
- **Recommendation:** React Flow — purpose-built for node-edge graphs in React,
  supports zoom/pan/selection, good layout algorithms via dagre integration
- **Status:** Open

## Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| Dependency graph overwhelms on large codebases | high | Default to module-level (not file-level) granularity; filter by layer |
| Snapshot comparison is noisy with many small changes | medium | Group changes by category; highlight net-new only by default |
| Suppression data is scattered across source files | medium | API aggregates from suppression parser; cache results |

## Ready Checklist

Change status to **Ready** when:

- [ ] Purpose and scope are clear
- [ ] Dependencies identified
- [ ] At least one task defined
- [ ] Graph library decision (D-DASHARCH-001) resolved
- [ ] DASH foundation tasks are Ready or In Progress

## Tasks

### DASHARCH-001: Architecture overview with layer diagram

- **Intent:** Visualise the project's architecture definition at a glance
- **Expected Outcome:** Page showing: visual layer diagram with allowed
  dependency arrows between layers, summary metric cards (total modules, total
  violations, new violations, circular dependencies, orphan modules), and active
  architecture template info (starter/layered/hexagonal/clean/ddd/monorepo).
  The diagram is rendered from the architecture YAML definition.
- **Scope:** `apps/anvil-ui/src/pages/architecture/`
- **Non-scope:** Editing architecture definitions
- **Dependencies:** DASH-004, DASH-006
- **Validation:** Layer diagram renders from architecture YAML; metrics reflect current state
- **Confidence:** medium

### DASHARCH-002: Boundary violation explorer

- **Intent:** Browse and understand all architecture boundary violations
- **Expected Outcome:** Sortable, filterable table of all boundary violations
  showing: source file, target file, source layer, target layer, violated rule,
  severity, new-since-baseline flag, import type (import/require/dynamic), line
  number. Filters: severity, layer pair, new-only, file path. Clicking a
  violation highlights it in the dependency graph (if graph view is loaded).
- **Scope:** `apps/anvil-ui/src/pages/architecture/`
- **Non-scope:** Violation resolution or suppression from this view
- **Dependencies:** DASH-004, DASH-006, DASH-008
- **Validation:** Table shows all violations; filters work; layer pair filter narrows to specific boundaries
- **Confidence:** high

### DASHARCH-003: Interactive dependency graph

- **Intent:** Graphically visualise module dependencies and violations
- **Expected Outcome:** Interactive node-edge graph where nodes represent
  modules/files (colored by layer), edges represent dependencies (green =
  allowed, red = violation, orange = warning). Supports: click a node to see
  its details (layer, dependency count, dependents, orphan status), filter by
  layer, zoom to specific boundary pairs, highlight circular dependencies,
  pan/zoom with mouse and keyboard. Module-level granularity by default with
  option to expand to file-level within a module.
- **Scope:** `apps/anvil-ui/src/pages/architecture/`
- **Non-scope:** Graph editing; this is read-only visualization
- **Dependencies:** DASHARCH-001, D-DASHARCH-001 (graph library decision)
- **Validation:** Graph renders modules with correct layer colors; violations are visually distinct; interaction works
- **Confidence:** low

### DASHARCH-004: Drift timeline and snapshot list

- **Intent:** Show how the codebase has changed over time via drift snapshots
- **Expected Outcome:** Page with: chronological list of all drift snapshots
  (name, timestamp, key metrics: boundary violations, antipattern count,
  suppression count, files analysed), line charts of each metric over time
  across all snapshots, overall trend badge (improving/stable/degrading). Each
  snapshot row is clickable for detail view. Two snapshots can be selected for
  comparison.
- **Scope:** `apps/anvil-ui/src/pages/drift/`
- **Non-scope:** Snapshot creation
- **Dependencies:** DASH-004, DASH-006
- **Validation:** Snapshot list renders; trend charts show historical data; trend badge reflects trajectory
- **Confidence:** high

### DASHARCH-005: Snapshot detail view

- **Intent:** Deep dive into a single drift snapshot's data
- **Expected Outcome:** Full snapshot view with: metrics panel (boundary
  violations, antipattern count, suppression count, expired suppressions, files
  analysed), anti-pattern breakdown bar chart by pattern ID, hotspot ranking
  (files with highest violation count), full tables for violations, anti-pattern
  instances, and active suppressions in this snapshot.
- **Scope:** `apps/anvil-ui/src/pages/drift/`
- **Non-scope:** Comparison view
- **Dependencies:** DASHARCH-004
- **Validation:** Snapshot detail renders all metrics and tables; hotspots are correctly ranked
- **Confidence:** high

### DASHARCH-006: Snapshot comparison view

- **Intent:** Compare two drift snapshots side-by-side to understand changes
- **Expected Outcome:** Side-by-side or diff view between two selected snapshots
  showing: metrics diff (before → after with delta and direction arrow), net
  change summary (+/- violations, antipatterns, suppressions), new violations
  (present in "after" but not "before"), resolved violations (present in
  "before" but not "after"), per-pattern anti-pattern deltas, and time duration
  between snapshots. Two snapshots are selected from the timeline or via URL
  params.
- **Scope:** `apps/anvil-ui/src/pages/drift/`
- **Non-scope:** Three-way or range comparisons
- **Dependencies:** DASHARCH-004, DASH-008
- **Validation:** Comparison renders with correct deltas; new/resolved items are correctly identified
- **Confidence:** medium

### DASHARCH-007: Suppression list with lifecycle views

- **Intent:** Make all suppressions visible with their lifecycle status
- **Expected Outcome:** Three filterable views (tabs): Active (all current
  suppressions), Expiring Soon (within configurable window: 7/14/30 days),
  Expired (past expiry date). Table columns: pattern ID, file, line, reason,
  scope (line/statement/file), author, created date, expiry date, status badge
  (active/expiring soon/expired). Filters: pattern, scope, author, expiry
  status, file path.
- **Scope:** `apps/anvil-ui/src/pages/suppressions/`
- **Non-scope:** Suppression creation, renewal, or deletion
- **Dependencies:** DASH-004, DASH-006, DASH-008
- **Validation:** All three views render; filtering works; expiry calculations are correct
- **Confidence:** high

### DASHARCH-008: Suppression trend analysis

- **Intent:** Understand suppression patterns and technical debt accumulation
- **Expected Outcome:** Charts page showing: total suppressions over time (are
  we accumulating technical debt?), suppressions by pattern (which patterns are
  most suppressed?), suppressions by author (where is friction?), average
  suppression lifetime, expiry compliance rate (time-boxed vs. permanent).
  These charts help teams make informed decisions about suppression policies.
- **Scope:** `apps/anvil-ui/src/pages/suppressions/`
- **Non-scope:** Suppression policy configuration
- **Dependencies:** DASH-004, DASH-006
- **Validation:** Charts render with suppression historical data; trends are visible
- **Confidence:** medium

## Execution

Steps: [../execution/DASHARCH.steps.md](../execution/DASHARCH.steps.md)
