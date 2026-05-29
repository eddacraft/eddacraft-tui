# Dashboard Foundation

| ID   | Owner      | Status | Progress |
| ---- | ---------- | ------ | -------- |
| DASH | @eddacraft | Ready  | 1/9      |

**Last reviewed:** 2026-04-26

## Purpose

Establish the dashboard route group, navigation shell, component catalogue, API
data layer, and cross-cutting infrastructure (search, deep linking, theming)
within the existing `apps/website/` Next.js application. The marketing site stays
at `/`; the dashboard lives under the `(dashboard)` route group at `/dashboard`.
This is the foundation that makes all page modules possible.

## In Scope

- `(dashboard)` route group with sidebar nav and top bar layout
- Extended theme tokens for dashboard (severity, status, chart palette)
- Shared component catalogue (MetricCard, DataTable, StatusBadge, etc.)
- Themed chart components via shadcn/ui + Recharts
- Next.js API routes reading `.anvil/` storage data
- Data fetching hooks with TanStack Query
- Command palette (Cmd+K) for global search
- URL deep linking with filter persistence
- Removal of `apps/anvil-ui/` placeholder

## Out of Scope

- Individual page implementations (see DASHCORE, DASHARCH, DASHOPS, DASHAI)
- Write endpoints (plan approval, suppression management) — deferred to page
  modules
- Authentication and multi-user — deferred until deployment model decided
- Real-time WebSocket/SSE updates — deferred

## Interfaces

**Depends on:**

- `apps/website/` — Existing Next.js 16 + Tailwind 4 + shadcn/ui application
- `contracts` — Zod schemas for all domain types (warnings, gates, plans, drift); see `schema-contracts` module
- `.anvil/` artefacts produced by the Rust CLI (`crates/anvil-cli/`) — API routes read JSON files; no TS core import needed [REVIEW: original plan referenced `@eddacraft/anvil-core`, retired per ADR-026 / anvil-ts-scanner-retirement]
- `drift-reporting` — Snapshot and comparison schemas [REVIEW: archived module — schemas now live with Rust kernel/contracts; verify availability before starting]
- `architecture-safety` — Boundary and layer schemas [REVIEW: archived module — boundary/layer logic now in `crates/anvil-architecture/`; verify schema source]
- `suppressions` — Suppression record schemas [REVIEW: archived module — suppression parser is now Rust per ADR-029; verify schema source]

**Exposes:**

- `(dashboard)` route group in `apps/website/`
- Component catalogue — Reusable shadcn/ui-based React components
- Data hooks — `useStatus()`, `useGates()`, `useWarnings()`, `useDrift()`, etc.
- Theme tokens — Extended design system consumed by all dashboard pages
- Layout primitives — `DashboardShell`, sidebar, top bar
- API routes at `app/api/anvil/*`

## Decisions

**D-DASH-001:** API approach

- **Options:** (a) Standalone `anvil-api` Hono server, (b) Next.js API routes
  reading `.anvil/` artefacts, (c) Embedded Vite plugin
- **Resolution:** Option (b) — Next.js API routes. Dashboard reads local
  `.anvil/` files (produced by the Rust CLI in `crates/anvil-cli/`); co-located
  API routes avoid a separate server process. The dashboard is primarily a
  local dev tool (`nx dev website`), not a hosted SaaS. `apps/anvil-api/`
  continues to serve cloud user/auth data from Neon Postgres.
- **Status:** Resolved

**D-DASH-002:** Component library

- **Options:** (a) shadcn/ui, (b) Radix + custom styling, (c) Headless UI +
  Tailwind
- **Resolution:** Option (a) — shadcn/ui. Already configured in `apps/website/`,
  provides accessible primitives with full design control and Tailwind
  integration.
- **Status:** Resolved

**D-DASH-003:** Chart library

- **Options:** (a) Recharts, (b) Nivo, (c) Victory, (d) Tremor
- **Resolution:** Option (a) — Recharts. shadcn/ui has pre-built chart wrappers
  for Recharts, making integration seamless.
- **Status:** Resolved

## Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| API design locks in before domain stabilises | medium | Read-only API first; match existing Zod schema shapes |
| Component catalogue grows without governance | medium | shadcn/ui primitives constrain scope; json-render schema validation in Wave 4 |
| Bundle size bloats with charting libraries | low | Tree-shaking + lazy loading per page route |

## Ready Checklist

Change status to **Ready** when:

- [x] Purpose and scope are clear
- [x] Dependencies identified
- [x] At least one task defined
- [x] Technology decisions (D-DASH-001 through D-DASH-003) resolved

## Wave

**Wave 1** — Must complete before Waves 2–4 can begin.

## Work Items

### DASH-001: Dashboard route group and layout shell

- **Intent:** Create `(dashboard)` route group with sidebar nav and top bar
- **Expected Outcome:** `/dashboard` renders a layout with collapsible sidebar,
  top bar with breadcrumbs and Cmd+K trigger, and main content area. The
  marketing site at `/` remains unchanged.
- **Files:**
  - `apps/website/app/(dashboard)/layout.tsx`
  - `apps/website/app/(dashboard)/dashboard/page.tsx`
  - `apps/website/components/dashboard/sidebar.tsx`
  - `apps/website/components/dashboard/top-bar.tsx`
  - `apps/website/components/dashboard/dashboard-shell.tsx`
- **Dependencies:** —
- **Validation:** `/dashboard` renders layout; marketing site unaffected
- **Confidence:** high

### DASH-002: Extended theme tokens for dashboard

- **Intent:** Add dashboard-specific colour tokens for severity, status, and
  chart palette
- **Expected Outcome:** CSS variables for severity (error→slag, warning→molten,
  info→steel), chart palette extending `--chart-1..5`, CLI theme colours (ember,
  steel, slag, molten, ash, smoke, charcoal) as Tailwind utilities. Scoped to
  dashboard layout.
- **Files:**
  - `apps/website/app/(dashboard)/dashboard.css`
  - `apps/website/app/(dashboard)/layout.tsx`
- **Dependencies:** DASH-001
- **Validation:** Theme tokens render correctly; colours match CLI palette
- **Confidence:** high

### DASH-003: Shared dashboard component catalogue

- **Intent:** Build reusable components all pages compose
- **Expected Outcome:** `MetricCard`, `DataTable` (sortable, filterable,
  paginated), `StatusBadge`, `SeverityBadge`, `CodeBlock`, `EmptyState`,
  `LoadingSkeleton` — all on shadcn/ui primitives with extended theme tokens.
- **Files:**
  - `apps/website/components/dashboard/metric-card.tsx`
  - `apps/website/components/dashboard/data-table.tsx`
  - `apps/website/components/dashboard/status-badge.tsx`
  - `apps/website/components/dashboard/severity-badge.tsx`
  - `apps/website/components/dashboard/code-block.tsx`
  - `apps/website/components/dashboard/empty-state.tsx`
  - `apps/website/components/dashboard/loading-skeleton.tsx`
- **Dependencies:** DASH-002
- **Validation:** Components render in isolation with correct theming
- **Confidence:** high

### DASH-004: Chart components via shadcn/ui + Recharts

- **Intent:** Themed chart components for all dashboard visualisations
- **Expected Outcome:** `LineChart`, `AreaChart`, `BarChart`, `SparklineChart`,
  `DonutChart` wrapping Recharts with Anvil theme tokens. Responsive, hover
  tooltips, consistent style.
- **Files:**
  - `apps/website/components/dashboard/charts/line-chart.tsx`
  - `apps/website/components/dashboard/charts/area-chart.tsx`
  - `apps/website/components/dashboard/charts/bar-chart.tsx`
  - `apps/website/components/dashboard/charts/sparkline-chart.tsx`
  - `apps/website/components/dashboard/charts/donut-chart.tsx`
  - `apps/website/components/dashboard/charts/index.ts`
- **Dependencies:** DASH-002
- **Validation:** Charts render with theme tokens; tooltips work; responsive
- **Confidence:** medium

### DASH-005: API data layer — Next.js API routes

- **Intent:** Serve Anvil domain data via Next.js API routes reading `.anvil/`
- **Expected Outcome:** Routes at `app/api/anvil/`: `status`, `gates`,
  `gates/[id]`, `warnings`, `drift/snapshots`, `drift/snapshots/[name]`,
  `drift/compare`, `suppressions`, `config`, `provenance`. All use
  `runtime: 'nodejs'`, return JSON matching contract schemas.
- **Files:**
  - `apps/website/app/api/anvil/status/route.ts`
  - `apps/website/app/api/anvil/gates/route.ts`
  - `apps/website/app/api/anvil/gates/[id]/route.ts`
  - `apps/website/app/api/anvil/warnings/route.ts`
  - `apps/website/app/api/anvil/drift/snapshots/route.ts`
  - `apps/website/app/api/anvil/drift/snapshots/[name]/route.ts`
  - `apps/website/app/api/anvil/drift/compare/route.ts`
  - `apps/website/app/api/anvil/suppressions/route.ts`
  - `apps/website/app/api/anvil/config/route.ts`
  - `apps/website/app/api/anvil/provenance/route.ts`
  - `apps/website/lib/anvil/workspace.ts`
- **Dependencies:** DASH-001
- **Validation:** Each endpoint returns valid JSON matching contract schemas
- **Confidence:** medium

### DASH-006: Data fetching hooks with TanStack Query

- **Intent:** Typed React hooks for API data with caching and background refresh
- **Expected Outcome:** TanStack Query configured in dashboard layout. Hooks:
  `useStatus()`, `useGates(filters?)`, `useGateDetail(id)`,
  `useWarnings(filters?)`, `useDriftSnapshots()`, `useSnapshotDetail(name)`,
  `useSnapshotComparison(a,b)`, `useSuppressions(filters?)`, `useConfig()`,
  `useProvenance(filters?)`. Sensible stale times. `QueryBoundary` wrapper for
  loading/error states.
- **Files:**
  - `apps/website/lib/anvil/api-client.ts`
  - `apps/website/hooks/use-status.ts`
  - `apps/website/hooks/use-gates.ts`
  - `apps/website/hooks/use-warnings.ts`
  - `apps/website/hooks/use-drift.ts`
  - `apps/website/hooks/use-suppressions.ts`
  - `apps/website/hooks/use-config.ts`
  - `apps/website/hooks/use-provenance.ts`
  - `apps/website/components/dashboard/query-boundary.tsx`
  - `apps/website/app/(dashboard)/providers.tsx`
- **Dependencies:** DASH-005
- **Validation:** Hooks return typed data; loading/error states handled; cache
  invalidation works
- **Confidence:** high

### DASH-007: Command palette (global search)

- **Intent:** Unified search via Cmd+K across all data domains
- **Expected Outcome:** shadcn/ui `Command` component searching warnings, gates,
  drift snapshots, suppressions, provenance. Results grouped by domain with
  navigation. Recent searches in session storage.
- **Files:**
  - `apps/website/components/dashboard/command-palette.tsx`
  - `apps/website/hooks/use-search.ts`
- **Dependencies:** DASH-001, DASH-006
- **Validation:** Cmd+K opens palette; search returns grouped results; navigation
  works
- **Confidence:** medium

### DASH-008: URL deep linking and filter persistence

- **Intent:** Make every filter state URL-addressable
- **Expected Outcome:** Generic `useFilterParams()` hook syncing filter state
  with URL search params via `useSearchParams()`. Shareable URLs, browser
  back/forward navigates filter changes.
- **Files:**
  - `apps/website/hooks/use-filter-params.ts`
- **Dependencies:** DASH-001
- **Validation:** Applying filters updates URL; pasting URL restores filters;
  back/forward works
- **Confidence:** high

### DASH-009: Remove `apps/anvil-ui/` placeholder

- **Intent:** Clean up unused placeholder application
- **Expected Outcome:** Delete `apps/anvil-ui/` directory. Update file map in
  `aps-project.md` to reference `apps/website/` paths.
- **Files:**
  - `apps/anvil-ui/` (delete)
  - `.claude/rules/aps-project.md`
- **Dependencies:** DASH-001
- **Validation:** `apps/anvil-ui/` no longer exists; no broken references remain
- **Confidence:** high
- **Status:** Done
