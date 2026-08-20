# anvil-tui

Anvil TUI surfaces built on `eddacraft-tui` — interactive terminal interfaces
for governance workflows.

## Surfaces

Located in `src/surfaces/`:

- **welcome** — landing screen and navigation menu
- **status** — project health overview
- **doctor** — environment diagnostics
- **init** — project initialisation wizard
- **audit** — audit log viewer
- **browser** — file/rule browser
- **gate** — gate result explorer
- **watch** — live file-watching dashboard
- **wizard** — guided configuration
- **tutorial** — interactive onboarding tutorial

## Key Modules

- **`surface.rs`** — `Surface` trait implemented by all surfaces
- **`shell.rs`** — shared chrome (header logo, footer watermark)
- **`app.rs`** — application state and routing
- **`migration.rs`** — Ink-to-Ratatui migration utilities
- **`compat.rs`** — backwards compatibility layer

## Architecture

See [`ARCHITECTURE.md`](ARCHITECTURE.md) for the component-owned surface,
event-adapter, tutorial, snapshot, and failure contracts. Terminal lifecycle and
input polling remain owned by [`anvil-cli`](../anvil-cli/ARCHITECTURE.md);
generic theme, keyboard, shell, widget, lifecycle, and snapshot contracts remain
owned by [`eddacraft-tui`](../eddacraft-tui/README.md).

## Part of

[eddacraft Anvil](../../README.md) monorepo (`crates/anvil-tui`).
