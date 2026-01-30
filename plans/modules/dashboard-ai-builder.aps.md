# Dashboard AI Builder

| ID | Owner | Status |
|----|-------|--------|
| DASHAI | @eddacraft | Draft |

## Purpose

Integrate the json-render approach into the web dashboard, enabling users to
compose custom dashboard views via natural language prompts. This is the
realization of the [json-render brainstorm](../brainstorms/json-render-dashboard.md)
— AI generates constrained JSON referencing the component catalog, which React
renders progressively. The structured pages (DASHCORE, DASHARCH, DASHOPS) cover
90% of usage; the AI Builder provides escape velocity for the remaining 10%.

## In Scope

- json-render runtime integration with the component catalog
- Prompt interface with live preview panel
- Dashboard template gallery (pre-built starting points)
- Dashboard persistence (save, load, share, version)
- Component catalog registration (mapping catalog components to json-render)
- Data context binding (connecting dashboard widgets to API data)

## Out of Scope

- The component catalog itself (see DASH-004)
- The data fetching layer (see DASH-005, DASH-006)
- Structured page implementations (see DASHCORE, DASHARCH, DASHOPS)
- AI model selection or LLM infrastructure — uses external API

## Interfaces

**Depends on:**

- `dashboard-foundation` — Component catalog, data hooks, theme, routing
- `@json-render/react` — json-render library for constrained AI rendering
- External LLM API — For generating dashboard JSON from prompts

**Exposes:**

- AI Builder page at `/builder`
- Saved dashboards at `/dashboards`, `/dashboards/:id`
- Template gallery at `/builder/templates`
- `DashboardRenderer` component — Renders any saved dashboard JSON
- Dashboard persistence API — Save/load/share operations

## Decisions

**D-DASHAI-001:** LLM integration model

- **Options:** (a) Direct API call to Anthropic/OpenAI from browser,
  (b) Proxy through anvil-api with server-side key management,
  (c) User provides their own API key stored in browser
- **Recommendation:** Option (b) for security; API keys should not live in
  browser storage. Fallback to (c) for self-hosted/local deployments.
- **Status:** Open

**D-DASHAI-002:** Dashboard persistence format

- **Options:** (a) JSON files in `.anvil/dashboards/`, (b) API-backed database,
  (c) URL-encoded JSON (shareable links only, no persistence)
- **Recommendation:** Option (a) for consistency with Anvil's file-based
  storage model. JSON is the canonical format; sharing is a URL pointing to the
  stored config.
- **Status:** Open

## Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| AI generates invalid component references | high | json-render schema validation rejects unknown components at parse time |
| AI-generated dashboards are slow or broken | medium | Preview renders in sandbox; user confirms before saving |
| Prompt quality varies widely by user | medium | Template gallery provides good starting points; prompt suggestions |
| LLM latency makes builder feel sluggish | medium | Progressive/streaming rendering shows partial results immediately |

## Ready Checklist

Change status to **Ready** when:

- [ ] Purpose and scope are clear
- [ ] Dependencies identified
- [ ] At least one task defined
- [ ] LLM integration decision (D-DASHAI-001) resolved
- [ ] Dashboard persistence decision (D-DASHAI-002) resolved
- [ ] DASH-004 (component catalog) is Complete or In Progress

## Tasks

### DASHAI-001: json-render runtime integration

- **Intent:** Wire the json-render library into the dashboard application
- **Expected Outcome:** `@json-render/react` is installed and configured. A
  `DashboardRenderer` component accepts a JSON configuration and renders the
  corresponding component tree. Schema validation rejects invalid JSON before
  rendering. Error boundaries catch rendering failures gracefully.
- **Scope:** `apps/anvil-ui/src/lib/`, `apps/anvil-ui/src/components/`
- **Non-scope:** Prompt interface, persistence
- **Dependencies:** DASH-004
- **Validation:** Passing valid JSON renders expected component tree; invalid JSON shows validation error
- **Confidence:** medium

### DASHAI-002: Component catalog registration

- **Intent:** Map the shared component catalog to json-render's component registry
- **Expected Outcome:** All catalog components (layout, data display, charts,
  interactive, Anvil-specific) are registered with json-render by name and prop
  schema. json-render can instantiate any catalog component from a JSON
  descriptor. Type safety is maintained — prop schemas are derived from
  component TypeScript types.
- **Scope:** `apps/anvil-ui/src/lib/`
- **Non-scope:** New component creation
- **Files:**
  - `apps/anvil-ui/src/lib/catalog-registry.ts`
- **Dependencies:** DASHAI-001, DASH-004
- **Validation:** Every catalog component is instantiable via json-render JSON descriptor
- **Confidence:** high

### DASHAI-003: Prompt interface with live preview

- **Intent:** Let users describe dashboards in natural language and see results
- **Expected Outcome:** Split-panel page: left panel with multi-line prompt
  input and "Generate" button, right panel with live preview of the generated
  dashboard. Generation streams progressively — partial results render as JSON
  arrives. Prompt history is preserved in session. Error states show clear
  messages when generation fails or produces invalid output.
- **Scope:** `apps/anvil-ui/src/pages/builder/`
- **Non-scope:** Template gallery, persistence
- **Dependencies:** DASHAI-001, DASHAI-002, D-DASHAI-001 (LLM decision)
- **Validation:** Typing a prompt and clicking Generate produces a rendered dashboard; streaming works
- **Confidence:** medium

### DASHAI-004: Dashboard template gallery

- **Intent:** Provide pre-built dashboard templates as starting points
- **Expected Outcome:** Gallery page showing 6+ templates: Team Health, Code
  Quality Deep Dive, Architecture Compliance, Suppression Audit, CI Pipeline
  Monitor, AI Tool Impact. Each template shows a preview thumbnail and
  description. Clicking a template loads it into the builder for customisation.
  Templates are stored as JSON configurations in the codebase.
- **Scope:** `apps/anvil-ui/src/pages/builder/`
- **Non-scope:** User-created templates
- **Files:**
  - `apps/anvil-ui/src/data/templates/`
- **Dependencies:** DASHAI-001, DASHAI-002
- **Validation:** Gallery renders all templates; clicking loads into builder; templates render correctly
- **Confidence:** high

### DASHAI-005: Dashboard persistence

- **Intent:** Allow users to save, load, and share custom dashboards
- **Expected Outcome:** Save button stores dashboard JSON configuration with
  a name and optional description. Saved dashboards appear in the sidebar
  navigation under "Saved Dashboards". Each saved dashboard has a shareable
  URL. Dashboards can be renamed, duplicated, and deleted. Loading a saved
  dashboard renders it via the DashboardRenderer.
- **Scope:** `apps/anvil-ui/src/pages/dashboards/`, API persistence layer
- **Non-scope:** Version history, collaborative editing
- **Dependencies:** DASHAI-001, DASH-005, D-DASHAI-002 (persistence decision)
- **Validation:** Save → reload → renders identically; share URL works; CRUD operations work
- **Confidence:** medium

### DASHAI-006: Dashboard versioning and iteration history

- **Intent:** Track how a dashboard evolves through prompt iterations
- **Expected Outcome:** Each saved dashboard maintains a version history of
  its JSON configurations. Users can view previous versions, compare changes,
  and revert to any version. Prompt text is stored alongside each version for
  context. Version list shows timestamp, prompt snippet, and change summary.
- **Scope:** `apps/anvil-ui/src/pages/dashboards/`
- **Non-scope:** Collaborative editing, branching
- **Dependencies:** DASHAI-005
- **Validation:** Iterating on a dashboard creates version history; revert restores previous state
- **Confidence:** medium

## Execution

Steps: [../execution/DASHAI.steps.md](../execution/DASHAI.steps.md)
