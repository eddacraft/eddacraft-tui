<!--
APS Module: Ratatui TUI
=========================
Rust TUI surfaces: watch dashboard, gate viewer, onboarding wizard.
Built on Ratatui, consuming kernel events.

Scopes: RATS (main)
-->

# Ratatui TUI

| ID   | Owner | Status |
| ---- | ----- | ------ |
| RATS | —     | Proposed |

## Purpose

Build Rust-native TUI surfaces using Ratatui that consume kernel events and
provide interactive terminal interfaces for Anvil.

**Why:** The current Ink-based TUI is coupled to the Node.js runtime. A Ratatui
TUI running in the same process as the Rust kernel eliminates IPC overhead and
provides a unified Rust binary. Ratatui also enables richer terminal UX (diagrams,
navigation, interactive widgets) that Ink cannot match.

**Spec:** [Architecture Evolution — H2 Surfaces](../../docs/architecture/anvil-architecture-evolution.md)

## In Scope

- Shared TUI crate (`eddacraft-tui`) with theme, keyboard conventions, widgets
- Watch dashboard (live gate results, file status, warning list)
- Gate result viewer (interactive, navigable)
- APS onboarding wizard (multi-step, template selection)

## Out of Scope

- Kernel implementation (see KERN module)
- Existing check ports (see RENG module)
- VS Code extension (separate surface, protocol client)
- Web dashboard (0.2.0 milestone, browser-based)
- Diagram rendering engine (research phase — see docs/research/)

## Interfaces

**Depends on:**

- KERN — consumes kernel events (Progress, Snapshot, Violation, Error)
- `anvil-kernel-types` — event types, graph node types
- `.anvil/` config — for onboarding wizard template selection

**Exposes:**

- `eddacraft-tui` — shared crate for EddaCraft product family TUI components
- TUI binary mode integrated into `anvil` binary (subcommand or default for
  watch)

## Constraints

- TUI render must not block kernel event processing
- Keyboard conventions: j/k navigation, space/enter select, esc back, q quit
- Must support terminal sizes down to 80x24
- Theme follows EddaCraft design system (dark-only, 5-colour palette)
- Depends on KERN Phase 3 (event emission) before meaningful integration

## Ready Checklist

Change status to **Ready** when:

- [ ] KERN module Phase 3 (event emission) is complete
- [x] Ratatui component library sufficiency validated (from KERN spike or
      separate prototype)
- [x] EddaCraft theme and keyboard conventions documented
- [ ] Watch dashboard wireframe approved

---

## Phase 1 — Shared Components

### RATS-001: eddacraft-tui shared crate (theme, keyboard, widgets)

- **Status:** Done
- **Intent:** Create a shared Ratatui component library with EddaCraft theme,
  keyboard conventions (j/k, space, enter, esc), and reusable widgets (Select,
  MultiSelect, TextInput, ProgressBar, StatusBar)
- **Expected Outcome:** Themed widget library usable by all EddaCraft products
  (Anvil, APS, Kindling)
- **Validation:** Visual parity with current Ink components, keyboard navigation
  works consistently across all widgets
- **Files:** `crates/eddacraft-tui/` _(implemented in external Rust workspace;
  will be vendored into this monorepo with KERN Phase 1)_
- **Confidence:** medium (Ratatui widget ecosystem is maturing)
- **Priority:** High
- **Dependencies:** None (can start with mock data before KERN events are ready;
  this task IS the component library referenced in the Ready Checklist)

---

## Phase 2 — Core Surfaces

### RATS-002: Watch dashboard (live gate results, file status)

- **Status:** Draft
- **Intent:** Build a Ratatui watch dashboard that renders live gate results,
  file change status, and violation list. Consumes kernel events via in-process
  channel.
- **Expected Outcome:** Dashboard shows real-time updates as files change,
  violations appear/disappear, and progress events stream
- **Validation:** Visual comparison with current Ink watch dashboard, render
  latency <5ms per frame
- **Files:** `crates/anvil-tui/src/dashboard/`
- **Confidence:** medium
- **Priority:** High
- **Dependencies:** RATS-001, PORT-030 (watch layout), KERN-033 (event emission)

---

### RATS-003: Gate result viewer (interactive)

- **Status:** Draft
- **Intent:** Build an interactive gate result viewer with navigable violation
  list, detail panes, and keyboard shortcuts for explaining/suppressing
  individual violations
- **Expected Outcome:** Users can browse violations, see file context, and take
  action without leaving the terminal
- **Validation:** Navigate 50+ violations with keyboard, verify responsiveness
  and correctness
- **Files:** `crates/anvil-tui/src/gate_view/`
- **Confidence:** medium
- **Priority:** Medium
- **Dependencies:** RATS-001, PORT-023 (gate layout), KERN-040 (embedded mode)

---

### RATS-004: APS onboarding wizard

- **Status:** Draft
- **Intent:** Build a multi-step onboarding wizard for APS project initialisation
  using Ratatui. Covers template selection, configuration, and scaffold
  generation.
- **Expected Outcome:** Multi-step wizard flow with equivalent UX to current
  `anvil init` Ink wizard
- **Validation:** Functional parity with current `anvil init` wizard, keyboard
  navigation works for all steps
- **Files:** `crates/anvil-tui/src/wizard/`
- **Confidence:** medium
- **Priority:** Low
- **Dependencies:** RATS-001

---

## Phase 3 — Integration

### RATS-005: Ink-to-Ratatui migration path

- **Status:** Draft
- **Intent:** Define and implement the deprecation/migration path from the
  existing Ink TUI to Ratatui surfaces, including feature flags to switch
  between Ink and Ratatui rendering for each surface
- **Expected Outcome:** Users can opt into Ratatui surfaces per-command via
  config or flag; Ink surfaces remain default until parity is validated
- **Validation:** Both `--tui=ink` and `--tui=ratatui` flags work for watch
  command; Ink remains default
- **Files:** `apps/anvil-cli/src/`, `crates/anvil-tui/`
- **Confidence:** medium (migration sequencing depends on surface parity)
- **Priority:** High
- **Dependencies:** RATS-002, PORT-023, PORT-030

---

### RATS-006: Terminal platform compatibility testing

- **Status:** Draft
- **Intent:** Validate Ratatui TUI surfaces render correctly across target
  terminals (iTerm2, WezTerm, GNOME Terminal, Windows Terminal, VS Code
  integrated terminal) and minimum terminal size (80×24)
- **Expected Outcome:** Compatibility matrix documenting rendering behaviour
  across terminals, with fixes for any identified rendering issues
- **Validation:** Manual or automated screenshot comparison across terminals;
  80×24 layout does not clip or overflow
- **Files:** `crates/eddacraft-tui/`, `crates/anvil-tui/`
- **Confidence:** medium
- **Priority:** Medium
- **Dependencies:** RATS-001, RATS-002

---

### RATS-007: `anvil watch` TUI integration entry point

- **Status:** Draft
- **Intent:** Wire the Ratatui watch dashboard into the `anvil` binary so that
  `anvil watch` launches the TUI, connects to the kernel event channel, and
  renders live updates
- **Expected Outcome:** `anvil watch` starts the Ratatui dashboard when the
  Ratatui TUI is selected, consuming real kernel events
- **Validation:** `anvil watch` launches TUI, displays live gate results on file
  change
- **Files:** `crates/anvil-tui/src/main.rs` or `crates/anvil-cli/src/`
- **Confidence:** medium
- **Priority:** High
- **Dependencies:** RATS-002, KERN-041

---

## Risks

| Risk | Likelihood | Impact | Mitigation |
| ---- | ---------- | ------ | ---------- |
| Ratatui widget gaps | Medium | Medium | Build custom widgets in eddacraft-tui |
| Terminal compatibility issues | Low | Medium | Test on common terminals (iTerm2, WezTerm, GNOME Terminal, Windows Terminal) |
| UX regression from Ink | Medium | Medium | Side-by-side comparison, user feedback |
| Theme/design system incomplete | Low | Low | Start with minimal theme, iterate |

## Stats

| Phase | Items | Status |
| ----- | ----- | ------ |
| 1 — Shared Components | 1 | Done |
| 2 — Core Surfaces | 3 | Draft |
| 3 — Integration | 3 | Draft |
| **Total** | **7** | — |
