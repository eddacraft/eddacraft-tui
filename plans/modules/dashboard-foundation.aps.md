# Dashboard Foundation

| ID | Owner | Status |
|----|-------|--------|
| DASH | @eddacraft | Draft |

## Purpose

Establish the web dashboard application scaffold, routing, layout shell, shared
component catalog, API data layer, and cross-cutting infrastructure (search,
deep linking, theming) that all dashboard pages depend on. This is the
foundation that makes individual page modules possible.

## In Scope

- React application scaffold in `apps/anvil-ui`
- Routing with type-safe route definitions
- App shell layout (top bar, sidebar navigation, content area)
- Theme system with dark/light mode
- Shared component catalog (json-render compatible)
- API server with read endpoints exposing `.anvil/` storage data
- Data fetching layer with caching and background refresh
- Global search across all data domains
- URL deep linking with filter state persistence
- Data export (JSON, CSV, Markdown) for all tables and charts

## Out of Scope

- Individual page implementations (see DASHCORE, DASHARCH, DASHOPS, DASHAI)
- Write endpoints (plan approval, suppression management) — deferred to page modules
- Authentication and multi-user — deferred until deployment model decided
- Real-time WebSocket/SSE updates — see DASHOPS-008

## Interfaces

**Depends on:**

- `monorepo-migration` — Nx workspace structure, `apps/anvil-ui` slot
- `contracts` — Zod schemas for all domain types (warnings, gates, plans, drift)
- `save-time-trust` — Analysis runner data formats
- `drift-reporting` — Snapshot and comparison schemas
- `architecture-safety` — Boundary and layer schemas
- `suppressions` — Suppression record schemas

**Exposes:**

- `apps/anvil-ui` — Deployable web application
- `apps/anvil-api` — Data-serving API (or embedded in anvil-ui dev server)
- Component catalog — Reusable, json-render-compatible React components
- Data hooks — `useGates()`, `useWarnings()`, `useDrift()`, etc.
- Theme tokens — Design system consumed by all pages
- Layout primitives — `AppShell`, `PageHeader`, `FilterBar`, etc.
- Export utilities — `exportAsJSON()`, `exportAsCSV()`, `exportAsMarkdown()`

## Decisions

**D-DASH-001:** API-embedded vs standalone

- **Options:** (a) Standalone `anvil-api` Express/Fastify server, (b) Embedded
  API routes in Vite dev server / SPA with file access
- **Recommendation:** Start with embedded (Vite plugin or lightweight server
  co-located with UI). Extracting to standalone is a refactor, not a rewrite.
- **Status:** Open

**D-DASH-002:** Component library approach

- **Options:** (a) Shadcn/ui (copy-paste primitives), (b) Radix + custom styling,
  (c) Headless UI + Tailwind
- **Recommendation:** Shadcn/ui — gives accessible primitives with full design
  control, large community, good Tailwind integration
- **Status:** Open

**D-DASH-003:** Chart library

- **Options:** (a) Recharts, (b) Nivo, (c) Victory, (d) Tremor
- **Recommendation:** Recharts — React-native, composable, good for dashboards,
  active maintenance
- **Status:** Open

## Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| API design locks in before domain stabilises | medium | Read-only API first; match existing CLI JSON output shapes |
| Component catalog grows without governance | medium | json-render schema validation constrains what AI can generate |
| Bundle size bloats with charting libraries | low | Tree-shaking + lazy loading per page route |

## Ready Checklist

Change status to **Ready** when:

- [ ] Purpose and scope are clear
- [ ] Dependencies identified
- [ ] At least one task defined
- [ ] Technology decisions (D-DASH-001 through D-DASH-003) resolved

## Tasks

### DASH-001: Application scaffold and build configuration

- **Intent:** Create a buildable, testable React application in the monorepo
- **Expected Outcome:** `apps/anvil-ui` builds with Vite, runs dev server, has
  base TypeScript and ESLint configuration matching monorepo standards
- **Scope:** `apps/anvil-ui/`
- **Non-scope:** Routing, layout, components
- **Files:**
  - `apps/anvil-ui/package.json`
  - `apps/anvil-ui/vite.config.ts`
  - `apps/anvil-ui/tsconfig.json`
  - `apps/anvil-ui/index.html`
  - `apps/anvil-ui/src/main.tsx`
- **Dependencies:** —
- **Validation:** `nx build anvil-ui && nx test anvil-ui`
- **Confidence:** high

### DASH-002: Routing and navigation shell

- **Intent:** Define all application routes and render a navigable app shell
- **Expected Outcome:** TanStack Router configured with type-safe routes for all
  planned pages; sidebar navigation renders links; route transitions work;
  404 handling in place
- **Scope:** `apps/anvil-ui/src/`
- **Non-scope:** Page content implementations
- **Files:**
  - `apps/anvil-ui/src/routes/`
  - `apps/anvil-ui/src/components/AppShell.tsx`
  - `apps/anvil-ui/src/components/Sidebar.tsx`
  - `apps/anvil-ui/src/components/TopBar.tsx`
- **Dependencies:** DASH-001
- **Validation:** All routes render placeholder content; sidebar links navigate correctly
- **Confidence:** high

### DASH-003: Theme system and design tokens

- **Intent:** Establish visual identity with dark/light mode support
- **Expected Outcome:** Tailwind CSS configured with Anvil design tokens (colors
  from CLI theme: ember, steel, slag, smoke); dark mode default with system
  preference detection; consistent spacing and typography scale
- **Scope:** `apps/anvil-ui/src/`
- **Non-scope:** Individual component styling
- **Files:**
  - `apps/anvil-ui/tailwind.config.ts`
  - `apps/anvil-ui/src/styles/`
  - `apps/anvil-ui/src/hooks/useTheme.ts`
- **Dependencies:** DASH-001
- **Validation:** Theme toggle switches between dark/light; tokens match CLI palette
- **Confidence:** high

### DASH-004: Shared component catalog

- **Intent:** Build the reusable component library that pages compose and
  json-render references
- **Expected Outcome:** Component catalog covering layout (GridLayout, Section,
  TabGroup), data display (DataTable, MetricCard, StatusBadge, Timeline,
  CodeBlock), charts (LineChart, BarChart, SparklineChart), and interactive
  elements (DateRangeFilter, SearchInput, RefreshButton). All components are
  json-render compatible (accept props, render deterministically).
- **Scope:** `apps/anvil-ui/src/components/`
- **Non-scope:** Anvil-specific domain components (GateResultCard, etc.)
- **Files:**
  - `apps/anvil-ui/src/components/catalog/`
  - `apps/anvil-ui/src/components/catalog/index.ts`
- **Dependencies:** DASH-003
- **Validation:** Component catalog renders in isolation (Storybook or test harness)
- **Confidence:** medium

### DASH-005: API data layer

- **Intent:** Serve Anvil domain data to the web UI over HTTP
- **Expected Outcome:** API endpoints that read from `.anvil/` storage and
  project files, exposing: status, gates (list + detail), warnings (list +
  summary), anti-patterns, architecture (definition + context), drift
  (snapshots + comparison), suppressions, plans (list + detail), audit log.
  Response shapes match existing Zod schemas from `@anvil/contracts`.
- **Scope:** `apps/anvil-api/` or `apps/anvil-ui/src/api/`
- **Non-scope:** Write operations, authentication, WebSocket
- **Files:**
  - API route handlers
  - Data access utilities reading `.anvil/` storage
- **Dependencies:** DASH-001
- **Validation:** Each endpoint returns valid JSON matching contract schemas
- **Confidence:** medium

### DASH-006: Data fetching hooks and cache management

- **Intent:** Provide React hooks that pages use to access API data with caching
- **Expected Outcome:** TanStack Query configured with sensible defaults (stale
  time, refetch intervals). Typed hooks for each data domain: `useGates()`,
  `useGateDetail(id)`, `useWarnings(filters)`, `useDriftSnapshots()`,
  `useSuppressions()`, `usePlans()`, `useAuditLog(filters)`. Loading, error,
  and empty states handled consistently.
- **Scope:** `apps/anvil-ui/src/hooks/`
- **Non-scope:** UI rendering of states
- **Files:**
  - `apps/anvil-ui/src/hooks/useGates.ts`
  - `apps/anvil-ui/src/hooks/useWarnings.ts`
  - `apps/anvil-ui/src/hooks/useDrift.ts`
  - `apps/anvil-ui/src/hooks/useSuppressions.ts`
  - `apps/anvil-ui/src/hooks/usePlans.ts`
  - `apps/anvil-ui/src/hooks/useAuditLog.ts`
  - `apps/anvil-ui/src/lib/api-client.ts`
- **Dependencies:** DASH-005
- **Validation:** Hooks return typed data; loading/error states handled; cache invalidation works
- **Confidence:** high

### DASH-007: Global search infrastructure

- **Intent:** Enable unified search across all data domains from the top bar
- **Expected Outcome:** Search input in top bar that queries across warnings,
  files, plans, pattern IDs, and users. Results grouped by domain with
  navigation to the relevant detail view. Keyboard shortcut (Cmd/Ctrl+K) to
  focus search.
- **Scope:** `apps/anvil-ui/src/components/`, `apps/anvil-ui/src/hooks/`
- **Non-scope:** Full-text search engine; this is filtered matching over API data
- **Files:**
  - `apps/anvil-ui/src/components/GlobalSearch.tsx`
  - `apps/anvil-ui/src/hooks/useSearch.ts`
- **Dependencies:** DASH-002, DASH-006
- **Validation:** Searching for a pattern ID (e.g., "AP-001") shows relevant results
- **Confidence:** medium

### DASH-008: URL deep linking and filter persistence

- **Intent:** Make every view, filter state, and detail panel URL-addressable
- **Expected Outcome:** Filter selections (date range, severity, status) are
  encoded in URL search params. Sharing a URL reproduces the exact view.
  Browser back/forward navigates filter state changes. Bookmarkable views.
- **Scope:** `apps/anvil-ui/src/hooks/`, `apps/anvil-ui/src/routes/`
- **Non-scope:** Server-side rendering
- **Files:**
  - `apps/anvil-ui/src/hooks/useFilterParams.ts`
  - Route definitions with search param schemas
- **Dependencies:** DASH-002
- **Validation:** Applying filters updates URL; pasting URL restores filters
- **Confidence:** high

## Execution

Steps: [../execution/DASH.steps.md](../execution/DASH.steps.md)
