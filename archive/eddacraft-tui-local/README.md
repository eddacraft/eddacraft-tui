# eddacraft-tui

> **⚠️ Historical archive — not the source of truth.** This is a
> _pre-publication_ fork of `eddacraft-tui`. The crate is now published on
> crates.io at `0.1.0` and consumed via the workspace dependency
> (`Cargo.toml`); **the published crate is authoritative**. This fork diverges
> from it — notably it is missing surfaces that exist only in the published
> crate (e.g. the `editor` widget consumed by `tutorial/fix.rs`). Retained for
> historical reference only; do not cite it for current widget behaviour. See
> `git log` for pre-publication history.

Shared Ratatui component library for the eddacraft product family.

## Modules

- **`theme/`** — eddacraft Terminal Standard colour palette, theme trait, and
  brand theming
- **`keyboard/`** — key binding definitions and action mapping
- **`widgets/`** — reusable TUI widgets (tables, badges, charts, panels)
- **`surface.rs`** — base `Surface` trait for TUI screens
- **`shell.rs`** — shared shell chrome renderer

## Design System

Implements the eddacraft Terminal Standard:

| Token       | Colour               |
| ----------- | -------------------- |
| Void        | `rgb(13, 13, 15)`    |
| Structure   | `rgb(42, 42, 46)`    |
| Off-White   | `rgb(235, 235, 235)` |
| Ghost Grey  | `rgb(133, 133, 138)` |
| Anvil Ember | `rgb(204, 85, 0)`    |
| Edda Growth | `rgb(46, 139, 87)`   |
| Brick Red   | `rgb(201, 74, 74)`   |
| Dull Amber  | `rgb(208, 140, 56)`  |

## Part of

[eddacraft Anvil](../../README.md) monorepo (`crates/eddacraft-tui`).
