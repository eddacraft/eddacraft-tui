# eddacraft-tui

Shared Ratatui component library for open-source TUIs that follow the eddacraft design system.

## Modules

- **`theme/`** — eddacraft Terminal Standard colour palette, theme trait, and
  brand theming
- **`keyboard/`** — key binding definitions and action mapping
- **`widgets/`** — reusable TUI widgets (tables, badges, charts, panels)
- **`pretext/`** — cached text measurement and exclusion-aware layout
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
| anvil Ember | `rgb(204, 85, 0)`    |
| edda Growth | `rgb(46, 139, 87)`   |
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
let spinner = Spinner::new(&theme).eddacraft().label("Loading...");
let forge_spinner = Spinner::new(&theme).anvil().label("Forging...");

// Pass the consuming crate's version explicitly.
// render_shell(frame, area, ShellBranding::Anvil, "anvil", "Watch", "[q] quit", &theme, env!("CARGO_PKG_VERSION"));
```

`ParallelProgress` also uses the branded `anvil` spinner automatically for running checks.

`render_shell` supports reusable shell marks for open-source apps:

- `ShellBranding::Plain`
- `ShellBranding::EddaCraft` -> `[■]`
- `ShellBranding::Edda` -> `[=]`
- `ShellBranding::Anvil` -> `[⚒]`

`ProgressBar` and `ParallelProgress` animate toward their target value. Your
event loop must call `animate_tick` each frame for the transition to play — see
[`docs/animations.md`](docs/animations.md).

## Documentation

Extended guides live in [`docs/`](docs/). Contributor docs remain at the repo
root ([`CONTRIBUTING.md`](CONTRIBUTING.md), [`RELEASE.md`](RELEASE.md),
[`SECURITY.md`](SECURITY.md)).

## Links

- eddacraft: <https://eddacraft.com>
- anvil repository: <https://github.com/EddaCraft/anvil>
- Brand and design system: <https://github.com/EddaCraft/brand-and-design>

## Acknowledgements

Spinner support is powered by [`rattles`](https://github.com/vyfor/rattles), a minimal Rust terminal spinner library.

## Licence

Apache-2.0
