# eddacraft-tui

> **A curated, themed, animation-ready Ratatui component library for
> eddacraft-style TUIs.** Drop-in compatible with `ratatui` 0.30 — add it to
> your `Cargo.toml` and gain a polished widget set, brand theming, a
> reflow-free text layout engine, and a JSON-driven dashboard renderer.

## Why eddacraft-tui over vanilla `ratatui`?

|                                          | `ratatui` |  `eddacraft-tui`  |
| ---------------------------------------- | :-------: | :---------------: |
| Core widget set                          |     ✓     |         ✓         |
| Curated themed widgets (status, data, …) |     —     |         ✓         |
| `pretext` two-phase text layout          |     —     |         ✓         |
| `json-render` JSON → terminal renderer   |     —     | ✓ (`json-render`) |
| Brand-themed spinners & progress         |     —     |         ✓         |
| Image pane (Kitty / Sixel / iTerm2)      |     —     |    ✓ (`image`)    |
| Big-text branded splashes                |     —     |  ✓ (`big-text`)   |

## Headline features

### `json-render` — render dashboard specs in a terminal

Bring your own JSON. `eddacraft-tui` parses the `@json-render/core` flat
element spec format into typed `RenderSpec` / `Element` / `PropValue`
structures and validates the spec against a component `Catalog` —
unknown component types, dangling or cyclic `children` references, and
missing roots are caught at parse time, not at render time. A separate
tool (a build step, a server response, an LLM-emitted artefact) can
ship a UI description and the TUI renders it without any extra glue
code. `serde` and `serde_json` are feature-gated behind `json-render`,
so the core widget library stays serde-free.

```toml
[dependencies]
eddacraft-tui = { version = "0.2", features = ["json-render"] }
```

```rust
use eddacraft_tui::json_render::{
    base_registry, parse, render_spec, validate, Catalog, RenderSpec, TuiRegistry,
};

let spec_json = r#"{"root":"dashboard","elements":[...]}"#;
let spec: RenderSpec = parse(spec_json).expect("valid JSON");
validate(&spec, &Catalog::base()).expect("spec is valid");

// In your render loop, draw the spec to a ratatui Frame:
let registry: TuiRegistry = base_registry();
render_spec(&spec, &registry, frame, area);
```

This is the parser + renderer foundation for shipping json-render
dashboard specs natively in a terminal — see the [`json_render` API docs]
for the full element set (`Stack`, `Grid`, `Card`, `Separator`, `Table`,
`Heading`, `Text`, `Badge`, `StatusBadge`, `MetricCard`, `Progress`,
`BarChart`, `LineChart`, `SparklineChart`, `Alert`, `Placeholder`).

[`json_render` API docs]: https://docs.rs/eddacraft-tui/0.2.4/eddacraft_tui/json_render/

### `pretext` — reflow-free text layout for streaming AI

Two-phase text layout inspired by Cheng Lou's Pretext for the browser.
`pretext` measures word widths once with `unicode-width`, caches the
layout per container width, and re-runs only on resize. Streaming
tokens land in a frame with no reflow stutter; container resizes
invalidate the cache and re-measure in a single pass. Exclusion zones
let text flow around moving shapes — a sidebar that animates, a live
chart. The widget itself is zero-sized: all caching lives on
`PretextState`, so caching works correctly across moves and
reparenting. See [Pretext layout](#pretext-layout) below for a worked
example.

### Curated themed widget library

The `widgets/` module ships a curated component set, themed against the
eddacraft design system. Highlights: `TextInput`, `Editor`, `Select`,
`Confirm` (inputs); `Spinner`, `ProgressBar`, `ParallelProgress`,
`StatusBadge`, `StatusBar` (status); `Container`, `Divider`, `Header`
(layout); `DataTable` (sortable, themed `▲`/`▼` indicators), `Tree`
(expand/collapse via `TreeState`), `LogPanel` (data); `OverlayStack` +
`Layer` + `Placement` (overlays); `HelpBar` (chrome); `PretextWidget`
(text reflow); and `Hideable`, `Disableable`, `Padded` wrappers that
decorate any `Widget` / `StatefulWidget` without bloating each widget's
API. See [Widgets](#widgets) below for the full reference.

### Brand theming + animated progress

`EddaCraftTheme` implements the eddacraft Terminal Standard — 8-colour
palette, semantic `Role` tokens — and pairs with brand spinners:
`Spinner::new(&theme).eddacraft()` for the bracket mark `[■]`, or
`.anvil()` for `[‡]`. `ProgressBar` and `ParallelProgress` animate
toward their target value: your event loop calls `animate_tick` each
frame and the transition plays. Animations are powered by
[`vyfor/animate`](https://github.com/vyfor/animate), a minimal
animation engine for Ratatui. Full guide at
[`docs/animations.md`](docs/animations.md).

## Quick start

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
