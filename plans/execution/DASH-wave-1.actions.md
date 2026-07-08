# Dashboard Wave 1 Implementation Plan

**Goal:** Implement DASH Wave 1 as a dedicated Vite dashboard host backed by a
local Rust dashboard server and two proof modules.
**Architecture:** `apps/dashboard` is a React/Vite/TanStack client. The browser
talks to `crates/anvil-dashboard-server`, which owns local artefact reads,
OpenAPI output, and kernel capability boundaries. Dashboard modules own UI
adapter concerns only; kernel/server code remains the authority for permissions,
workflow state, audit, evidence, and policy decisions.
**Tech Stack:** React 19, Vite 8, TanStack Router, TanStack Query, TanStack
Table, shadcn/ui, Tailwind v4, Zod, OpenAPI generated TypeScript client,
json-render first, Recharts as chart primitives.

---

## File Map

- `apps/dashboard/` — new dashboard client app and Nx project.
- `apps/dashboard/src/routes/` — TanStack Router routes.
- `apps/dashboard/src/modules/` — dashboard module manifests and module UI.
- `apps/dashboard/src/components/` — shell, primitives, query boundaries, and
  command palette.
- `apps/dashboard/src/api/` — generated client wrapper, query client, and query
  keys.
- `crates/anvil-dashboard-server/` — local read-only dashboard server.
- `crates/anvil-dashboard-server/src/capabilities/` — capability adapters for
  Protection Overview and Plan Driver.
- `Cargo.toml` — Cargo workspace membership for the dashboard server crate.
- `.claude/rules/aps-project.md` — remove the old `apps/anvil-ui` placeholder
  map once the new dashboard app exists.

## Tasks

### Task 1: Scaffold `apps/dashboard`

**Files:**

- Create: `apps/dashboard/package.json`
- Create: `apps/dashboard/project.json`
- Create: `apps/dashboard/index.html`
- Create: `apps/dashboard/vite.config.ts`
- Create: `apps/dashboard/tsconfig.json`
- Create: `apps/dashboard/components.json`
- Create: `apps/dashboard/src/main.tsx`
- Create: `apps/dashboard/src/routes/__root.tsx`
- Create: `apps/dashboard/src/routes/index.tsx`
- Create: `apps/dashboard/src/styles.css`

- [ ] Add failing build/typecheck target expectations with Nx project metadata.
- [ ] Run `pnpm exec nx show project dashboard` and verify the project resolves.
- [ ] Implement the minimal Vite app scaffold.
- [ ] Run `pnpm exec nx run dashboard:build` and verify it exits 0.
- [ ] Commit: `feat(dash): scaffold dashboard app`

### Task 2: Add module host and navigation shell

**Files:**

- Create: `apps/dashboard/src/modules/manifest.ts`
- Create: `apps/dashboard/src/modules/registry.ts`
- Create: `apps/dashboard/src/components/shell/dashboard-shell.tsx`
- Create: `apps/dashboard/src/components/shell/sidebar.tsx`
- Create: `apps/dashboard/src/components/shell/top-bar.tsx`
- Modify: `apps/dashboard/src/routes/__root.tsx`

- [ ] Write a manifest registry test for duplicate IDs and unknown module
      failures.
- [ ] Run the test and verify it fails.
- [ ] Implement the module manifest type, registry, shell, sidebar, and top bar.
- [ ] Run the test and `pnpm exec nx run dashboard:typecheck`.
- [ ] Commit: `feat(dash): add dashboard module host`

### Task 3: Add theme and component catalogue

**Files:**

- Create: `apps/dashboard/src/lib/theme.ts`
- Create: `apps/dashboard/src/components/primitives/metric-card.tsx`
- Create: `apps/dashboard/src/components/primitives/data-table.tsx`
- Create: `apps/dashboard/src/components/primitives/status-badge.tsx`
- Create: `apps/dashboard/src/components/primitives/severity-badge.tsx`
- Create: `apps/dashboard/src/components/primitives/code-block.tsx`
- Create: `apps/dashboard/src/components/primitives/empty-state.tsx`
- Create: `apps/dashboard/src/components/primitives/loading-skeleton.tsx`
- Create: `apps/dashboard/src/components/primitives/charts.tsx`
- Create: `apps/dashboard/src/lib/render/catalog.ts`

- [ ] Add render/catalogue tests proving known components validate and unknown
      components fail.
- [ ] Run the test and verify it fails.
- [ ] Implement primitives, Tailwind tokens, and json-render catalogue mapping.
- [ ] Run dashboard tests and typecheck.
- [ ] Commit: `feat(dash): add dashboard component catalogue`

### Task 4: Scaffold `crates/anvil-dashboard-server`

**Files:**

- Create: `crates/anvil-dashboard-server/Cargo.toml`
- Create: `crates/anvil-dashboard-server/src/lib.rs`
- Create: `crates/anvil-dashboard-server/src/main.rs`
- Create: `crates/anvil-dashboard-server/src/server.rs`
- Create: `crates/anvil-dashboard-server/src/openapi.rs`
- Create: `crates/anvil-dashboard-server/tests/server_smoke.rs`
- Modify: `Cargo.toml`

- [ ] Write a smoke test for `/healthz` and `/openapi.json`.
- [ ] Run `cargo test -p eddacraft-anvil-dashboard-server` and verify it fails.
- [ ] Implement loopback-only server startup and the two endpoints.
- [ ] Run `cargo test -p eddacraft-anvil-dashboard-server`.
- [ ] Commit: `feat(dash): add dashboard server crate`

### Task 5: Enforce workspace read containment

**Files:**

- Create: `crates/anvil-dashboard-server/src/workspace.rs`
- Create: `crates/anvil-dashboard-server/src/error.rs`
- Create: `crates/anvil-dashboard-server/tests/workspace_boundary.rs`
- Modify: `crates/anvil-dashboard-server/src/lib.rs`

- [ ] Add tests for `..`, symlink escapes, absolute outside-root paths, missing
      artefacts, and oversized artefacts.
- [ ] Run `cargo test -p eddacraft-anvil-dashboard-server workspace_boundary` and
      verify it fails.
- [ ] Implement root canonicalisation, path containment, size limits, and
      structured errors.
- [ ] Run `cargo test -p eddacraft-anvil-dashboard-server`.
- [ ] Commit: `feat(dash): constrain dashboard artefact reads`

### Task 6: Generate OpenAPI and TypeScript client

**Files:**

- Modify: `crates/anvil-dashboard-server/src/openapi.rs`
- Create: `crates/anvil-dashboard-server/tests/openapi_snapshot.rs`
- Create: `apps/dashboard/src/api/generated/`
- Create: `apps/dashboard/src/api/client.ts`
- Modify: `apps/dashboard/package.json`

- [ ] Add OpenAPI snapshot coverage for Protection Overview and Plan Driver
      endpoint groups.
- [ ] Run the snapshot test and verify it fails.
- [ ] Implement OpenAPI generation and the reproducible TS client generation
      command.
- [ ] Run OpenAPI snapshot tests and `pnpm exec nx run dashboard:typecheck`.
- [ ] Commit: `feat(dash): add dashboard OpenAPI client seam`

### Task 7: Add TanStack Query resource layer

**Files:**

- Create: `apps/dashboard/src/api/query-client.tsx`
- Create: `apps/dashboard/src/api/query-keys.ts`
- Create: `apps/dashboard/src/hooks/use-protection-overview.ts`
- Create: `apps/dashboard/src/hooks/use-plan-driver.ts`
- Create: `apps/dashboard/src/components/query-boundary.tsx`
- Modify: `apps/dashboard/src/main.tsx`

- [ ] Add hook tests for loading, success, structured error, and query-key
      stability.
- [ ] Run dashboard tests and verify they fail.
- [ ] Implement query providers, query keys, hooks, and query boundary.
- [ ] Run dashboard tests and typecheck.
- [ ] Commit: `feat(dash): add dashboard query layer`

### Task 8: Add URL state and command palette

**Files:**

- Modify: `apps/dashboard/src/routes/__root.tsx`
- Create: `apps/dashboard/src/lib/search-params.ts`
- Create: `apps/dashboard/src/components/command-palette.tsx`
- Create: `apps/dashboard/src/hooks/use-command-search.ts`

- [ ] Add tests for valid/invalid route search params and command navigation.
- [ ] Run dashboard tests and verify they fail.
- [ ] Implement Zod-validated search params and Cmd+K search over registered
      modules.
- [ ] Run dashboard tests and typecheck.
- [ ] Commit: `feat(dash): add dashboard deep linking`

### Task 9: Remove the old placeholder app

**Files:**

- Delete: `apps/anvil-ui/`
- Modify: `.claude/rules/aps-project.md`

- [ ] Search for `apps/anvil-ui`.
- [ ] Delete the placeholder and update the local agent file map.
- [ ] Run `rg "apps/anvil-ui"` and verify no active references remain.
- [ ] Commit: `chore(dash): remove dashboard placeholder app`

### Task 10: Ship Protection Overview proof module

**Files:**

- Create: `crates/anvil-dashboard-server/src/capabilities/protection.rs`
- Create: `crates/anvil-dashboard-server/tests/protection_overview.rs`
- Create: `apps/dashboard/src/modules/protection/manifest.ts`
- Create: `apps/dashboard/src/modules/protection/protection-overview.tsx`
- Create: `apps/dashboard/src/routes/protection.index.tsx`

- [ ] Add API fixture tests for present artefacts and empty-state artefacts.
- [ ] Add UI tests for the Protection Overview module.
- [ ] Run tests and verify they fail.
- [ ] Implement read-only protection state, latest runs, active warnings,
      affected files, freshness, and evidence links.
- [ ] Run `cargo test -p eddacraft-anvil-dashboard-server` and dashboard tests.
- [ ] Commit: `feat(dash): add protection overview module`

### Task 11: Ship Plan Driver proof module

**Files:**

- Create: `crates/anvil-dashboard-server/src/capabilities/plans.rs`
- Create: `crates/anvil-dashboard-server/tests/plan_driver.rs`
- Create: `apps/dashboard/src/modules/plans/manifest.ts`
- Create: `apps/dashboard/src/modules/plans/plan-list.tsx`
- Create: `apps/dashboard/src/modules/plans/plan-detail.tsx`
- Create: `apps/dashboard/src/modules/plans/evidence-timeline.tsx`
- Create: `apps/dashboard/src/routes/plans.index.tsx`
- Create: `apps/dashboard/src/routes/plans.$id.tsx`

- [ ] Add API fixture tests for plan list and detail.
- [ ] Add UI tests proving disabled action affordances cannot mutate state.
- [ ] Run tests and verify they fail.
- [ ] Implement read-only Plan Driver list, detail, evidence timeline, and
      disabled/deferred action affordances.
- [ ] Run `cargo test -p eddacraft-anvil-dashboard-server` and dashboard tests.
- [ ] Commit: `feat(dash): add plan driver module`

### Task 12: Reconcile APS and validation

**Files:**

- Modify: `plans/modules/dashboard-foundation.aps.md`
- Modify: `plans/index.aps.md`

- [ ] Mark completed DASH work items with validation evidence.
- [ ] Run `pnpm aps:active-lint`.
- [ ] Run `pnpm aps:index:check`.
- [ ] Run `pnpm validate:changed`.
- [ ] Commit: `docs(dash): record dashboard wave 1 evidence`
