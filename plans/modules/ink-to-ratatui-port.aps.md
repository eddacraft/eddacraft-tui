<!--
APS Module: Ink-to-Ratatui Port
=================================
Systematic port of existing Ink TUI surfaces and shared components
to Ratatui, using the eddacraft-tui shared crate (RATS-001).

Scopes: PORT (main)
-->

# Ink-to-Ratatui Port

| ID   | Owner | Status   |
| ---- | ----- | -------- |
| PORT | —     | Complete |

## Purpose

Systematically port all existing Ink (React-based) TUI surfaces and shared
components to Ratatui, producing feature-equivalent Rust implementations that
use the `eddacraft-tui` shared crate.

**Why:** The Ink surfaces are the user-facing TUI today. RATS defines *new*
surfaces (watch dashboard, gate viewer, onboarding wizard) but does not cover
the 1:1 port of existing Ink components and command surfaces. Without this
module, the migration path (RATS-005) has nothing to migrate *from* — the Ink
surfaces would remain as the only implementation for welcome, doctor, status,
audit, template browser, and tutorial flows.

**Why now:** RATS-001 (shared component library) is complete. Most Ink surfaces
are purely presentational — they receive data as props and render it. These can
be ported immediately with mock data, without waiting for kernel events (KERN
Phase 3). Only the watch dashboard and gate explorer need live kernel events,
and those are already covered by RATS-002/RATS-003.

**Spec:** [Architecture Evolution — H2 Surfaces](../../docs/architecture/anvil-architecture-evolution.md)

## In Scope

- Port of 15 shared Ink components to Ratatui equivalents in `eddacraft-tui`
- Port of 9 Ink command surfaces to Ratatui (welcome, doctor, status, init,
  audit, new, gate, watch, tutorial)
- Visual parity validation (side-by-side comparison with Ink originals)
- Keyboard navigation parity (same keybindings, same flow)
- Service interface adaptation (TypeScript service props → Rust trait boundaries)

## Out of Scope

- Kernel event integration (see KERN module — surfaces use mock data until
  kernel events are ready)
- New TUI features beyond current Ink parity (see RATS module for new surfaces)
- The `--tui=ink`/`--tui=ratatui` switching mechanism (see RATS-005)
- Web dashboard (see 0.2.0 milestone)
- VS Code extension TUI (separate surface)

## Interfaces

**Depends on:**

- RATS-001 — `eddacraft-tui` shared crate (theme, keyboard, widgets) — **Done**
  _(implemented in external Rust workspace; will be vendored into this monorepo
  with KERN Phase 1)_
- `apps/anvil-cli/src/tui/` — existing Ink surfaces as reference implementations
- `apps/anvil-cli/src/services/` — service interfaces consumed by audit and
  template browser surfaces

**Exposes:**

- `crates/anvil-tui/src/surfaces/` — Ratatui implementations of all Ink surfaces
- Shared components added to `eddacraft-tui` as needed (Confirm, Divider,
  Header, Spinner, StatusBadge, etc.)

## Constraints

- Visual parity with Ink originals (layout, colour, information density)
- Keyboard conventions from RATS: j/k navigation, space/enter select, esc back,
  q quit
- Must support terminal sizes down to 80x24
- Theme follows EddaCraft design system (dark-only, 5-colour palette)
- Surfaces must accept data as trait objects or generic parameters — no coupling
  to specific service implementations
- Tutorial port must preserve all 4 tutorial paths with their step sequences

## Ready Checklist

Change status to **Ready** when:

- [x] RATS-001 (eddacraft-tui shared crate) is complete
- [x] EddaCraft theme and keyboard conventions documented
- [ ] Ink surface inventory confirmed (this module documents it)
- [ ] Port ordering agreed (proposed: shared components → simple → medium →
      complex)

---

## Ink Surface Inventory

### Shared Components (`apps/anvil-cli/src/tui/components/`)

15 components to evaluate for porting:

| Component | Type | RATS-001 Equivalent | Port Needed |
| --------- | ---- | ------------------- | ----------- |
| Select | Input | `eddacraft-tui::widgets::Select` | Done (RATS-001) |
| TextInput | Input | `eddacraft-tui::widgets::TextInput` | Done (RATS-001) |
| ProgressBar | Feedback | `eddacraft-tui::widgets::ProgressBar` | Done (RATS-001) |
| Spinner | Feedback | — | Yes |
| StatusBadge | Display | `eddacraft-tui::widgets::StatusBar` (partial) | Yes |
| Header | Layout | — | Yes |
| Container | Layout | — | Yes |
| Divider | Layout | — | Yes |
| Confirm | Input | — | Yes |
| ErrorBoundary | Utility | — | Rust equivalent (Result handling) |
| LogPanel/ | Display | — | Yes |
| MermaidDiagram | Display | — | Deferred (complex rendering) |
| ParallelProgress/ | Display | — | Yes |
| QuickWinsPanel | Display | — | Yes |
| ResultsDashboard | Display | — | Yes |

### Command Surfaces (`apps/anvil-cli/src/tui/commands/`)

9 surfaces ordered by complexity:

| Surface | Main Component | Service Dependencies | Complexity |
| ------- | -------------- | -------------------- | ---------- |
| welcome/ | Welcome.tsx | None (static content) | Low |
| doctor/ | Diagnostics.tsx | Props only (`DiagnosticCheck[]`) | Low-Medium |
| status/ | StatusDashboard.tsx | Props only (`StatusData`) | Medium |
| init/ | InitWizard.tsx | Multi-step wizard (5 steps) | Medium |
| audit/ | AuditResults.tsx | `services/repo-scanner` | Medium |
| new/ | TemplateBrowser.tsx | `services/template-loader` | Medium |
| gate/ | GateExplorer.tsx | Props only (`GateResult`) | Medium-High |
| watch/ | WatchDashboard.tsx | Props + imperative handle API | High |
| tutorial/ | Tutorial.tsx | 4 paths, ~23 step components | High |

---

## Phase 1 — Shared Component Ports

### PORT-001: Port shared layout and display components to eddacraft-tui

- **Status:** Done
- **Completed:** 2026-03-16
- **Intent:** Port Header, Container, Divider, Spinner, StatusBadge, and Confirm
  components to Ratatui equivalents in the `eddacraft-tui` crate
- **Expected Outcome:** Shared component library covers all layout and feedback
  primitives needed by surface ports
- **Validation:** Components render correctly in isolation tests; visual parity
  with Ink originals
- **Files:** `crates/eddacraft-tui/src/widgets/`
- **Confidence:** high
- **Priority:** High
- **Dependencies:** RATS-001

---

### PORT-002: Port LogPanel, ParallelProgress, QuickWinsPanel, ResultsDashboard

- **Status:** Done
- **Completed:** 2026-03-16
- **Intent:** Port the composite display components that combine multiple
  primitives into reusable panels
- **Expected Outcome:** Panel components render structured data (logs, progress
  bars, quick wins, results) with the same information density as Ink originals
- **Validation:** Components render correctly with mock data; keyboard scrolling
  works for LogPanel
- **Files:** `crates/eddacraft-tui/src/widgets/`
- **Confidence:** high
- **Priority:** High
- **Dependencies:** PORT-001

---

## Phase 2 — Simple Surface Ports

### PORT-010: Port welcome surface

- **Status:** Done
- **Completed:** 2026-03-16
- **Intent:** Port the first-run welcome screen to Ratatui — static content,
  no service dependencies
- **Expected Outcome:** Welcome screen renders with EddaCraft branding, value
  proposition text, and quick-start guidance
- **Validation:** Visual parity with Ink welcome; keyboard dismiss works
- **Files:** `crates/anvil-tui/src/surfaces/welcome/`
- **Confidence:** high
- **Priority:** Medium
- **Dependencies:** PORT-001

---

### PORT-011: Port doctor surface

- **Status:** Done
- **Completed:** 2026-03-16
- **Intent:** Port the diagnostics screen to Ratatui — receives diagnostic
  check results as input, renders pass/fail status with details
- **Expected Outcome:** Doctor surface displays diagnostic checks with status
  badges, expandable details, and auto-fix suggestions
- **Validation:** Visual parity with Ink doctor; keyboard navigation between
  checks works
- **Files:** `crates/anvil-tui/src/surfaces/doctor/`
- **Confidence:** high
- **Priority:** Medium
- **Dependencies:** PORT-001

---

### PORT-012: Port status dashboard surface

- **Status:** Done
- **Completed:** 2026-03-16
- **Intent:** Port the status dashboard to Ratatui — three-panel layout (hooks,
  profile, results) receiving `StatusData` as input
- **Expected Outcome:** Status dashboard renders all three panels with correct
  layout and data display
- **Validation:** Visual parity with Ink status; panel focus switching works
- **Files:** `crates/anvil-tui/src/surfaces/status/`
- **Confidence:** high
- **Priority:** Medium
- **Dependencies:** PORT-001

---

## Phase 3 — Medium Surface Ports

### PORT-020: Port init wizard surface

- **Status:** Done
- **Completed:** 2026-03-16
- **Intent:** Port the multi-step init wizard (Mode, Format, Directory, Checks,
  Summary) to Ratatui with step navigation and configuration output
- **Expected Outcome:** Wizard flow with forward/back navigation, input
  validation, and configuration generation equivalent to Ink wizard
- **Validation:** Complete wizard flow produces valid configuration; back
  navigation preserves state
- **Files:** `crates/anvil-tui/src/surfaces/init/`
- **Confidence:** medium
- **Priority:** Medium
- **Dependencies:** PORT-001

---

### PORT-021: Port audit results surface

- **Status:** Done
- **Completed:** 2026-03-16
- **Intent:** Port the audit results viewer to Ratatui — displays repo scan
  results with grouping and detail expansion
- **Expected Outcome:** Audit surface renders scan results with the same
  grouping and detail levels as the Ink original
- **Validation:** Visual parity; keyboard expand/collapse works; large result
  sets scroll smoothly
- **Files:** `crates/anvil-tui/src/surfaces/audit/`
- **Confidence:** high
- **Priority:** Medium
- **Dependencies:** PORT-002

---

### PORT-022: Port template browser surface

- **Status:** Done
- **Completed:** 2026-03-16
- **Intent:** Port the template browser to Ratatui — list view with preview
  pane, search/filter, and template selection
- **Expected Outcome:** Template browser renders template list with metadata
  and preview, supporting search and selection
- **Validation:** Visual parity; search filters correctly; template selection
  triggers output
- **Files:** `crates/anvil-tui/src/surfaces/new/`
- **Confidence:** high
- **Priority:** Low
- **Dependencies:** PORT-001

---

### PORT-023: Port gate explorer surface

- **Status:** Done
- **Completed:** 2026-03-16
- **Intent:** Port the gate explorer to Ratatui — three-panel layout (check
  tree, detail, filter) with navigable violation list
- **Expected Outcome:** Gate explorer renders check results with tree
  navigation, detail panes, and filter controls
- **Validation:** Visual parity; tree expand/collapse works; filter narrows
  results correctly
- **Files:** `crates/anvil-tui/src/surfaces/gate/`
- **Confidence:** medium
- **Priority:** Medium
- **Dependencies:** PORT-002
- **Tags:** overlaps with RATS-003 (gate result viewer) — coordinate scope

---

## Phase 4 — Complex Surface Ports

### PORT-030: Port watch dashboard surface

- **Status:** Done
- **Completed:** 2026-03-16
- **Intent:** Port the watch dashboard to Ratatui — four-panel layout with
  live updates, imperative handle API for external control
- **Expected Outcome:** Watch dashboard renders file status, gate results,
  warnings, and log output with the same layout as the Ink original
- **Validation:** Visual parity; panel focus switching works; mock event stream
  updates render smoothly
- **Files:** `crates/anvil-tui/src/surfaces/watch/`
- **Confidence:** medium
- **Priority:** High
- **Dependencies:** PORT-002
- **Tags:** overlaps with RATS-002 (watch dashboard) — this task ports the Ink
  layout; RATS-002 adds live kernel event integration

---

### PORT-040: Port tutorial orchestrator and picker

- **Status:** Done
- **Completed:** 2026-03-16
- **Intent:** Port the tutorial orchestrator (Tutorial.tsx, TutorialPicker.tsx)
  and core step components (ScanStep, WatchStep, FixStep, NextStepsStep) to
  Ratatui
- **Expected Outcome:** Tutorial picker and core flow work with step
  progression, progress tracking, and path selection
- **Validation:** Tutorial picker displays all paths; core steps progress
  correctly; completion triggers next-steps
- **Files:** `crates/anvil-tui/src/surfaces/tutorial/`
- **Confidence:** medium
- **Priority:** Low
- **Dependencies:** PORT-001

---

### PORT-041: Port policy tutorial path (6 steps)

- **Status:** Done
- **Completed:** 2026-03-16
- **Intent:** Port the policy tutorial path (CreateDir, Customise, Intro,
  SeePolicyFire, TestPolicy, WritePolicy) to Ratatui
- **Expected Outcome:** All 6 policy tutorial steps render and progress
  correctly
- **Validation:** Complete policy tutorial path end-to-end
- **Files:** `crates/anvil-tui/src/surfaces/tutorial/policy/`
- **Confidence:** medium
- **Priority:** Low
- **Dependencies:** PORT-040

---

### PORT-042: Port architecture tutorial path (6 steps)

- **Status:** Done
- **Completed:** 2026-03-16
- **Intent:** Port the architecture tutorial path (Compile, Detect, Intro,
  Summary, Template, Validate) to Ratatui
- **Expected Outcome:** All 6 architecture tutorial steps render and progress
  correctly
- **Validation:** Complete architecture tutorial path end-to-end
- **Files:** `crates/anvil-tui/src/surfaces/tutorial/architecture/`
- **Confidence:** medium
- **Priority:** Low
- **Dependencies:** PORT-040

---

### PORT-043: Port drift tutorial path (5 steps)

- **Status:** Done
- **Completed:** 2026-03-16
- **Intent:** Port the drift tutorial path (Capture, Compare, Inspect, Intro,
  Summary) to Ratatui
- **Expected Outcome:** All 5 drift tutorial steps render and progress correctly
- **Validation:** Complete drift tutorial path end-to-end
- **Files:** `crates/anvil-tui/src/surfaces/tutorial/drift/`
- **Confidence:** medium
- **Priority:** Low
- **Dependencies:** PORT-040

---

### PORT-044: Port CI tutorial path (6 steps)

- **Status:** Done
- **Completed:** 2026-03-16
- **Intent:** Port the CI tutorial path (Detect, ExitCodes, Hooks, Intro,
  Summary, Workflow) to Ratatui
- **Expected Outcome:** All 6 CI tutorial steps render and progress correctly
- **Validation:** Complete CI tutorial path end-to-end
- **Files:** `crates/anvil-tui/src/surfaces/tutorial/ci/`
- **Confidence:** medium
- **Priority:** Low
- **Dependencies:** PORT-040

---

## Coordination with RATS

Several PORT tasks overlap with RATS tasks. The boundary:

| PORT Task | RATS Task | PORT Scope | RATS Scope |
| --------- | --------- | ---------- | ---------- |
| PORT-023 | RATS-003 | Port Ink gate layout with mock data | Wire live kernel events into PORT-023 layout |
| PORT-030 | RATS-002 | Port Ink watch layout with mock data | Wire live kernel events into PORT-030 layout |

PORT tasks produce Ratatui surfaces that accept mock/static data. RATS tasks
wire those surfaces to live kernel events and extend them with new capabilities.

> **Note:** PORT-020 (init wizard port) and RATS-004 (APS onboarding wizard) are
> independent surfaces — PORT-020 ports the existing `anvil init` flow for Ink
> parity, while RATS-004 is a new APS-specific onboarding experience.

## Risks

| Risk | Likelihood | Impact | Mitigation |
| ---- | ---------- | ------ | ---------- |
| Ink layout patterns don't map to Ratatui | Medium | Medium | Build custom layout widgets in eddacraft-tui |
| Tutorial complexity explosion | Medium | High | Port orchestrator first, then paths incrementally |
| Service interface mismatch (TS → Rust) | Medium | Medium | Define Rust trait boundaries early, use mock implementations |
| MermaidDiagram port infeasible | High | Low | Defer to research phase; ASCII fallback |
| Visual parity subjective | Low | Low | Screenshot comparison, user feedback |

## Stats

| Phase | Items | Status |
| ----- | ----- | ------ |
| 1 — Shared Component Ports | 2 | Done |
| 2 — Simple Surface Ports | 3 | Done |
| 3 — Medium Surface Ports | 4 | Done |
| 4 — Complex Surface Ports | 6 | Done |
| **Total** | **15** | **Complete** |
