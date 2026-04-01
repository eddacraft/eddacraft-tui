# eddacraft-tui

Shared Ratatui component library for the EddaCraft product family.

## Modules

- **`theme/`** — EddaCraft Terminal Standard colour palette, theme trait, and
  brand theming
- **`keyboard/`** — key binding definitions and action mapping
- **`widgets/`** — reusable TUI widgets (tables, badges, charts, panels)
- **`surface.rs`** — base `Surface` trait for TUI screens
- **`shell.rs`** — shared shell chrome renderer

## Design System

Implements the EddaCraft Terminal Standard:

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

## Usage

```toml
[dependencies]
eddacraft-tui = "0.1"
```

```rust
use eddacraft_tui::prelude::*;

let theme = EddaCraftTheme;
```

## Licence

Apache-2.0
