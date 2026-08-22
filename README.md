# eddacraft-tui

> **A curated, themed, animation-ready Ratatui component library.** Drop-in
> compatible with `ratatui` 0.30. It gives you a polished widget set, the
> `pretext` reflow-free streaming text engine, a JSON-driven UI renderer, smooth
> animations, composable overlays, and responsive layout primitives — with
> theming and shell branding fully optional. The default theme follows the
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
| Interactive flow graphs                  |     —     |    ✓ (`flow`)     |
| Big-text branded splashes                |     —     |  ✓ (`big-text`)   |

Theming and shell branding are optional. `ShellBranding::Plain` (the default)
gives you a clean header/footer with no logo mark. Implement the `Theme` trait
for a completely custom palette and semantic roles. The brand-specific spinner
presets (`.anvil()`, `.eddacraft()`) are just ergonomic helpers.

## Headline features

### `json-render` — declarative JSON UI specs in the terminal

`eddacraft-tui` provides a generic engine (developed by the eddacraft team as
part of the Anvil/TUIDASH work) for the `@json-render/core` flat element spec
format. The spec format and overall approach were inspired by Vercel’s
json-render product. A spec describes a tree of components (`Stack`, `Grid`,
`Card`, `Heading`, `Text`, `Table`, `Alert`, charts, etc.) using a flat
id-referenced structure. The crate parses it, validates structural integrity
against a `Catalog` (unknown types, dangling/cyclic children, missing root), and
renders it via a pluggable `TuiRegistry`.

`serde`/`serde_json` are behind the `json-render` feature so the core library
stays free of them. A separate producer (build step, server, LLM-emitted
artefact, …) can ship one UI description that the TUI renders with no additional
glue code.

While the initial consumer and example specs are dashboards and monitoring
surfaces, the engine is not dashboard-specific. The base components are general
layout and content primitives, the catalogue is extensible, and features like
data binding (`$data` references), sanitisation for untrusted specs, and
responsive breakpoints (`Narrow`/`Medium`/`Wide`) make it suitable for many
declarative UI use cases (detail views, status pages, settings surfaces,
LLM-generated interfaces, etc.).

```toml
[dependencies]
eddacraft-tui = { version = "0.5", features = ["json-render"] }
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

Components consult shared width breakpoints (`Narrow` / `Medium` / `Wide`) so a
single spec degrades gracefully: grids collapse to one column, tables
progressively drop columns, etc. Thresholds are tuned for real terminal sizes
(80×24, 120×40, 200×60, …).

See the [`json_render` API docs] for the base element set and the `TuiComponent`
/ `TuiRegistry` extension points.

[`json_render` API docs]: https://docs.rs/eddacraft-tui/latest/eddacraft_tui/json_render/

### `pretext` — reflow-free text layout for streaming AI

Two-phase text layout inspired by Cheng Lou's Pretext for the browser. `pretext`
measures word widths once with `unicode-width`, caches the layout per container
width, and re-runs only on resize. Streaming tokens land in a frame with no
reflow stutter; container resizes invalidate the cache and re-measure in a
single pass. Exclusion zones let text flow around moving shapes — a sidebar that
animates, a live chart. The widget itself is zero-sized: all caching lives on
`PretextState`, so caching works correctly across moves and reparenting. See
[Pretext layout](#pretext-layout) below for a worked example.

### Curated themed widget library

The `widgets/` module ships a curated set of production-ready components with a
default theme that follows the eddacraft Terminal Standard (easily replaced by
implementing `Theme`). Highlights: `TextInput`, `Editor`, `Select`, `Confirm`
(inputs); `Spinner`, `ProgressBar`, `ParallelProgress`, `StatusBadge`,
`StatusBar` (status); `Container`, `Divider`, `Header` (layout); `DataTable`
(sortable, themed `▲`/`▼` indicators), `Tree` (expand/collapse via `TreeState`),
`LogPanel` (data); `OverlayStack` + `Layer` + `Placement` (overlays for modals,
toasts, command palettes); `HelpBar` (auto-generated key hints that stay in sync
with your `KeyHandler` bindings); `PretextWidget` (text reflow); and `Hideable`,
`Disableable`, `Padded` wrappers. See [Widgets](#widgets) below for the full
reference.

Other standouts include smooth animated progress (call `animate_tick` each
frame; the library handles easing), `ParallelProgress` with per-task status, ETA
calculation, and live spinners, plus careful engineering details like iterative
tree teardown to avoid stack overflows on deep hierarchies.

### Theming + animated progress

`EddaCraftTheme` provides the default 8-colour palette and semantic `Role`
tokens. `ProgressBar` and `ParallelProgress` animate toward their targets (250
ms quad-out by default). Your event loop just calls `animate_tick(delta_ms)`
each frame (and can check `is_animating()` to decide whether to short-poll input
for smooth playback). The underlying animation engine is
[`vyfor/animate`](https://github.com/vyfor/animate). See
[`docs/animations.md`](docs/animations.md) for the wiring pattern.

Brand marks are provided via `ShellBranding` (Plain, Edda, Anvil, Custom, …) and
the convenience spinner presets, but are never required.

## Quick start

```toml
[dependencies]
eddacraft-tui = "0.5"
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
(brand mark is configurable). `render_shell` supports a few reusable marks plus
`ShellBranding::Plain` and `ShellBranding::Custom("…")`.

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

The built-in `EddaCraftTheme` follows the eddacraft Terminal Standard palette.
All colours are exposed through the `Theme` trait, so you can implement your own
without touching widget code.

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

| Feature       | Adds                                                                                        |
| ------------- | ------------------------------------------------------------------------------------------- |
| `image`       | `ImagePane` — themed wrapper around [`ratatui-image`] (Kitty / Sixel / iTerm2 / halfblocks) |
| `big-text`    | `BigBanner` — themed wrapper around [`tui-big-text`] for branded splashes                   |
| `lifecycle`   | `TerminalGuard` raw-mode / alternate-screen RAII guard + panic restore                      |
| `runner`      | Small fallback CLI shell — global flags + first-level commands (enables `lifecycle`)        |
| `json-render` | Parser + registry for the `@json-render/core` declarative spec format                       |
| `flow`        | Themed `rataflow` graphs — Sugiyama layout, container boxes, zoom-to-read                   |
| `test-utils`  | Snapshot testing helpers re-exported for downstream crates                                  |

```toml
[dependencies]
eddacraft-tui = { version = "0.5", features = ["image", "big-text"] }
```

[`ratatui-image`]: https://crates.io/crates/ratatui-image
[`tui-big-text`]: https://crates.io/crates/tui-big-text

## Fallback CLI shell (`runner`)

Building a terminal tool that doesn't have a CLI yet? The `runner` feature ships
a small fallback shell so you don't hand-roll the same envelope every time. It
parses shared global flags (`--help`, `--version`, `--theme`, `--no-tui`,
`--config`) and selects a first-level command, then hands the rest to you — in
~3 lines, with no `clap`-weight dependency (it uses zero-dep [`lexopt`]
internally).

```toml
[dependencies]
eddacraft-tui = { version = "0.5", features = ["runner"] }
```

```rust,ignore
fn main() -> std::process::ExitCode {
    eddacraft_tui::runner::launch_cli(MyTool::new())
}
```

**It is a fallback, not a CLI framework.** It intentionally does not do nested
command trees, argument validation, generated per-command `--help`,
environment-variable binding, or shell completions. A serious CLI wants those —
and that is the expected outcome, not a missing feature.

When you reach that point, **bring your own parser** and hand the runner
pre-parsed options via `launch_with`. You keep full control of your CLI surface
and still get the runner's command dispatch, config handoff, and lifecycle/theme
integration. The runner never owns or re-exports your parser, so its API stays
independent of your `clap` major version.

```toml
[dependencies]
clap = { version = "4", features = ["derive"] }
eddacraft-tui = { version = "0.5", features = ["runner"] }
```

```rust,ignore
use clap::Parser;
use eddacraft_tui::runner::{launch_with, RunnerMode, RunnerOptions};

#[derive(Parser)]
#[command(name = "mytool", version)]
struct Cli {
    command: String,
    #[arg(long)]
    theme: Option<String>,
    #[arg(long = "no-tui")]
    no_tui: bool,
    #[arg(long)]
    config: Option<std::path::PathBuf>,
}

fn main() -> std::process::ExitCode {
    let cli = Cli::parse(); // clap owns help, validation, completions, …
    let options = RunnerOptions {
        command: Some(cli.command),
        config_path: cli.config,
        theme: cli.theme,
        mode: if cli.no_tui { RunnerMode::Plain } else { RunnerMode::Tui },
        ..RunnerOptions::default()
    };
    launch_with(MyApp::new(), options)
}
```

See the
[`runner` module docs](https://docs.rs/eddacraft-tui/latest/eddacraft_tui/runner/)
for the full contract.

[`lexopt`]: https://crates.io/crates/lexopt

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

### Interactive demos (public showcase)

The interesting demos that used to live in the prototype
[`pretext-tui`](https://github.com/joshuaboys/pretext-tui) repo now ship **in
this crate** so they appear on the public mirror and stay in lock-step with the
engine:

```bash
# Streaming / exclusion / masonry tabs
cargo run -p eddacraft-tui --example pretext_demos

# Agent-style dashboard (streaming + spinning exclusion + sidebar reflow)
cargo run -p eddacraft-tui --example pretext_agent_dashboard
```

Keys for `pretext_demos`: `Tab` / `1`–`3` switch tabs · `Space` pause ·
`r` reset streaming · `+`/`-` token speed · arrows move the exclusion · `q` quit.

## Documentation

Extended guides live in [`docs/`](docs/). Contributor docs remain at the repo
root ([`CONTRIBUTING.md`](CONTRIBUTING.md), [`RELEASE.md`](RELEASE.md),
[`SECURITY.md`](SECURITY.md)).

## Links

- Public crate: <https://crates.io/crates/eddacraft-tui>
- Canonical source (Anvil monorepo):
  <https://github.com/eddacraft/anvil-001/tree/main/crates/eddacraft-tui>
- Pretext demos: `cargo run -p eddacraft-tui --example pretext_demos`

## Acknowledgements

Smooth progress/spinner animations are powered by
[`vyfor/animate`](https://github.com/vyfor/animate).

The `pretext` two-phase layout engine and widget were originally prototyped in
[`joshuaboys/pretext-tui`](https://github.com/joshuaboys/pretext-tui) (now
archived; the engine and demos live here), a terminal port of ideas from Cheng
Lou's browser [Pretext](https://github.com/chenglou/pretext) library.

The layered overlay model (`OverlayStack` / `Layer` / `Placement`) draws
inspiration from Cursive's `StackView`.

The `json-render` engine and supporting types were developed by the eddacraft
team (TUIDASH work in the Anvil monorepo) for portable declarative UI specs.

## Licence

Apache-2.0
