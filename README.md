# eddacraft-tui

Shared Ratatui component library for open-source TUIs that follow the eddacraft
design system.

## Modules

- **`theme/`** — eddacraft Terminal Standard colour palette, `Theme` trait,
  semantic `Role` tokens, and brand theming
- **`keyboard/`** — key binding definitions, action mapping, and introspectable
  `Binding` table
- **`widgets/`** — reusable TUI widgets (see [Widgets](#widgets) below)
- **`pretext/`** — two-phase prepare/layout text engine for streaming AI output
  and dynamic reflow
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
eddacraft-tui = "0.2"
```

```rust
use eddacraft_tui::prelude::*;

let theme = EddaCraftTheme;
let spinner = Spinner::new(&theme).eddacraft().label("Loading...");
let forge_spinner = Spinner::new(&theme).anvil().label("Forging...");

// Pass the consuming crate's version explicitly.
// render_shell(frame, area, ShellBranding::Anvil, "anvil", "Watch", "[q] quit", &theme, env!("CARGO_PKG_VERSION"));
```

`ParallelProgress` also uses the branded `anvil` spinner automatically for
running checks.

`render_shell` supports reusable shell marks for open-source apps:

- `ShellBranding::Plain`
- `ShellBranding::EddaCraft` -> `[■]`
- `ShellBranding::Edda` -> `[=]`
- `ShellBranding::Anvil` -> `[‡]`

`ProgressBar` and `ParallelProgress` animate toward their target value. Your
event loop must call `animate_tick` each frame for the transition to play — see
[`docs/animations.md`](docs/animations.md).

## Widgets

The `widgets/` module ships a curated component set. Highlights:

- **Inputs** — `TextInput`, `Editor`, `Select`, `Confirm`
- **Status** — `Spinner`, `ProgressBar`, `ParallelProgress`, `StatusBadge`,
  `StatusBar`
- **Layout** — `Container`, `Divider`, `Header`
- **Data** — `DataTable` (sortable, themed `▲`/`▼` indicators), `Tree`
  (expand/collapse via `TreeState`), `LogPanel`
- **Overlays** — `OverlayStack` + `Layer` + `Placement` for layered modals;
  `Modal` and `Toast`/`ToastStack` as ready-made consumers
- **Chrome** — `HelpBar` auto-renders key hints from
  `KeyHandler::default_bindings()`, so help text stays in sync with the keymap
- **Text reflow** — `PretextWidget` + `PretextState` for cached two-phase layout
  (see [Pretext layout](#pretext-layout) below)
- **Wrappers** — `Hideable`, `Disableable`, `Padded` decorate any
  `Widget`/`StatefulWidget` without bloating each widget's API

## Optional features

| Feature      | Adds                                                                                        |
| ------------ | ------------------------------------------------------------------------------------------- |
| `image`      | `ImagePane` — themed wrapper around [`ratatui-image`] (Kitty / Sixel / iTerm2 / halfblocks) |
| `big-text`   | `BigBanner` — themed wrapper around [`tui-big-text`] for branded splashes                   |
| `test-utils` | Snapshot testing helpers re-exported for downstream crates                                  |

```toml
[dependencies]
eddacraft-tui = { version = "0.2", features = ["image", "big-text"] }
```

[`ratatui-image`]: https://crates.io/crates/ratatui-image
[`tui-big-text`]: https://crates.io/crates/tui-big-text

## Pretext layout

`pretext` is a two-phase text layout engine inspired by Cheng Lou's
[Pretext](https://github.com/chenglou/pretext) library for the browser. Measure
word widths once with `unicode-width`, cache the layout per container width, and
re-run only on resize — eliminating reflow stutter for streaming AI output and
animated layouts.

```rust,ignore
use eddacraft_tui::prelude::*;
use ratatui::widgets::StatefulWidget;

let theme = EddaCraftTheme;
let mut state = PretextState::new("streaming tokens flow here…");
let widget = PretextWidget::themed(&theme);

// Each frame:
// widget.render(area, frame.buffer_mut(), &mut state);

// On new tokens:
state.append(" more text from the model");

// Flow text around moving shapes:
state.set_exclusions(vec![ExclusionZone::circle(40, 8, 5)]);
```

The widget itself is zero-sized — all caching lives on `PretextState`. At
unchanged container width subsequent renders skip layout entirely; the cache is
invalidated by a width change, by any text mutation (`set_text`,
`set_styled_text`, `append`, `append_styled`), by `set_exclusions`, or by an
explicit `invalidate_layout()` call.

## Documentation

Extended guides live in [`docs/`](docs/). Contributor docs remain at the repo
root ([`CONTRIBUTING.md`](CONTRIBUTING.md), [`RELEASE.md`](RELEASE.md),
[`SECURITY.md`](SECURITY.md)).

## Links

- eddacraft: <https://eddacraft.ai>
- anvil public repository: <https://github.com/eddacraft/anvil>
- Brand and design system: <https://github.com/eddacraft/brand-and-design>
- pretext-tui demos: <https://github.com/joshuaboys/pretext-tui>

## Acknowledgements

Smooth progress and spinner animations are powered by
[`vyfor/animate`](https://github.com/vyfor/animate), a minimal animation engine
for Ratatui.

The `pretext` module ports the layout engine and widget originally prototyped in
[`joshuaboys/pretext-tui`](https://github.com/joshuaboys/pretext-tui), itself
inspired by Cheng Lou's [Pretext](https://github.com/chenglou/pretext) for the
browser.

## Licence

Apache-2.0
