# eddacraft-tui

> **This repository is a read-only mirror.** The canonical source for
> `eddacraft-tui` lives inside the Anvil project repository. Direct commits and
> source PRs against `main` here are not accepted — `main` is force-pushed by
> the mirror automation whenever the canonical tree changes, and any local
> commits will be overwritten on the next sync.

## How to depend on this crate

Depend on the **crates.io release**, not on git `main`:

```toml
[dependencies]
eddacraft-tui = "0.2"
```

`main` is mirror-managed and may be force-pushed at any time. Release tags
(`eddacraft-tui-v*`) are append-only and never rewritten by the mirror job — pin
to a tag if you need a frozen reference.

## How to report bugs or propose changes

Issues are open on this mirror — file them here and a maintainer will triage and
forward into the canonical project as needed. Source PRs opened against this
mirror are **auto-closed** by the
[`pr-redirect.yml`](.github/workflows/pr-redirect.yml) workflow with a link to
the contribution guide; auto-close protects your work, because `main` here is
force-pushed by automation on every canonical change and any local commits would
be silently overwritten on the next sync. If a maintainer accepts a change you
proposed, they will port it into the canonical tree and the next mirror sync
will carry it out — credit goes via a `Co-Authored-By:` trailer on the ported
commit.

## Why this layout

Anvil is the load-bearing consumer of `eddacraft-tui`. Hosting the canonical
source in the Anvil monorepo lets widget and consumer changes ship atomically
without a publish-then-bump round trip; mirroring out and publishing to
crates.io preserves the public trust surface for external users. See
[`CONTRIBUTING.md`](CONTRIBUTING.md) "External contributors" section for the
contribution path that applies from this mirror.

---
# eddacraft-tui

> **A curated, themed, animation-ready Ratatui component library.**
> Drop-in compatible with `ratatui` 0.30. It gives you a polished widget set,
> the `pretext` reflow-free streaming text engine, a JSON-driven UI renderer,
> smooth animations, composable overlays, and responsive layout primitives —
> with theming and shell branding fully optional. The default theme follows the
> eddacraft Terminal Standard, but nothing forces you to use it.

## Why eddacraft-tui over vanilla `ratatui`?

|                                          | `ratatui` |  `eddacraft-tui`  |
| ---------------------------------------- | :-------: | :---------------: |
| Core widget set                          |     ✓     |         ✓         |
| Curated themed widgets (status, data, …) |     —     |         ✓         |
| `pretext` two-phase text layout          |     —     |         ✓         |
| `json-render` JSON → terminal renderer   |     —     | ✓ (`json-render`) |
| Themed spinners, progress & status       |     —     |         ✓         |
| Image pane (Kitty / Sixel / iTerm2)      |     —     |    ✓ (`image`)    |
| Big-text branded splashes                |     —     |  ✓ (`big-text`)   |

Theming and shell branding are optional. `ShellBranding::Plain` (the default)
gives you a clean header/footer with no logo mark. Implement the `Theme` trait
for a completely custom palette and semantic roles. The brand-specific spinner
presets (`.anvil()`, `.eddacraft()`) are just ergonomic helpers.

## Headline features

### `json-render` — declarative JSON UI specs in the terminal

`eddacraft-tui` provides a generic engine (developed by the eddacraft team
as part of the Anvil/TUIDASH work) for the `@json-render/core` flat element
spec format. The spec format and overall approach were inspired by Vercel’s
json-render product. A spec describes a tree of components (`Stack`, `Grid`,
`Card`, `Heading`, `Text`, `Table`, `Alert`, charts, etc.) using a flat
id-referenced structure. The crate parses it, validates structural integrity
against a `Catalog` (unknown types, dangling/cyclic children, missing root),
and renders it via a pluggable `TuiRegistry`.

`serde`/`serde_json` are behind the `json-render` feature so the core
library stays free of them. A separate producer (build step, server,
LLM-emitted artefact, …) can ship one UI description that the TUI renders
with no additional glue code.

While the initial consumer and example specs are dashboards and monitoring
surfaces, the engine is not dashboard-specific. The base components are
general layout and content primitives, the catalogue is extensible, and
features like data binding (`$data` references), sanitisation for untrusted
specs, and responsive breakpoints (`Narrow`/`Medium`/`Wide`) make it
suitable for many declarative UI use cases (detail views, status pages,
settings surfaces, LLM-generated interfaces, etc.).

```toml
[dependencies]
eddacraft-tui = { version = "0.2", features = ["json-render"] }
```

```rust
use eddacraft_tui::json_render::{
    base_registry, parse, render_spec, validate, Catalog, RenderSpec, TuiRegistry,
};

let spec_json = r#"{"root":"page","elements":[...]}"#;
let spec: RenderSpec = parse(spec_json).expect("valid JSON");
validate(&spec, &Catalog::base()).expect("spec is valid");

// In your render loop, draw the spec to a ratatui Frame:
let registry: TuiRegistry = base_registry();
render_spec(&spec, &registry, frame, area);
```

Components consult shared width breakpoints (`Narrow` / `Medium` / `Wide`)
so a single spec degrades gracefully: grids collapse to one column, tables
progressively drop columns, etc. Thresholds are tuned for real terminal sizes
(80×24, 120×40, 200×60, …).

See the [`json_render` API docs] for the base element set and the
`TuiComponent` / `TuiRegistry` extension points.

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

The `widgets/` module ships a curated set of production-ready components
with a default theme that follows the eddacraft Terminal Standard (easily
replaced by implementing `Theme`). Highlights: `TextInput`, `Editor`, `Select`,
`Confirm` (inputs); `Spinner`, `ProgressBar`, `ParallelProgress`,
`StatusBadge`, `StatusBar` (status); `Container`, `Divider`, `Header`
(layout); `DataTable` (sortable, themed `▲`/`▼` indicators), `Tree`
(expand/collapse via `TreeState`), `LogPanel` (data); `OverlayStack` +
`Layer` + `Placement` (overlays for modals, toasts, command palettes);
`HelpBar` (auto-generated key hints that stay in sync with your `KeyHandler`
bindings); `PretextWidget` (text reflow); and `Hideable`, `Disableable`,
`Padded` wrappers. See [Widgets](#widgets) below for the full reference.

Other standouts include smooth animated progress (call `animate_tick` each
frame; the library handles easing), `ParallelProgress` with per-task status,
ETA calculation, and live spinners, plus careful engineering details like
iterative tree teardown to avoid stack overflows on deep hierarchies.

### Theming + animated progress

`EddaCraftTheme` provides the default 8-colour palette and semantic `Role`
tokens. `ProgressBar` and `ParallelProgress` animate toward their targets
(250 ms quad-out by default). Your event loop just calls `animate_tick(delta_ms)`
each frame (and can check `is_animating()` to decide whether to short-poll
input for smooth playback). The underlying animation engine is
[`vyfor/animate`](https://github.com/vyfor/animate). See
[`docs/animations.md`](docs/animations.md) for the wiring pattern.

Brand marks are provided via `ShellBranding` (Plain, Edda, Anvil, Custom, …)
and the convenience spinner presets, but are never required.

## Quick start

```toml
[dependencies]
eddacraft-tui = "0.2"
```

```rust
use eddacraft_tui::prelude::*;

let theme = EddaCraftTheme;

// Plain (unbranded) shell chrome is the default and recommended for most apps.
let inner = render_shell(
    frame, area,
    ShellBranding::Plain,   // or Custom("[*]"), Edda, Anvil, ...
    "myapp", "Main", "q quit  j/k move",
    &theme, env!("CARGO_PKG_VERSION"),
);

// Most widgets only need a &impl Theme. Brand presets are optional helpers:
let spinner = Spinner::new(&theme).label("Loading...");
// ... or Spinner::new(&theme).anvil().label("Forging...");
```

`ParallelProgress` can show a live animated spinner for the "Running" state
(brand mark is configurable). `render_shell` supports a few reusable marks
plus `ShellBranding::Plain` and `ShellBranding::Custom("…")`.

## Modules

- **`theme/`** — `Theme` trait + `Role` tokens for semantic styling, plus the
  built-in `EddaCraftTheme` (the default palette)
- **`keyboard/`** — key binding definitions, action mapping, and introspectable
  `Binding` table
- **`widgets/`** — reusable TUI widgets (see [Widgets](#widgets) below)
- **`pretext/`** — two-phase prepare/layout text engine for streaming AI output
  and dynamic reflow
- **`surface.rs`** — base `Surface` trait for TUI screens
- **`shell.rs`** — shared shell chrome renderer

## Default theme (EddaCraftTheme)

The built-in `EddaCraftTheme` follows the eddacraft Terminal Standard
palette. All colours are exposed through the `Theme` trait, so you can
implement your own without touching widget code.

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

The `Role` enum (`Primary`, `Accent`, `Success`, `BorderEmphasis`, …) gives
widgets a semantic handle; custom themes can override `role_style` centrally.

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

- Public crate: <https://crates.io/crates/eddacraft-tui>
- Canonical source (Anvil monorepo): <https://github.com/eddacraft/anvil>
- pretext-tui origin: <https://github.com/joshuaboys/pretext-tui>

## Acknowledgements

Smooth progress/spinner animations are powered by
[`vyfor/animate`](https://github.com/vyfor/animate).

The `pretext` two-phase layout engine and widget were originally prototyped in
[`joshuaboys/pretext-tui`](https://github.com/joshuaboys/pretext-tui), a
terminal port of ideas from Cheng Lou's browser [Pretext](https://github.com/chenglou/pretext) library.

The layered overlay model (`OverlayStack` / `Layer` / `Placement`) draws
inspiration from Cursive's `StackView`.

The `json-render` engine and supporting types were developed by the eddacraft
team (TUIDASH work in the Anvil monorepo) for portable declarative UI specs.

## Licence

Apache-2.0
