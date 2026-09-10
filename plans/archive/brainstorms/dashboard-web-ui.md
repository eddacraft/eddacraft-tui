# Dashboard Web UI — Brainstorm

## Context

This builds on [json-render-dashboard.md](./json-render-dashboard.md), which established the rendering approach: a constrained component catalog + AI-generated JSON + progressive React rendering. This document answers the next question: **what should the web UI actually contain?**

The CLI already provides 8 TUI dashboards (status, watch, gate explorer, doctor, init wizard, template browser, tutorial, welcome). The web UI should not just replicate these — it should leverage the medium to provide views that a terminal cannot: persistent state across sessions, multi-project overviews, interactive charts, team-level aggregations, and shareable layouts.

---

## Design Principles

1. **Observe, don't obstruct** — The dashboard is read-heavy. It surfaces what already happened (gate runs, drift snapshots, violations). Write actions (approve suppressions, create plans) are secondary and always require confirmation.
2. **Defaults that work** — Every page has a sensible default view. The AI prompt builder (json-render) is an advanced feature for customisation, not the entry point.
3. **Data-first, chrome-second** — Dense information display. Avoid decorative emptiness. Engineers want to see numbers, not illustrations.
4. **CLI parity then beyond** — Every view the CLI provides should have a web equivalent, then the web goes further with capabilities the terminal cannot support.

---

## Information Architecture

```
┌─────────────────────────────────────────────────┐
│  Top Bar                                         │
│  [Logo] [Project Selector ▼] [Search] [⚙ Settings] [User ▼] │
└─────────────────────────────────────────────────┘
│                                                   │
│  Sidebar Navigation                               │
│  ─────────────────                                │
│  📊 Overview (Home)                               │
│  🚧 Gates                                         │
│  ⚠  Warnings                                     │
│  🏗 Architecture                                   │
│  📐 Drift                                         │
│  🔕 Suppressions                                   │
│  📋 Plans                                         │
│  📜 Audit Trail                                   │
│  ─────────────────                                │
│  🤖 AI Dashboard Builder                          │
│  📌 Saved Dashboards                              │
│  ─────────────────                                │
│  ⚙  Configuration                                │
│  🩺 Diagnostics                                   │
│                                                   │
└───────────────────────────────────────────────────┘
```

---

## Page Specifications

### 1. Overview (Home)

**Purpose**: At-a-glance health of the current project. The first thing a developer sees. Answers: "Is my project healthy right now?"

**Layout**: Grid of metric cards across the top, trend charts in the middle, recent activity at the bottom.

#### Top Metrics Row

| Metric | Source | Display |
|---|---|---|
| Gate pass rate (last 30 days) | Provenance records | Percentage + sparkline |
| Active warnings | Latest check run | Count by severity (error/warn/info) |
| Architecture drift score | Latest drift snapshot | Score + trend arrow (improving/stable/degrading) |
| Pending suppressions | Suppression records | Count + oldest age |
| Active plans | APS plan index | Count by status (draft/approved/applied) |
| Last gate run | Provenance records | Relative timestamp + pass/fail badge |

#### Trend Section (mid-page)

- **Warning trend chart** — Line chart, 30/60/90 day view, grouped by category (anti-pattern, boundary, architecture). Shows whether the codebase is getting cleaner or dirtier over time.
- **Gate pass rate over time** — Area chart showing pass/fail ratio per day/week.
- **Drift trajectory** — Line chart of drift snapshot metrics over time (boundary violations, antipattern count, suppression count).

#### Recent Activity Feed (bottom)

Chronological list of recent events:
- Gate runs (passed/failed, which checks, score)
- New warnings introduced (with file + line links)
- Suppressions created/expired
- Plans created/approved/applied
- Architecture definition changes

Each entry shows: timestamp, event type badge, summary text, actor (user/CI/watch).

#### Quick Actions

- "Run gate" → triggers gate execution (if API connected)
- "View latest warnings" → navigates to Warnings page
- "Compare drift" → navigates to Drift page with latest two snapshots pre-selected

---

### 2. Gates

**Purpose**: Explore gate run history and drill into individual check results. Web equivalent of the Gate Explorer TUI, but with history across runs.

#### Sub-views

**2a. Gate History (default)**

Table of all gate runs, most recent first:

| Column | Data |
|---|---|
| Timestamp | `provenance.timestamp` |
| Status | Overall pass/fail badge |
| Score | Numeric score with color coding |
| Checks | `passed/total` summary |
| Trigger | manual / pre-commit / CI / watch / api |
| Duration | Execution time in ms |
| Scope | directory / staged / files / plan |
| Files | Count of files checked |
| Actor | User or CI system |

Filters: status (pass/fail), trigger type, date range, minimum score.

Clicking a row opens the **Gate Detail** view.

**2b. Gate Detail**

Full breakdown of a single gate run. Maps directly to the Gate Explorer TUI but richer:

- **Header**: Overall status badge, score, timestamp, duration, trigger, scope.
- **Check List**: Expandable list of all checks run.
  - Each check shows: name, status (passed/failed/skipped), score, message, duration.
  - Expanding reveals: detailed output, specific issues found, file references.
- **Evidence Panel**: Raw evidence entries (`EvidenceEntrySchema`) with status, timestamps, details.
- **Provenance Footer**: Full provenance record — environment, git context, AI tool detection.

**2c. Gate Trends**

- Pass rate by check type over time (which checks fail most?)
- Average score trend
- Most common failure reasons
- Duration trends (are gates getting slower?)

---

### 3. Warnings

**Purpose**: Browse, filter, and understand all warnings in the codebase. This is the most data-dense page — it's where developers spend time understanding what needs fixing.

#### Sub-views

**3a. Warning List (default)**

Table of all active warnings from the latest analysis:

| Column | Data |
|---|---|
| ID | Warning ID (AP-001, ARCH-002, BOUND-001) |
| Severity | Error / Warning / Info badge |
| Category | Anti-pattern / Boundary / Architecture |
| Title | Human-readable title |
| File | File path (clickable, opens in context) |
| Line | Line number |
| New? | Drift indicator — is this a new violation? |
| Suppressed? | Whether an `@anvil-ignore` exists |

Filters: severity, category, file path glob, new-only, suppressed/unsuppressed, confidence level.

Group-by options: file, category, severity, pattern ID.

Clicking a warning opens a **Warning Detail** slide-out panel:
- Full message and explanation
- Suggestion for fixing
- Code context (surrounding lines with the violation highlighted)
- Suppression status (if suppressed: reason, author, expiry)
- Drift info (is this new? how many existing instances?)

**3b. Warning Breakdown**

Visual summary of warning distribution:
- **By pattern**: Bar chart of warning counts per pattern ID (AP-001, AP-003, etc.)
- **By file**: Treemap or bar chart of warnings per file (hotspot identification)
- **By severity**: Donut chart (error/warning/info split)
- **By category**: Donut chart (anti-pattern/boundary/architecture split)
- **New vs existing**: Stacked bar showing drift — how many warnings are new this analysis vs. baseline

**3c. Anti-Pattern Registry**

Reference page listing all defined anti-patterns:

| Column | Data |
|---|---|
| ID | AP-001 through AP-007 |
| Name | Human name |
| Category | escape-hatch / error-handling / code-quality / type-safety |
| Severity | Default severity |
| Enabled | Yes/No |
| Opt-in | Whether this is opt-in |
| Current count | How many instances exist now |
| Trend | Sparkline of count over time |

Clicking opens full documentation: explanation, detection config, suggestion, allowlist.

---

### 4. Architecture

**Purpose**: Visualize and understand the project's architecture boundaries, layers, and violations. This is where the web UI significantly exceeds the CLI — graphical dependency visualizations are not possible in a terminal.

#### Sub-views

**4a. Architecture Overview (default)**

- **Layer diagram**: Visual representation of defined layers (presentation → application → domain → infrastructure → shared) with dependency arrows showing allowed and violated paths.
- **Summary cards**: Total modules, total violations, new violations, error/warn/info counts, circular dependencies, orphan modules.
- **Template info**: Which architecture template is active (starter/layered/hexagonal/clean/ddd/monorepo).

**4b. Boundary Violations**

Table of all boundary violations:

| Column | Data |
|---|---|
| From file | Source file path |
| To file | Target file path |
| From layer | Source layer |
| To layer | Target layer |
| Rule | Which boundary rule is violated |
| Severity | Error / Warn / Info |
| New? | Is this violation new since baseline? |
| Type | import / require / dynamic |
| Line | Line number in source file |

Filters: severity, layer pair, new-only, file path.

**4c. Dependency Graph**

Interactive graph visualization:
- Nodes = modules/files, colored by layer
- Edges = dependencies, colored by violation status (green = allowed, red = violation, orange = warning)
- Click a node to see its details: layer, dependency count, dependent count, orphan status
- Filter by layer, zoom to specific boundaries
- Highlight circular dependencies

**4d. Layer Statistics**

Table per layer:

| Layer | Module Count | Violations From | Violations To | Dependencies |
|---|---|---|---|---|
| presentation | 12 | 0 | 3 | [application] |
| domain | 8 | 2 | 0 | [] |
| ... | ... | ... | ... | ... |

**4e. Entry Points**

List of detected entry points with type (package/application/http/api/cli/worker/test), confidence level, and exports.

---

### 5. Drift

**Purpose**: Track how the codebase changes over time. Answers: "Are we getting better or worse?" This maps to `anvil drift` but with rich comparison views.

#### Sub-views

**5a. Drift Timeline (default)**

- **Snapshot list**: All drift snapshots ordered by date, each showing: name, timestamp, key metrics (boundary violations, antipattern count, suppression count, files analysed).
- **Trend charts**: Line charts of each metric over time across all snapshots.
- **Overall trend badge**: Improving / Stable / Degrading (computed from `SnapshotComparison.overall_trend`).

**5b. Snapshot Detail**

Deep dive into a single snapshot:
- **Metrics panel**: boundary_violations, antipattern_count, suppression_count, expired_suppressions, files_analysed.
- **Anti-pattern breakdown**: Bar chart by pattern ID.
- **Hotspots**: Ranked list of files with highest violation count, with types.
- **Violations list**: Full table of all boundary violations in this snapshot.
- **Anti-patterns list**: Full table of all anti-pattern instances.
- **Suppressions list**: All active suppressions with expiry status.

**5c. Snapshot Comparison**

Side-by-side or diff view between two snapshots:
- **Metrics diff**: Each metric shown as `before → after` with delta and direction arrow.
- **Net change summary**: +/- violations, +/- antipatterns, +/- suppressions.
- **New violations**: Items present in "after" but not "before".
- **Resolved violations**: Items present in "before" but not "after".
- **Anti-pattern changes**: Per-pattern delta (e.g., AP-001: -3, AP-003: +1).
- **Duration**: Days between snapshots.

---

### 6. Suppressions

**Purpose**: Manage `@anvil-ignore` suppressions across the codebase. Anvil's philosophy is that suppressions are accountable — they require justification. This page makes suppression management visible and auditable.

#### Sub-views

**6a. Active Suppressions (default)**

Table of all current suppressions:

| Column | Data |
|---|---|
| Pattern | Which warning is suppressed (AP-001, etc.) |
| File | File path |
| Line | Line number |
| Reason | Justification text |
| Scope | line / statement / file |
| Author | Who created it |
| Created | Timestamp |
| Expires | Expiry date (if time-boxed) |
| Status | Active / Expiring Soon / Expired |

Filters: pattern, scope, author, expiry status, file path.

**6b. Expiring Soon**

Filtered view showing only suppressions that will expire within a configurable window (7/14/30 days). These need attention — either the underlying issue should be fixed or the suppression renewed with fresh justification.

**6c. Expired**

Suppressions past their expiry date. These represent warnings that are now active again and may be causing gate failures.

**6d. Suppression Trends**

- **Total suppressions over time**: Are we accumulating technical debt via suppressions?
- **By pattern**: Which patterns are most suppressed?
- **By author**: Who creates the most suppressions? (Not for blame — for understanding where friction exists.)
- **Average lifetime**: How long do suppressions typically live?
- **Expiry compliance**: What percentage of suppressions are time-boxed vs. permanent?

---

### 7. Plans

**Purpose**: View and manage APS plans. Plans are Anvil's unit of tracked change — they bundle intent, proposed changes, evidence, approvals, and execution results.

#### Sub-views

**7a. Plan List (default)**

Table of all plans:

| Column | Data |
|---|---|
| ID | `aps-[hash]` |
| Intent | Plan intent text |
| Status | Draft / Validated / Approved / Applied / Rolled back |
| Source | cli / api / automation / manual |
| Author | Provenance author |
| Created | Timestamp |
| Changes | Count of proposed changes |
| Evidence | Count of evidence entries |
| Tags | Tag list |

Filters: status, source, author, date range, tags.

**7b. Plan Detail**

Full plan view:
- **Header**: ID, hash, intent, schema version, status, provenance.
- **Proposed Changes**: Expandable list of changes (file_create, file_update, etc.) with path, description, diff/content preview.
- **Validations**: Schema valid, hash verified, changes valid, all check results.
- **Evidence Trail**: All evidence bundles — gate version, overall status, individual check results, timestamps. This is the accountability record.
- **Approval**: Approved by, when, notes. Or pending approval status.
- **Execution History**: Apply/rollback/dry-run results with success/failure details, change-by-change status.
- **Metadata & Tags**: Any additional metadata.

**7c. Plan Comparison**

Compare two plans or a plan's state before/after execution.

---

### 8. Audit Trail

**Purpose**: Full provenance history. Every gate run, every check, every action is recorded. This page is the compliance and accountability backbone.

#### Sub-views

**8a. Audit Log (default)**

Chronological log of all provenance records:

| Column | Data |
|---|---|
| Timestamp | Record timestamp |
| Event | What happened (gate run, check, plan action) |
| Result | Passed / Failed |
| Score | Overall score |
| Trigger | manual / pre-commit / CI / watch / api |
| Scope | directory / staged / files / plan |
| Files | Count of files checked |
| Duration | Execution time |
| User | Actor |
| AI Tool | Detected AI tool (cursor/copilot/claude-code/etc.) |
| Git | Branch, commit (short hash) |

Filters: result, trigger, user, AI tool, date range, branch.

**8b. User Activity**

Grouped by user:
- Gate runs per user
- Pass/fail rate per user
- Most common triggers
- Most active times

**8c. AI Tool Tracking**

Grouped by detected AI tool:
- Which AI tools are being used?
- Pass rate by AI tool (do some tools produce more violations?)
- Confidence of AI tool detection (high/medium/low/inferred)

This is a unique Anvil capability — understanding how different AI coding tools affect code quality.

**8d. Environment & CI**

- Gate runs by environment (local vs CI)
- CI provider breakdown
- Node version distribution
- Anvil version distribution

---

### 9. AI Dashboard Builder

**Purpose**: The json-render integration point. Advanced users compose custom dashboards via natural language prompts.

This page is the realization of the json-render brainstorm — it doesn't replace the structured pages above; it supplements them with infinite customisation.

#### Layout

- **Left panel**: Prompt input (multi-line text area) + "Generate" button
- **Right panel**: Live preview of generated dashboard (progressive rendering as JSON streams in)
- **Bottom bar**: Save / Share / Export / Edit JSON directly

#### Workflow

1. User types: *"Show me a grid with gate pass rate this month, top 5 warning hotspots, and a drift trend chart"*
2. AI generates constrained JSON referencing catalog components
3. Dashboard renders progressively in the preview panel
4. User can iterate ("Add a suppression expiry countdown"), save, or share

#### Template Gallery

Pre-built dashboard templates as starting points:
- **Team Health** — pass rate, drift score, warning count, recent activity
- **Code Quality Deep Dive** — anti-pattern breakdown, hotspots, trends
- **Architecture Compliance** — boundary violations, layer stats, drift
- **Suppression Audit** — active suppressions, expiry timeline, trends
- **CI Pipeline Monitor** — gate history, failure analysis, duration trends
- **AI Tool Impact** — AI tool detection stats, pass rate by tool

Users can start from a template and customise via prompt.

#### Saved Dashboards

- Per-user saved configurations (stored as JSON)
- Shareable via link (the JSON is the persistence format)
- Version history of dashboard configurations
- Pin favorites to sidebar navigation

---

### 10. Configuration

**Purpose**: View and manage Anvil settings. Maps to `.anvilrc` and gate configuration.

#### Sections

- **General**: Project name, planning directory, format, schema version.
- **Checks**: Enable/disable individual checks, set thresholds.
- **Architecture**: View/edit architecture definition (template, layers, boundaries, rules).
- **Policy**: OPA policy configuration, custom rules.
- **Watch**: File patterns, debounce settings, actions.
- **Integrations**: CI hooks, API keys, notification settings.

Read-only for most settings (config lives in `.anvilrc` which is code-managed). Some settings could be editable if the API supports writes.

---

### 11. Diagnostics

**Purpose**: Web equivalent of `anvil doctor`. System health checks for the Anvil installation.

- **Check list**: Each diagnostic check with pass/warn/fail status
- **Auto-fix suggestions**: Actionable steps for each issue
- **Environment info**: OS, Node, Anvil version, pnpm version
- **Connectivity**: API status, data freshness, last sync time

---

## Cross-Cutting Features

### Global Search

Unified search across all data domains:
- Warning text → navigates to Warning Detail
- File paths → shows all warnings/violations for that file
- Plan IDs → navigates to Plan Detail
- Pattern IDs (AP-001) → shows pattern info + all instances
- User names → shows audit activity

### Notifications

Non-intrusive indicators:
- Suppressions expiring soon (badge on Suppressions nav item)
- Gate failures since last visit
- New drift snapshots available
- Plans pending approval

### Data Freshness

Every page shows when data was last updated. If the web UI is reading from `.anvil/` storage, it needs to indicate staleness. Options:
- Periodic polling (configurable interval)
- Manual refresh button (per-page)
- WebSocket/SSE push from a running `anvil watch` or `anvil-api` process

### Export

Every table and chart should support:
- **Copy as JSON** — raw data
- **Copy as CSV** — for spreadsheet import
- **Copy as Markdown** — for pasting into PRs/issues
- **Screenshot** — chart image export

### URL Deep Linking

Every view, filter state, and detail panel should be URL-addressable:
- `/gates/run/abc123` → specific gate run
- `/warnings?severity=error&new=true` → filtered warning list
- `/drift/compare/snap-1/snap-2` → specific snapshot comparison
- `/audit?user=jdoe&trigger=ci` → filtered audit log

This enables sharing specific views in Slack, PRs, and documentation.

---

## Role-Based Views

json-render supports conditional visibility. Different roles see different default content:

### Developer

- **Overview**: Personal pass rate, warnings in their recent files, suppression expiry reminders.
- **Focus**: Warning detail, architecture understanding, "how do I fix this?"

### Team Lead

- **Overview**: Team-level metrics, per-developer pass rates, aggregate trends.
- **Focus**: Drift trajectory, suppression accumulation, hotspot identification.

### Platform / DevEx Engineer

- **Overview**: Cross-project metrics, CI pipeline health, tool adoption.
- **Focus**: Gate configuration, policy management, diagnostics.

### Security / Compliance

- **Overview**: Audit completeness, suppression justification quality, evidence coverage.
- **Focus**: Audit trail, provenance records, export for compliance reporting.

---

## Data Layer Requirements

The web UI needs data from these sources. This informs the `anvil-api` design:

### Read Endpoints Needed

| Endpoint | Source | Data |
|---|---|---|
| `GET /status` | `.anvilrc` + `.anvil/cache` | Project health summary |
| `GET /gates` | Provenance records | Gate run history |
| `GET /gates/:id` | Provenance records | Single gate run detail |
| `GET /warnings` | Latest check results | Current warning list |
| `GET /warnings/summary` | Latest check results | Warning breakdown |
| `GET /antipatterns` | Anti-pattern registry | Pattern definitions |
| `GET /architecture` | Architecture definition | Layers, boundaries, rules |
| `GET /architecture/context` | Architecture analysis | Violations, modules, stats |
| `GET /drift/snapshots` | `.anvil/` snapshot storage | All drift snapshots |
| `GET /drift/snapshots/:id` | Snapshot storage | Single snapshot |
| `GET /drift/compare` | Snapshot comparison | Diff between two snapshots |
| `GET /suppressions` | Suppression parser | All active suppressions |
| `GET /plans` | Plan index | All plans |
| `GET /plans/:id` | Plan storage | Single plan detail |
| `GET /audit` | Provenance storage | Audit log |
| `GET /audit/users` | Provenance storage | Per-user activity |

### Write Endpoints (Phase 2+)

| Endpoint | Action |
|---|---|
| `POST /gates/run` | Trigger a gate run |
| `POST /plans/:id/approve` | Approve a plan |
| `POST /suppressions/:id/renew` | Extend a suppression |
| `POST /dashboards` | Save a custom dashboard |
| `PUT /dashboards/:id` | Update a saved dashboard |
| `DELETE /dashboards/:id` | Delete a saved dashboard |

### Real-Time (Phase 3+)

| Channel | Events |
|---|---|
| `ws://watch` | File change events, gate run results, status updates |
| `sse://audit` | New provenance records as they happen |

---

## Technology Choices (Proposed)

| Concern | Choice | Rationale |
|---|---|---|
| Framework | React 19 | json-render requires React; team already uses Ink (React for terminals) |
| Routing | TanStack Router | Type-safe routes, matches our Zod-first approach |
| Data fetching | TanStack Query | Cache management, background refetch, optimistic updates |
| Styling | Tailwind CSS 4 | Utility-first, fast iteration, good density control |
| Charts | Recharts or Nivo | React-native charting, composable, good for dashboards |
| Tables | TanStack Table | Headless, sortable/filterable/paginated, our data is tabular |
| Component library | Shadcn/ui or Radix primitives | Unstyled accessible primitives, full design control |
| State management | Zustand | Lightweight, no boilerplate, good for dashboard state |
| AI rendering | @json-render/react | The foundation from the json-render brainstorm |
| Build | Vite | Already in the monorepo toolchain |
| Testing | Vitest + Playwright | Already in the monorepo toolchain |

---

## Component Catalog Mapping

Connecting the json-render component catalog (from json-render-dashboard.md) to specific page needs:

| Component | Used On |
|---|---|
| `GridLayout` | Overview, AI Builder, any multi-widget page |
| `Section` | All pages (titled content areas) |
| `TabGroup` | Gate Detail, Plan Detail, Architecture |
| `Sidebar` | App shell, AI Builder |
| `DataTable` | Gates, Warnings, Suppressions, Plans, Audit |
| `MetricCard` | Overview top row, Drift summary, Gate trends |
| `StatusBadge` | Everywhere (pass/fail/warn/info indicators) |
| `Timeline` | Overview activity feed, Plan execution history |
| `CodeBlock` | Warning detail (code context), Plan changes |
| `LineChart` | Overview trends, Drift timeline, Gate trends |
| `BarChart` | Warning breakdown, Anti-pattern counts |
| `SparklineChart` | Overview metric cards, Anti-pattern registry |
| `HeatMap` | Warning hotspots by file by day |
| `GateResultCard` | Gate detail, Overview recent activity |
| `WarningList` | Warning page, Gate detail check expansion |
| `DriftIndicator` | Overview, Drift page header |
| `SuppressionRequest` | Suppression list, Warning detail panel |
| `PlanCard` | Plan list, Overview |
| `EvidenceEntry` | Gate detail evidence panel, Plan detail |
| `FileViolationMap` | Architecture page, Warning hotspots |
| `DateRangeFilter` | All list pages (global filter) |
| `PackageSelector` | All pages (monorepo scope filter) |
| `SearchInput` | Global search, per-table search |
| `RefreshButton` | All pages (data freshness control) |

---

## Open Questions

1. **API-first or file-first?** — Should the web UI read from `.anvil/` storage directly (via a thin file-serving API), or should we build a full REST API (`anvil-api`) first? File-first is faster to ship but limits deployment to same-machine. API-first enables remote dashboards and team sharing.

2. **Authentication model** — For solo use (developer on their machine), no auth needed. For team use, need user identity. Options: GitHub OAuth, simple API keys, or defer to deployment platform (Vercel, etc.).

3. **Multi-project** — The project selector in the top bar implies multi-project support. How does that work? Options: one API per project, a meta-API that aggregates, or a single deployment pointing at a monorepo.

4. **Offline / static export** — Can we generate a static HTML report (no server needed) for CI artifacts? Like a gate report that gets uploaded as a build artifact. This would be a subset of the full dashboard.

5. **Shared component logic with CLI TUI** — Both web and CLI use React. Can we share component logic (not rendering) between Ink and React DOM? The data formatting, filtering, and aggregation logic is identical. A shared `@anvil/dashboard-logic` package could hold hooks and utilities both targets consume.

6. **Dark mode** — Engineers live in dark mode. Should this be the only theme, or do we support both? Recommendation: dark default, light available, follows system preference.

7. **Mobile** — Is mobile viewing a priority? Dashboard data is inherently wide (tables, charts). Recommendation: responsive down to tablet, not phone-optimised. Use the mobile viewport for the Overview page only.

8. **Notification delivery** — In-app badges are straightforward. But should we also support Slack/email/webhook notifications for events like "suppression expiring" or "gate failure in CI"? This could be a separate concern (`anvil-notify`).

---

## Implementation Priority

Based on value delivered vs. effort:

### Tier 1 — Core (ship first)

1. **Overview page** — immediate value, showcases the product
2. **Warnings page** — most frequently needed data
3. **Gates page** — gate history is the primary audit mechanism
4. **Global search + deep linking** — usability foundation

### Tier 2 — Differentiation

5. **Architecture visualization** — this is where web >> CLI
6. **Drift comparison** — side-by-side snapshot diffs
7. **Suppression management** — accountability visibility

### Tier 3 — Power Features

8. **AI Dashboard Builder** — the json-render showcase
9. **Audit trail** — compliance and team insights
10. **Plans page** — plan lifecycle management

### Tier 4 — Polish

11. **Configuration page** — settings management
12. **Diagnostics page** — doctor equivalent
13. **Role-based views** — conditional content
14. **Real-time updates** — WebSocket integration

---

## Verdict

The web UI should be a **data-dense engineering dashboard**, not a marketing-friendly admin panel. It has 11 core pages organized around Anvil's domain concepts (gates, warnings, architecture, drift, suppressions, plans, audit). Every structured page has a default view that works out of the box. The AI Dashboard Builder provides escape velocity for custom views — but the structured pages are where 90% of usage happens.

The json-render component catalog (28 components) maps cleanly to page needs. Every component is used by at least one structured page, and the AI Builder can compose them freely for custom layouts.

Ship Tier 1 (Overview + Warnings + Gates + Search) first. That covers the daily workflow: "open dashboard, see health, drill into problems, search for specifics." Everything else layers on top.
