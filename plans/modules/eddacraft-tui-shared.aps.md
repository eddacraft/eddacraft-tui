<!--
APS Module: Eddacraft-TUI Shared Extraction
====================================
Extract common TUI into a shared eddacraft repo.
See: plans/aps-rules.md
-->

# Eddacraft-TUI Shared Extraction

| ID        | Owner | Status    |
| --------- | ----- | --------- |
| TUIEXTRACT | —     | Complete (crates.io v0.1.0) |

## Purpose

Extract the common TUI widget library from `crates/eddacraft-tui` into a
standalone shared repository so other eddacraft projects can use the theme,
keyboard, and widget library independently of Anvil.

**Problem:** The `eddacraft-tui` crate contains 15+ reusable widgets (Select,
TextInput, ProgressBar, Spinner, StatusBadge, Header, Container, Divider,
Confirm, LogPanel, ParallelProgress, QuickWinsPanel, ResultsDashboard), theme
system, and keyboard abstraction — all useful beyond Anvil. Keeping it in
the Anvil monorepo couples its release cycle to Anvil's and prevents other
eddacraft projects from using it.

## In Scope

- **Repository extraction:** Move eddacraft-tui to `eddacraft/eddacraft`
  (separate repo)
- **API surface stabilisation:** Define public API, mark internal items
- **Documentation:** Widget catalogue, theme customisation guide, examples
- **Publish strategy:** crates.io publish, versioning
- **Dependency decoupling:** Ensure no Anvil-specific imports remain
- **Migration path:** Update Anvil's Cargo.toml to use published crate or
  git dependency

## Out of Scope

- Moving anvil-tui (Ratatui surfaces) — those stay in Anvil
- Moving eddacraft-tui's test infrastructure — stays with the crate
- Widget redesign — extract as-is, refine later

## Interfaces

**Depends on:**

- `eddacraft/eddacraft` — current source repository for the extracted
  `eddacraft-tui` crate
- RATS (done) — the Ratatui surfaces that consume eddacraft-tui

**Exposes:**

- Standalone eddacraft-tui crate (external repo: eddacraft/eddacraft)
- Published crate on crates.io
- Widget catalogue documentation

## Estimated Scope

- **Effort:** 1-2 weeks

## Tasks

- [x] TUIEXTRACT-001: Audit eddacraft-tui for Anvil-specific imports
  - **Result:** Zero Anvil-specific imports found. Only deps are ratatui,
    crossterm, unicode-width. The `Surface` trait and `render_shell` function
    referenced `eddacraftTheme` concretely rather than the `Theme` trait — fixed
    by genericising both.
- [x] TUIEXTRACT-002: Create separate repo/workspace for eddacraft-tui
  - **Result:** Extracted to `eddacraft/eddacraft` on GitHub. Standalone
    Cargo.toml with pinned deps (no workspace refs). All 54 tests pass. Apache-2.0
    licence.
- [x] TUIEXTRACT-003: Stabilise public API surface (pub items, feature flags)
  - **Result:** `Surface<T: Theme = eddacraftTheme>` is now generic with
    backward-compatible default. `render_shell` accepts any `Theme`. Crate-level
    rustdoc added. Cargo.toml updated with publishing metadata.
- [ ] TUIEXTRACT-004: Write widget catalogue documentation
- [ ] TUIEXTRACT-005: Set up crates.io publish pipeline
- [ ] TUIEXTRACT-006: Update Anvil to use published eddacraft-tui
- [ ] TUIEXTRACT-007: Theme customisation guide and examples
