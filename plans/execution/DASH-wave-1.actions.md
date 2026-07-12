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

## Approved Visual Contract

- **Approval:** Approved by the owner in the DASH-001 implementation thread;
  the desktop direction was locked first and the mobile sibling was accepted
  when the owner authorised the quick shadcn/Recharts implementation path.
- **Desktop reference:** owner-approved desktop Protection Overview concept in
  the DASH-001 implementation thread.
- **Mobile portrait reference:** owner-approved responsive sibling generated in
  the same implementation thread.
- **Evidence lock:** Protection state is the first scan target, followed by the
  latest run, freshness, next attention item, recent runs/warnings, the selected
  evidence inspector, affected files, and the read-only/local boundary. Values
  remain code-native and are never baked into an image.
- **Locked desktop structure:** fixed left module rail, compact workspace/search
  bar, open evidence canvas, status and next-attention bands, two-column
  runs/warnings plus inspector region, and a full-width affected-files table.
- **Locked mobile structure:** compact command bar, status before controls,
  Runs/Warnings switch, full-width inspector, affected-file rows, and bottom
  module navigation. The page must not horizontally scroll; code may.
- **Visual system:** near-black background and surfaces, graphite one-pixel
  dividers, restrained 2-4 px radii, monospaced UI typography, blue selection,
  green protection/clean, red high/issues, amber medium, and yellow low.
- **Accessibility contract:** essential values are visible without hover or
  colour alone; tables retain headings; selection is keyboard-reachable; mobile
  targets are at least 44 CSS px; reduced motion removes non-essential
  transitions; empty, loading, stale, partial, offline, and error states remain
  distinct.
- **Intentional flexibility:** exact breakpoints, minor spacing, final font
  fallback, and row truncation may adapt when required for legibility without
  changing the reading order or evidence hierarchy.

## Visualisation Mini-Briefs

| Visual layer | Story job | Encoding and interaction | Fallback and QA |
| --- | --- | --- | --- |
| Protection status band | Answer whether save-time protection is active now | Direct state label, shield, mode, latest result, absolute and relative freshness | Text remains complete in stale/offline states; desktop and mobile screenshot comparison |
| Latest runs | Compare recent outcomes and select evidence | Dense semantic table; result word plus colour; committed row selection drives the inspector | Mobile exposes the essential columns without page overflow; keyboard row selection test |
| Active warnings | Rank the next findings by severity and recency | Dense semantic table; severity word, marker, rule/category, path, age, evidence | No hover-only values; empty and partial fixtures; deterministic component tests |
| Evidence inspector | Explain why the selected finding exists | Direct metadata, explanation, code lines with the matched line highlighted, copy actions | Horizontally scrollable code, accessible label, stable selected evidence in offline mode |
| Affected files | Show the blast radius by file | Lookup table ordered by highest severity then recency | Compact mobile rows; text severity is redundant with colour |

Recharts is dependency-ready for future genuine time-series data. Wave 1 does
not add a decorative or fixture-invented chart to the approved proof screen.

## Delivery Boundary

Wave 1 runs as a Vite client plus a loopback-only Rust API. Vite proxies `/api`
and `/openapi.json` during development, so browser code has a same-origin
contract. Bundling the Vite assets into the shipped `anvil` binary, adding an
`anvil dashboard` public command, browser launch/lifecycle, and install/update
behaviour remain a separate architecture and public-contract checkpoint; Wave 1
must not accidentally decide those through a development-server shortcut.

## PROBE+ Capability Evidence

- Package manager: `pnpm`, proved by `pnpm-lock.yaml` and root
  `packageManager`/engine policy.
- Dashboard tests: `pnpm exec nx run dashboard:test --skip-nx-cache`, derived
  from `apps/dashboard/project.json`; baseline 1 file / 3 tests green. In the
  managed sandbox use `NX_DAEMON=false`,
  `NX_WORKSPACE_DATA_DIRECTORY=/tmp/nx-dash-workspace`, and
  `NX_CACHE_DIRECTORY=/tmp/nx-dash-cache` because Nx's native workspace DB
  cannot initialise on the default path there.
- Dashboard build/typecheck/lint: `pnpm exec nx run dashboard:build`,
  `pnpm exec nx run dashboard:typecheck`, and
  `pnpm exec nx run dashboard:lint`, derived from
  `apps/dashboard/project.json` and `apps/dashboard/package.json`.
- Rust tests/lint: `cargo test -p eddacraft-anvil-dashboard-server`,
  `cargo clippy -p eddacraft-anvil-dashboard-server --all-targets -- -D warnings`,
  and `cargo fmt --all --check`, derived from the crate manifest and workspace
  Cargo configuration once Task 4 creates the crate.
- Repository gates: `pnpm validate:changed`, `pnpm docs:check`,
  `pnpm aps:active-lint`, `pnpm aps:index:check`, `pnpm format:check`, and
  `git diff --check`, derived from root scripts and CI.
- Isolation: existing Worktrunk worktree
  `feat/dash-001-dashboard-scaffold`; the dashboard smoke suite is the
  post-isolation green proof.
- CI: `.github/workflows/ci.yml`, Rust, security, CodeQL, Council, and
  infrastructure workflows are present; draft PR #3261 had a green scaffold
  baseline before this wave continued.

## File Map

- `apps/dashboard/` — new dashboard client app and Nx project.
- `apps/dashboard/src/routes/` — TanStack Router routes.
- `apps/dashboard/src/modules/` — dashboard module manifests and module UI.
- `apps/dashboard/src/components/` — shell, primitives, query boundaries, and
  command palette.
- `apps/dashboard/src/api/` — generated client wrapper, query client, and query
  keys.
- `apps/dashboard/src/components/ui/` — shadcn source components used by the
  dashboard shell and command surfaces.
- `apps/dashboard/src/data/` — deterministic browser fixtures used only when the
  local API is unavailable in development/tests.
- `crates/anvil-dashboard-server/` — local read-only dashboard server.
- `crates/anvil-dashboard-server/src/api.rs` — versioned sealed dashboard DTOs.
- `crates/anvil-dashboard-server/src/capabilities/` — capability adapters for
  Protection Overview and Plan Driver.
- `crates/anvil-dashboard-server/tests/fixtures/` — canonical API fixtures for
  full, empty, partial, and plan data.
- `Cargo.toml` — Cargo workspace membership for the dashboard server crate.
- `plans/modules/dashboard-foundation.aps.md` — item-level status and validation
  evidence only; feature work does not rewrite aggregate counters.

## Tasks

### Task 1: Scaffold `apps/dashboard` (implemented on DASH-001 branch)

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

- [x] Add failing build/typecheck target expectations with Nx project metadata.
- [x] Run `pnpm exec nx show project dashboard` and verify the project resolves.
- [x] Implement the minimal Vite app scaffold.
- [x] Run `pnpm exec nx run dashboard:build` and verify it exits 0.
- [x] Commit: `feat(dash): scaffold dashboard app`

### Task 2: Add module host and navigation shell

**Files:**

- Create: `apps/dashboard/src/modules/manifest.ts`
- Create: `apps/dashboard/src/modules/registry.ts`
- Create: `apps/dashboard/src/components/shell/dashboard-shell.tsx`
- Create: `apps/dashboard/src/components/shell/sidebar.tsx`
- Create: `apps/dashboard/src/components/shell/top-bar.tsx`
- Create: `apps/dashboard/src/components/shell/mobile-navigation.tsx`
- Create: `apps/dashboard/src/components/shell/workspace-switcher.tsx`
- Modify: `apps/dashboard/src/routes/__root.tsx`

- [x] Write a manifest registry test for duplicate IDs and unknown module
      failures.
- [x] Run the test and verify it fails.
- [x] Implement the module manifest type, registry, shell, sidebar, and top bar.
- [x] Run the test and `pnpm exec nx run dashboard:typecheck`.
- [x] Commit in the combined DASH-002..005 foundation slice.

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
- Create: `apps/dashboard/src/lib/utils.ts`
- Create via shadcn CLI: `apps/dashboard/src/components/ui/badge.tsx`
- Create via shadcn CLI: `apps/dashboard/src/components/ui/button.tsx`
- Create via shadcn CLI: `apps/dashboard/src/components/ui/command.tsx`
- Create via shadcn CLI: `apps/dashboard/src/components/ui/dialog.tsx`
- Create via shadcn CLI: `apps/dashboard/src/components/ui/sheet.tsx`
- Create via shadcn CLI: `apps/dashboard/src/components/ui/skeleton.tsx`
- Create via shadcn CLI: `apps/dashboard/src/components/ui/table.tsx`

- [x] Add render/catalogue tests proving known components validate and unknown
      components fail.
- [x] Run the test and verify it fails.
- [x] Implement primitives, Tailwind tokens, and the thin dashboard adapter over
      the authoritative `@eddacraft/render` json-render catalogue.
- [x] Run dashboard tests and typecheck.
- [x] Commit in the combined DASH-002..005 foundation slice.

### Task 4: Scaffold `crates/anvil-dashboard-server`

**Files:**

- Create: `crates/anvil-dashboard-server/Cargo.toml`
- Create: `crates/anvil-dashboard-server/src/lib.rs`
- Create: `crates/anvil-dashboard-server/src/main.rs`
- Create: `crates/anvil-dashboard-server/src/server.rs`
- Create: `crates/anvil-dashboard-server/src/api.rs`
- Create: `crates/anvil-dashboard-server/src/openapi.rs`
- Create: `crates/anvil-dashboard-server/tests/server_smoke.rs`
- Modify: `Cargo.toml`

- [x] Write a smoke test for `/healthz` and `/openapi.json`.
- [x] Run `cargo test -p eddacraft-anvil-dashboard-server` and verify it fails.
- [x] Implement `GET /healthz`, `GET /openapi.json`,
      `GET /api/v1/protection`, `GET /api/v1/plans`, and
      `GET /api/v1/plans/{id}`. The listener must reject non-loopback bind
      addresses and the router must expose no mutating method.
- [x] Run `cargo test -p eddacraft-anvil-dashboard-server`.
- [x] Commit in the combined DASH-002..005 foundation slice.

### Task 5: Enforce workspace read containment

**Files:**

- Create: `crates/anvil-dashboard-server/src/workspace.rs`
- Create: `crates/anvil-dashboard-server/src/error.rs`
- Create: `crates/anvil-dashboard-server/tests/workspace_boundary.rs`
- Modify: `crates/anvil-dashboard-server/src/lib.rs`

- [x] Add tests for `..`, symlink escapes, absolute outside-root paths, missing
      artefacts, and oversized artefacts.
- [x] Run `cargo test -p eddacraft-anvil-dashboard-server workspace_boundary` and
      verify it fails.
- [x] Implement root canonicalisation, path containment, size limits, and
      structured errors.
- [x] Run `cargo test -p eddacraft-anvil-dashboard-server`.
- [x] Commit in the combined DASH-002..005 foundation slice.

### Task 6: Generate OpenAPI and TypeScript client

**Files:**

- Modify: `crates/anvil-dashboard-server/src/openapi.rs`
- Create: `crates/anvil-dashboard-server/tests/openapi_snapshot.rs`
- Create: `apps/dashboard/src/api/generated/`
- Create: `apps/dashboard/src/api/client.ts`
- Create: `crates/anvil-dashboard-server/src/bin/export_openapi.rs`
- Modify: `apps/dashboard/package.json`

- [x] Add OpenAPI snapshot coverage for Protection Overview and Plan Driver
      endpoint groups.
- [x] Run the snapshot test and verify it fails.
- [x] Implement deterministic OpenAPI generation plus
      `pnpm --dir apps/dashboard generate:api`; the command writes
      `src/api/generated/openapi.json` and uses `openapi-typescript` plus
      `openapi-fetch` for the typed client seam.
- [x] Run OpenAPI snapshot tests and `pnpm exec nx run dashboard:typecheck`.
- [x] Commit in the combined DASH-006..008 client-foundation slice.

### Task 7: Add TanStack Query resource layer

**Files:**

- Create: `apps/dashboard/src/api/query-client.tsx`
- Create: `apps/dashboard/src/api/query-keys.ts`
- Create: `apps/dashboard/src/hooks/use-protection-overview.ts`
- Create: `apps/dashboard/src/hooks/use-plan-driver.ts`
- Create: `apps/dashboard/src/components/query-boundary.tsx`
- Create: `apps/dashboard/src/api/fixtures.ts`
- Modify: `apps/dashboard/src/main.tsx`

- [x] Add hook tests for loading, success, structured error, and query-key
      stability.
- [x] Run dashboard tests and verify they fail.
- [x] Implement query providers, query keys, hooks, and query boundary.
- [x] Run dashboard tests and typecheck.
- [x] Commit in the combined DASH-006..008 client-foundation slice.

### Task 8: Add URL state and command palette

**Files:**

- Modify: `apps/dashboard/src/routes/__root.tsx`
- Create: `apps/dashboard/src/lib/search-params.ts`
- Create: `apps/dashboard/src/components/command-palette.tsx`
- Create: `apps/dashboard/src/hooks/use-command-search.ts`

- [x] Add tests for valid/invalid route search params and command navigation.
- [x] Run dashboard tests and verify they fail.
- [x] Implement Zod-validated search params and Cmd+K search over registered
      modules.
- [x] Run dashboard tests and typecheck.
- [x] Commit in the combined DASH-006..008 client-foundation slice.

### Task 9: Remove the old placeholder app (already done on `main`)

**Files:**

- Delete: `apps/anvil-ui/`
- Modify: `.claude/rules/aps-project.md`

- [x] Search for `apps/anvil-ui`.
- [x] Delete the placeholder and update the local agent file map.
- [x] Run `rg "apps/anvil-ui"` and verify no active references remain.
- [x] Commit: `chore(dash): remove dashboard placeholder app`

### Task 10: Ship Protection Overview proof module

**Status:** In Progress 2026-07-13 — implementation and focused browser proof
are green on `feat/dash-wave-1`; the original concept binaries are unavailable
in this execution thread, so visual fidelity was checked against the approved
textual contract and captured at `/tmp/dash-protection-desktop.png` and
`/tmp/dash-protection-mobile-390.png` pending branch integration.

**Files:**

- Create: `crates/anvil-dashboard-server/src/capabilities/protection.rs`
- Create: `crates/anvil-dashboard-server/tests/protection_overview.rs`
- Create: `apps/dashboard/src/modules/protection/manifest.ts`
- Create: `apps/dashboard/src/modules/protection/protection-overview.tsx`
- Create: `apps/dashboard/src/modules/protection/latest-runs.tsx`
- Create: `apps/dashboard/src/modules/protection/active-warnings.tsx`
- Create: `apps/dashboard/src/modules/protection/evidence-inspector.tsx`
- Create: `apps/dashboard/src/modules/protection/affected-files.tsx`
- Create: `apps/dashboard/src/routes/protection.index.tsx`

- [x] Add API fixture tests for present artefacts and empty-state artefacts.
- [x] Add UI tests for the Protection Overview module.
- [x] Run tests and verify they fail for the missing typed contract and views.
- [x] Implement read-only protection state, latest runs, active warnings,
      affected files, freshness, and evidence links.
- [ ] Match both approved concept references at their native aspect; the image
      binaries are unavailable in this thread. The accepted textual contract
      was verified at desktop and 390 px mobile portrait, including no page
      overflow, labelled full/partial/empty and offline evidence, preserved
      last-known-good evidence, and console health.
- [x] Run `CARGO_TARGET_DIR=/tmp/anvil-dash-target cargo test -p
      eddacraft-anvil-dashboard-server` (15 passed), dashboard tests (23 passed),
      dashboard typecheck, and the Playwright Protection/Plan flow (1 passed).
- [x] Commit in `feat(dash): complete proof modules`.

### Task 11: Ship Plan Driver proof module

**Status:** In Progress 2026-07-13 — typed list/detail routes, selected module
timeline, canonical read-model reuse, and inert actions are green on
`feat/dash-wave-1`; browser proof is captured at
`/tmp/dash-plan-detail-desktop.png`, pending branch integration.

**Files:**

- Create: `crates/anvil-dashboard-server/src/capabilities/plans.rs`
- Create: `crates/anvil-dashboard-server/tests/plan_driver.rs`
- Create: `apps/dashboard/src/modules/plans/manifest.ts`
- Create: `apps/dashboard/src/modules/plans/plan-list.tsx`
- Create: `apps/dashboard/src/modules/plans/plan-detail.tsx`
- Create: `apps/dashboard/src/modules/plans/evidence-timeline.tsx`
- Create: `apps/dashboard/src/routes/plans.index.tsx`
- Create: `apps/dashboard/src/routes/plans.$id.tsx`

- [x] Add API fixture tests for plan list and detail.
- [x] Add UI tests proving disabled action affordances cannot mutate state.
- [x] Run tests and verify they fail for the missing detail route and view.
- [x] Implement read-only Plan Driver list, detail, evidence timeline, and
      disabled/deferred action affordances.
- [x] Run dashboard server tests (15 passed), dashboard tests (23 passed),
      dashboard typecheck, and Playwright Cmd+K/list/detail/action proof (1
      passed).
- [x] Commit in `feat(dash): complete proof modules`.

### Task 12: Reconcile APS and validation

**Files:**

- Modify: `plans/modules/dashboard-foundation.aps.md`
- Modify: `plans/index.aps.md`

- [x] Reconcile implemented DASH work items with validation evidence while the
      unmerged items remain In Progress.
- [x] Run `pnpm aps:active-lint` (117 active plans clean).
- [x] Run `pnpm aps:index:check` (green with existing stale-count advisories).
- [ ] Run `pnpm validate:changed` — DASH-relevant checks passed, but the
      repo-wide Rust test phase cannot allocate an inotify instance for the
      existing `anvil-bench` watcher-saturation tests (`MaxFilesWatch`).
- [x] Commit: `docs(dash): record dashboard wave 1 evidence`
