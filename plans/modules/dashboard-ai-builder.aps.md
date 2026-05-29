# Dashboard AI Builder

| ID     | Owner      | Status | Progress |
| ------ | ---------- | ------ | -------- |
| DASHAI | @eddacraft | Draft  | 0/6      |

**Last reviewed:** 2026-04-26

## Purpose

Integrate the json-render approach into the web dashboard, enabling users to
compose custom dashboard views via natural language prompts. This is the
realisation of the [json-render brainstorm](../brainstorms/json-render-dashboard.md)
— AI generates constrained JSON referencing the component catalogue, which React
renders progressively. The structured pages (DASHCORE, DASHARCH, DASHOPS) cover
90% of usage; the AI Builder provides escape velocity for the remaining 10%.

## In Scope

- json-render runtime integration with the component catalogue
- Prompt interface with live preview panel
- Dashboard template gallery (pre-built starting points)
- Dashboard persistence (save, load, share, version)
- Component catalogue registration (mapping catalogue components to json-render)
- Data context binding (connecting dashboard widgets to API data)

## Out of Scope

- The component catalogue itself (see DASH-003)
- The data fetching layer (see DASH-005, DASH-006)
- Structured page implementations (see DASHCORE, DASHARCH, DASHOPS)
- AI model selection or LLM infrastructure — uses external API

## Interfaces

**Depends on:**

- `dashboard-foundation` — Component catalogue, data hooks, theme, routing
- `@json-render/react` — json-render library for constrained AI rendering
- External LLM API — For generating dashboard JSON from prompts
- Coordinates with `tui-dashboard-render` (TUIDASH) — the Rust/Ratatui json-render
  spec interpreter; both modules consume the same dashboard JSON schema where
  feasible (per ADR-011 Ratatui is the TUI surface)

**Exposes:**

- AI Builder page at `/dashboard/builder`
- Saved dashboards at `/dashboard/dashboards`, `/dashboard/dashboards/:id`
- Template gallery at `/dashboard/builder/templates`
- `DashboardRenderer` component — Renders any saved dashboard JSON
- Dashboard persistence to `.anvil/dashboards/`

## Decisions

**D-DASHAI-001:** LLM integration model

- **Options:** (a) Direct API call from browser, (b) Proxy through API routes
  with server-side key management, (c) User provides own API key in browser
- **Resolution:** Deferred to Wave 4. Will resolve when implementation begins.
- **Status:** Open (deferred)

**D-DASHAI-002:** Dashboard persistence format

- **Options:** (a) JSON files in `.anvil/dashboards/`, (b) API-backed database,
  (c) URL-encoded JSON
- **Resolution:** Option (a) — `.anvil/dashboards/` JSON files. Consistent with
  Anvil's file-based storage model.
- **Status:** Resolved

## Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| AI generates invalid component references | high | json-render schema validation rejects unknown components at parse time |
| AI-generated dashboards are slow or broken | medium | Preview renders in sandbox; user confirms before saving |
| Prompt quality varies widely by user | medium | Template gallery provides good starting points; prompt suggestions |
| LLM latency makes builder feel sluggish | medium | Progressive/streaming rendering shows partial results immediately |

## Ready Checklist

Change status to **Ready** when:

- [x] Purpose and scope are clear
- [x] Dependencies identified
- [x] At least one task defined
- [ ] LLM integration decision (D-DASHAI-001) resolved — deferred to Wave 4
- [x] Dashboard persistence decision (D-DASHAI-002) resolved
- [x] DASH foundation tasks are Ready

## Wave

**Wave 4** — Final wave. Begins after Wave 2 (DASHCORE, DASHARCH) ships.

## Work Items

### DASHAI-001: json-render runtime integration

- **Intent:** Wire the json-render library into the dashboard application
- **Expected Outcome:** `@json-render/react` installed. `DashboardRenderer`
  component renders JSON config to component tree with schema validation and
  error boundaries.
- **Files:**
  - `apps/website/lib/json-render/dashboard-renderer.tsx`
  - `apps/website/lib/json-render/schema-validator.ts`
- **Dependencies:** DASH-003, DASH-004
- **Validation:** Valid JSON renders expected component tree; invalid JSON shows
  validation error
- **Confidence:** medium

### DASHAI-002: Component catalogue registration

- **Intent:** Map the shared component catalogue to json-render's component
  registry
- **Expected Outcome:** All catalogue + domain components registered with
  json-render by name and prop schema. Auto-generated manifest.
- **Files:**
  - `apps/website/lib/json-render/catalog-registry.ts`
- **Dependencies:** DASHAI-001, DASH-003
- **Validation:** Every catalogue component is instantiable via json-render JSON
  descriptor
- **Confidence:** high

### DASHAI-003: Prompt interface with live preview

- **Intent:** Let users describe dashboards in natural language and see results
- **Expected Outcome:** Split-panel: prompt input + generate button → live
  preview via DashboardRenderer. Progressive streaming. Prompt history. Error
  states.
- **Files:**
  - `apps/website/app/(dashboard)/dashboard/builder/page.tsx`
  - `apps/website/components/dashboard/builder/prompt-panel.tsx`
  - `apps/website/components/dashboard/builder/preview-panel.tsx`
- **Dependencies:** DASHAI-001, DASHAI-002
- **Validation:** Typing a prompt and clicking Generate produces a rendered
  dashboard; streaming works
- **Confidence:** medium

### DASHAI-004: Dashboard template gallery

- **Intent:** Provide pre-built dashboard templates as starting points
- **Expected Outcome:** 6+ templates (Team Health, Code Quality, Architecture
  Compliance, Suppression Audit, CI Pipeline, AI Tool Impact). Gallery with
  previews. Click loads into builder.
- **Files:**
  - `apps/website/app/(dashboard)/dashboard/builder/templates/page.tsx`
  - `apps/website/components/dashboard/builder/template-gallery.tsx`
  - `apps/website/data/dashboard-templates/`
- **Dependencies:** DASHAI-001, DASHAI-002
- **Validation:** Gallery renders all templates; clicking loads into builder;
  templates render correctly
- **Confidence:** high

### DASHAI-005: Dashboard persistence

- **Intent:** Allow users to save, load, and share custom dashboards
- **Expected Outcome:** Save/load/share dashboard JSON to
  `.anvil/dashboards/`. Sidebar listing. Shareable URLs. CRUD operations.
- **Files:**
  - `apps/website/app/(dashboard)/dashboard/dashboards/page.tsx`
  - `apps/website/app/(dashboard)/dashboard/dashboards/[id]/page.tsx`
  - `apps/website/app/api/anvil/dashboards/route.ts`
  - `apps/website/app/api/anvil/dashboards/[id]/route.ts`
- **Dependencies:** DASHAI-001, DASH-005
- **Validation:** Save → reload → renders identically; share URL works; CRUD
  operations work
- **Confidence:** medium

### DASHAI-006: Dashboard versioning

- **Intent:** Track how a dashboard evolves through prompt iterations
- **Expected Outcome:** Version history per dashboard in
  `.anvil/dashboards/[name]/versions/`. View, compare, revert. Prompt text
  stored per version.
- **Files:**
  - `apps/website/components/dashboard/builder/version-history.tsx`
  - `apps/website/app/api/anvil/dashboards/[id]/versions/route.ts`
- **Dependencies:** DASHAI-005
- **Validation:** Iterating on a dashboard creates version history; revert
  restores previous state
- **Confidence:** medium
