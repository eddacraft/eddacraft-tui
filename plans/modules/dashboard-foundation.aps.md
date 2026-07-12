# Dashboard Foundation

| ID   | Owner      | Status | Progress |
| ---- | ---------- | ------ | -------- |
| DASH | @eddacraft | Ready  | 1/11     |

**Last reviewed:** 2026-07-09

## Purpose

Establish the dedicated browser dashboard host, local Rust dashboard server,
OpenAPI client seam, dashboard module model, and first proof modules. The
dashboard is no longer a route group inside `apps/website/`; it is a dedicated
Vite app in `apps/dashboard/` backed by a loopback-bound read-only API in
`crates/anvil-dashboard-server/`.

Dashboard modules are UI adapters over kernel capabilities. They own
navigation, layout, rendering, interaction, local UI state, and schema-driven
views. The kernel/server side owns permissions, workflow state, audit, evidence,
and policy decisions. A dashboard module may request an action in a later wave;
it must not decide whether that action is authorised.

## In Scope

- `apps/dashboard/` Vite + React + TanStack Router app scaffold
- Dashboard module host and navigation registry
- shadcn/ui + Tailwind v4 component catalogue for dashboard modules
- json-render-first dashboard catalogue, with Recharts only as chart primitives
  where needed
- `crates/anvil-dashboard-server/` local read-only API scaffold
- OpenAPI contract generation for dashboard endpoints
- Generated TypeScript client consumed through TanStack Query hooks
- Workspace/root containment for local artefact reads
- URL/search-param state through TanStack Router and Zod validation
- Protection Overview proof module for user-facing save-time protection state
- Plan Driver proof module for APS dogfood views
- Removal of `apps/anvil-ui/` placeholder

## Out of Scope

- Write endpoints and approval/transition actions
- Kernel action authority changes
- Authentication, multi-user, RBAC, and Better Auth/OIDC rollout
- Monaco and xterm.js
- Real-time WebSocket/SSE updates
- Hosted cloud access to local `.anvil/` artefacts
- Full downstream page implementations (see DASHCORE, DASHARCH, DASHOPS, DASHAI)

## Interfaces

**Depends on:**

- ADR-104 — dashboard host, server, and module authority boundary
- Rust kernel/crate APIs that own Anvil facts and decisions
- `.anvil/` and tracked `anvil/` artefacts produced by the CLI/kernel
- `packages/anvil/contracts` and Rust wire types where schemas already exist
- `@eddacraft/render` for json-render React/shadcn rendering
- `apps/website/` only as an existing website that DASH no longer embeds in

**Exposes:**

- `apps/dashboard/` — dedicated dashboard host
- `crates/anvil-dashboard-server/` — loopback-bound local read-only API
- OpenAPI contract for dashboard data
- Generated TypeScript dashboard client
- TanStack Query resource hooks
- Dashboard module registry and manifest shape
- Protection Overview dashboard module
- Plan Driver dashboard module

## Decisions

**D-DASH-001:** Dashboard host

- **Options:** (a) Dedicated Vite app, (b) Next.js route group in
  `apps/website/`, (c) migrate all website/docs/marketing surfaces now
- **Resolution:** Option (a) — `apps/dashboard/` is the dashboard host. It uses
  React, Vite 8, TanStack Router, TanStack Query, TanStack Table, shadcn/ui,
  Tailwind v4, Zod, and json-render-first composition. `apps/website/` remains
  the existing Next.js site unless separately migrated.
- **Status:** Resolved by ADR-104

**D-DASH-002:** Local API boundary

- **Options:** (a) `crates/anvil-dashboard-server/`, (b) Next API routes,
  (c) hosted `apps/anvil-api/`, (d) pure browser/static file access
- **Resolution:** Option (a) — Wave 1 introduces
  `crates/anvil-dashboard-server/` as a loopback-bound, read-only API that owns
  workspace artefact access through kernel/crate APIs. Browser code never reads
  local files directly. `apps/anvil-api/` remains the hosted cloud/user/auth API
  unless separately re-scoped.
- **Status:** Resolved by ADR-104

**D-DASH-003:** Client seam

- **Options:** (a) OpenAPI -> generated TypeScript client -> TanStack Query,
  (b) hand-written fetch wrappers only, (c) direct TS imports from domain code
- **Resolution:** Option (a). OpenAPI is the dashboard API contract, generated
  TypeScript clients are the app seam, and TanStack Query owns client-side
  server-state caching.
- **Status:** Resolved by ADR-104

**D-DASH-004:** Dashboard module authority model

- **Options:** (a) Dashboard modules as UI adapters over kernel capabilities,
  (b) page-owned domain logic, (c) plugin-owned action authority
- **Resolution:** Option (a). Modules own navigation, layout, rendering,
  interaction, local UI state, and schema-driven views. Kernel/server code owns
  permissions, workflow state, audit, evidence, and policy decisions. Action
  requests are deferred out of Wave 1.
- **Status:** Resolved by ADR-104

**D-DASH-005:** First proof modules

- **Options:** (a) Protection Overview + Plan Driver, (b) Plan Driver only,
  (c) Architecture graph first, (d) Suppressions first
- **Resolution:** Option (a). Protection Overview is the user-facing proof of
  Anvil's core promise. Plan Driver remains as an internal APS/dogfood module.
- **Status:** Resolved by ADR-104

## Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| Local API reads outside the intended workspace | high | Canonical root containment, symlink handling, path traversal tests, read-only policy |
| OpenAPI client churn while endpoint shapes stabilise | medium | Start with narrow Protection Overview and Plan Driver endpoint groups |
| Dashboard modules grow authority accidentally | high | Module manifest permits action requests only; server/kernel remains authority |
| Visual scope expands before the seam is proven | medium | Wave 1 proves host/server/client/modules before rich graphs or write flows |
| `apps/website` and `apps/dashboard` ownership blurs | medium | ADR-104 keeps website migration separate from DASH |

## Ready Checklist

Change status to **Ready** when:

- [x] Purpose and scope are clear
- [x] Dependencies identified
- [x] At least one task defined
- [x] Technology and authority decisions (D-DASH-001 through D-DASH-005)
      resolved by ADR-104

## Wave

**Wave 1** — Must complete before Waves 2-4 can begin.

## Work Items

### DASH-001: Vite dashboard app scaffold

- **Status:** In Progress 2026-07-10 — scaffold started on
  `feat/dash-001-dashboard-scaffold`.
- **Intent:** Create the dedicated dashboard app host.
- **Expected Outcome:** `apps/dashboard/` builds and serves a React 19 +
  Vite 8 app with TanStack Router, Tailwind v4, shadcn/ui configuration, and
  Nx targets for `dev`, `build`, `test`, `lint`, and `typecheck`.
- **Files:**
  - `apps/dashboard/package.json`
  - `apps/dashboard/project.json`
  - `apps/dashboard/index.html`
  - `apps/dashboard/vite.config.ts`
  - `apps/dashboard/tsconfig.json`
  - `apps/dashboard/components.json`
  - `apps/dashboard/src/main.tsx`
  - `apps/dashboard/src/routes/__root.tsx`
  - `apps/dashboard/src/routes/index.tsx`
  - `apps/dashboard/src/styles.css`
- **Dependencies:** —
- **Validation:** `pnpm exec nx run dashboard:build`; dashboard root renders
  without involving `apps/website/`
- **Confidence:** high

### DASH-002: Dashboard module host and navigation

- **Status:** Done 2026-07-13 — manifests, fail-closed registry, and
  registry-driven desktop/mobile navigation verified on `feat/dash-wave-1`.
- **Intent:** Define the dashboard module adapter shape and navigation registry.
- **Expected Outcome:** Dashboard modules register manifests with route entries,
  nav metadata, query bindings, renderers, and optional action-request
  descriptors. The host renders the sidebar/top bar and module routes without
  giving modules authority over kernel decisions.
- **Files:**
  - `apps/dashboard/src/modules/manifest.ts`
  - `apps/dashboard/src/modules/registry.ts`
  - `apps/dashboard/src/modules/registry.test.ts`
  - `apps/dashboard/src/components/shell/dashboard-shell.tsx`
  - `apps/dashboard/src/components/shell/sidebar.tsx`
  - `apps/dashboard/src/components/shell/top-bar.tsx`
  - `apps/dashboard/src/components/shell/mobile-navigation.tsx`
  - `apps/dashboard/src/components/shell/workspace-switcher.tsx`
  - `apps/dashboard/src/routes/__root.tsx`
- **Dependencies:** DASH-001
- **Validation:** `pnpm exec nx run dashboard:test --skip-nx-cache` (11 tests);
  `pnpm exec nx run dashboard:typecheck --skip-nx-cache`; duplicate and unknown
  module identifiers fail closed.
- **Confidence:** high

### DASH-003: Theme and component catalogue

- **Status:** Done 2026-07-13 — shared primitives and a thin dashboard adapter
  over the authoritative `@eddacraft/render` catalogue verified on
  `feat/dash-wave-1`.
- **Intent:** Build the shared UI primitives and json-render catalogue used by
  dashboard modules.
- **Expected Outcome:** Dashboard theme tokens cover severity, status, and chart
  palette. Shared components include `MetricCard`, `DataTable`, `StatusBadge`,
  `SeverityBadge`, `CodeBlock`, `EmptyState`, `LoadingSkeleton`, and chart
  primitives registered for json-render where appropriate.
- **Files:**
  - `apps/dashboard/src/lib/theme.ts`
  - `apps/dashboard/src/components/primitives/metric-card.tsx`
  - `apps/dashboard/src/components/primitives/data-table.tsx`
  - `apps/dashboard/src/components/primitives/status-badge.tsx`
  - `apps/dashboard/src/components/primitives/severity-badge.tsx`
  - `apps/dashboard/src/components/primitives/code-block.tsx`
  - `apps/dashboard/src/components/primitives/empty-state.tsx`
  - `apps/dashboard/src/components/primitives/loading-skeleton.tsx`
  - `apps/dashboard/src/components/primitives/charts.tsx`
  - `apps/dashboard/src/lib/render/catalog.ts`
  - `apps/dashboard/src/lib/render/catalog.test.tsx`
  - `apps/dashboard/src/components/ui/card.tsx`
  - `apps/dashboard/src/components/ui/empty.tsx`
  - `apps/dashboard/src/components/ui/skeleton.tsx`
  - `apps/dashboard/package.json`
  - `pnpm-lock.yaml`
- **Dependencies:** DASH-001
- **Validation:** `pnpm exec nx run dashboard:test --skip-nx-cache` (11 tests);
  dashboard lint, typecheck, and build targets exit 0; json-render accepts known
  catalogue components and rejects unknown component names.
- **Confidence:** medium

### DASH-004: Local dashboard server crate

- **Status:** Done 2026-07-13 — loopback listener enforcement, Host-header
  guard, read-only routing, health, and OpenAPI behaviour verified on
  `feat/dash-wave-1`.
- **Intent:** Create the Rust server boundary for local dashboard data.
- **Expected Outcome:** `crates/anvil-dashboard-server/` exposes a loopback-only
  read-only HTTP server with health and OpenAPI endpoints. It does not implement
  write/action execution.
- **Files:**
  - `crates/anvil-dashboard-server/Cargo.toml`
  - `crates/anvil-dashboard-server/src/lib.rs`
  - `crates/anvil-dashboard-server/src/main.rs`
  - `crates/anvil-dashboard-server/src/server.rs`
  - `crates/anvil-dashboard-server/src/openapi.rs`
  - `crates/anvil-dashboard-server/tests/server_smoke.rs`
  - `Cargo.toml`
- **Dependencies:** DASH-001
- **Validation:** `cargo test -p eddacraft-anvil-dashboard-server` (14 tests);
  dashboard-server clippy passes with warnings denied; the listener rejects
  non-loopback addresses and exposes only read routes, including `/healthz` and
  `/openapi.json`.
- **Confidence:** medium

### DASH-005: Workspace artefact read boundary

- **Status:** Done 2026-07-13 — held-root containment, traversal and symlink
  rejection, size limits, and structured read-error codes verified on
  `feat/dash-wave-1`.
- **Intent:** Make local `.anvil/` and tracked `anvil/` reads safe and explicit.
- **Expected Outcome:** Dashboard server resolves a configured workspace root,
  canonicalises requested paths, rejects traversal/symlink escapes, applies size
  limits, and returns structured errors. No endpoint accepts arbitrary path
  reads.
- **Files:**
  - `crates/anvil-dashboard-server/src/workspace.rs`
  - `crates/anvil-dashboard-server/src/error.rs`
  - `crates/anvil-dashboard-server/tests/workspace_boundary.rs`
- **Dependencies:** DASH-004
- **Validation:** `cargo test -p eddacraft-anvil-dashboard-server` (14 tests);
  boundary tests cover `..`, symlink escapes, outside-root absolute paths,
  missing artefacts, oversized artefacts, and stable structured error codes;
  `cargo fmt --all --check`.
- **Confidence:** high

### DASH-006: OpenAPI contract and generated TypeScript client

- **Status:** In Progress 2026-07-13 — deterministic Rust export, committed
  OpenAPI/TypeScript output, typed `openapi-fetch` seam, and byte-for-byte drift
  check implemented on `feat/dash-wave-1`.
- **Intent:** Establish the Rust API -> OpenAPI -> generated client seam.
- **Expected Outcome:** Dashboard server emits OpenAPI for Protection Overview
  and Plan Driver read endpoints. A generated TypeScript client is committed or
  reproducibly generated for `apps/dashboard/`, and validation fails on drift.
- **Files:**
  - `crates/anvil-dashboard-server/src/openapi.rs`
  - `crates/anvil-dashboard-server/src/bin/export_openapi.rs`
  - `crates/anvil-dashboard-server/tests/openapi_snapshot.rs`
  - `apps/dashboard/src/api/generated/`
  - `apps/dashboard/src/api/client.ts`
  - `apps/dashboard/scripts/generate-api.mjs`
  - `apps/dashboard/package.json`
  - `pnpm-lock.yaml`
- **Dependencies:** DASH-004, DASH-005
- **Validation:** `pnpm --dir apps/dashboard check:api` exits 0;
  `cargo test -p eddacraft-anvil-dashboard-server --test openapi_snapshot`
  passes 2 tests; generated TypeScript client passes dashboard typecheck.
- **Confidence:** medium

### DASH-007: TanStack Query resource layer

- **Status:** In Progress 2026-07-13 — stable query provider/client lifetimes,
  generated-client hooks, structured query boundary, and test fixtures
  implemented on `feat/dash-wave-1`.
- **Intent:** Wrap the generated client with stable query keys and loading/error
  boundaries.
- **Expected Outcome:** Dashboard app has query providers and hooks for
  protection overview and plan driver resources. Components consume hooks, not
  raw fetch calls.
- **Files:**
  - `apps/dashboard/src/api/query-client.tsx`
  - `apps/dashboard/src/api/query-keys.ts`
  - `apps/dashboard/src/hooks/use-protection-overview.ts`
  - `apps/dashboard/src/hooks/use-plan-driver.ts`
  - `apps/dashboard/src/components/query-boundary.tsx`
  - `apps/dashboard/src/api/fixtures.ts`
  - `apps/dashboard/src/api/query-layer.test.tsx`
  - `apps/dashboard/src/main.tsx`
- **Dependencies:** DASH-006
- **Validation:** `pnpm exec nx run dashboard:test --skip-nx-cache` passes 19
  tests including loading, success, structured error, and stable key coverage;
  dashboard typecheck, lint, and build targets exit 0.
- **Confidence:** high

### DASH-008: URL state, command palette, and deep linking

- **Status:** In Progress 2026-07-13 — Zod-validated search state, router-owned
  view/evidence filters, registered module/resource commands, and Cmd+K
  navigation implemented on `feat/dash-wave-1`. This slice also repairs the
  prior manifest's single-route shape with explicit resource-bound routes.
- **Intent:** Make dashboard navigation and filters addressable without a Next
  route dependency.
- **Expected Outcome:** TanStack Router owns route/search params with Zod
  validation. Cmd+K opens a command palette over registered modules and current
  module resources. Back/forward navigation restores filters.
- **Files:**
  - `apps/dashboard/src/router.tsx`
  - `apps/dashboard/src/lib/search-params.ts`
  - `apps/dashboard/src/lib/search-params.test.ts`
  - `apps/dashboard/src/components/command-palette.tsx`
  - `apps/dashboard/src/hooks/use-command-search.ts`
  - `apps/dashboard/src/hooks/use-command-search.test.ts`
  - `apps/dashboard/src/modules/manifest.ts`
  - `apps/dashboard/src/modules/registry.ts`
  - `apps/dashboard/src/modules/registry.test.ts`
  - `apps/dashboard/src/routes/plans.tsx`
  - `apps/dashboard/src/components/shell/dashboard-shell.tsx`
  - `apps/dashboard/src/components/shell/sidebar.tsx`
  - `apps/dashboard/src/components/shell/mobile-navigation.tsx`
  - `apps/dashboard/src/modules/protection/protection-overview.tsx`
  - `apps/dashboard/src/modules/protection/protection-tables.tsx`
- **Dependencies:** DASH-002, DASH-007
- **Validation:** Dashboard tests cover valid and invalid search fallbacks,
  explicit manifest route/resource bindings, registered command entries, and
  Cmd+K visibility for Protection and Plan Driver resources; dashboard
  typecheck, lint, and build targets exit 0.
- **Confidence:** medium

### DASH-009: Remove `apps/anvil-ui/` placeholder

- **Intent:** Clean up unused placeholder application.
- **Expected Outcome:** Delete `apps/anvil-ui/` directory. Update local agent
  file maps to reference the dedicated dashboard app.
- **Files:**
  - `apps/anvil-ui/` (delete)
  - `.claude/rules/aps-project.md`
- **Dependencies:** DASH-001
- **Validation:** `apps/anvil-ui/` no longer exists; no broken references remain
- **Confidence:** high
- **Status:** Done

### DASH-010: Protection Overview proof module

- **Status:** In Progress 2026-07-13 — typed full/partial/empty and
  offline/loading/error presentation, runs, warnings, affected files, evidence
  selection, freshness, desktop/mobile rendering, and read-only API fixtures
  implemented on `feat/dash-wave-1`; focused server, UI, typecheck, and
  Playwright evidence is green, pending branch integration.
- **Intent:** Ship the first user-facing dashboard module for Anvil's core
  protection promise.
- **Expected Outcome:** Protection Overview shows current save-time protection
  state, latest runs, active warnings by severity/category, affected files,
  evidence links, freshness, and the next attention item. It is read-only and
  planless; users do not need APS to benefit.
- **Files:**
  - `crates/anvil-dashboard-server/src/capabilities/protection.rs`
  - `crates/anvil-dashboard-server/tests/protection_overview.rs`
  - `apps/dashboard/src/modules/protection/manifest.ts`
  - `apps/dashboard/src/modules/protection/protection-overview.tsx`
  - `apps/dashboard/src/routes/protection.index.tsx`
- **Dependencies:** DASH-003, DASH-006, DASH-007
- **Validation:** API fixture renders the Protection Overview module with real
  typed data; empty state explains missing local artefacts without implying
  failure
- **Confidence:** medium

### DASH-011: Plan Driver proof module

- **Status:** In Progress 2026-07-13 — `/plans` and `/plans/$id`, typed list and
  detail hooks, readiness/evidence timeline, visibly deferred disabled actions,
  and the bounded `anvil-plan-read-model` adapter implemented on
  `feat/dash-wave-1`; focused server, UI, typecheck, and Playwright evidence is
  green, pending branch integration.
- **Intent:** Keep an internal/dogfood APS module that proves the same dashboard
  module seam over plan data.
- **Expected Outcome:** Plan Driver shows APS plan list, selected plan detail,
  run/evidence timeline, status/readiness indicators, and disabled/deferred
  approval action affordances. The UI can request no write actions in Wave 1.
- **Files:**
  - `crates/anvil-dashboard-server/src/capabilities/plans.rs`
  - `crates/anvil-dashboard-server/tests/plan_driver.rs`
  - `apps/dashboard/src/modules/plans/manifest.ts`
  - `apps/dashboard/src/modules/plans/plan-list.tsx`
  - `apps/dashboard/src/modules/plans/plan-detail.tsx`
  - `apps/dashboard/src/modules/plans/evidence-timeline.tsx`
  - `apps/dashboard/src/routes/plans.index.tsx`
  - `apps/dashboard/src/routes/plans.$id.tsx`
- **Dependencies:** DASH-003, DASH-006, DASH-007
- **Validation:** API fixture renders plan list and plan detail; disabled action
  affordances cannot mutate state
- **Confidence:** medium
