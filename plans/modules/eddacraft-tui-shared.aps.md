<!--
APS Module: Eddacraft-TUI Shared Extraction
====================================
Extract common TUI into a shared EddaCraft repo.
See: plans/aps-rules.md
-->

# Eddacraft-TUI Shared Extraction

| ID        | Owner | Status    |
| --------- | ----- | --------- |
| TUIEXTRACT | —     | Draft |

## Purpose

Extract the common TUI widget library from `crates/eddacraft-tui` into a
standalone shared repository so other EddaCraft projects can use the theme,
keyboard, and widget library independently of Anvil.

**Problem:** The `eddacraft-tui` crate contains 15+ reusable widgets (Select,
TextInput, ProgressBar, Spinner, StatusBadge, Header, Container, Divider,
Confirm, LogPanel, ParallelProgress, QuickWinsPanel, ResultsDashboard), theme
system, and keyboard abstraction — all useful beyond Anvil. Keeping it in
the Anvil monorepo couples its release cycle to Anvil's and prevents other
EddaCraft projects from using it.

## In Scope

- **Repository extraction:** Move eddacraft-tui to `eddacraft/eddacraft-tui`
  (separate repo or workspace)
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

- `crates/eddacraft-tui` — current code to extract
- RATS (done) — the Ratatui surfaces that consume eddacraft-tui

**Exposes:**

- Standalone eddacraft-tui crate
- Published crate on crates.io
- Widget catalogue documentation

## Estimated Scope

- **Effort:** 1-2 weeks

## Tasks

- TUIEXTRACT-001: Audit eddacraft-tui for Anvil-specific imports
- TUIEXTRACT-002: Create separate repo/workspace for eddacraft-tui
- TUIEXTRACT-003: Stabilise public API surface (pub items, feature flags)
- TUIEXTRACT-004: Write widget catalogue documentation
- TUIEXTRACT-005: Set up crates.io publish pipeline
- TUIEXTRACT-006: Update Anvil to use published eddacraft-tui
- TUIEXTRACT-007: Theme customisation guide and examples
