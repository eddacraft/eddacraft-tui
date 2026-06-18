# Changelog

All notable changes to `eddacraft-tui` are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html). For 0.x releases, a
minor version bump indicates a breaking change.

## [Unreleased]

## [0.4.0] - 2026-06-11

### Added

- **`lifecycle` feature** (off by default): terminal session lifecycle helpers.
  `TerminalGuard::enter` enables raw mode and the alternate screen and restores
  both on `Drop` — including during unwinding panics via an installed panic hook
  — with `TerminalGuard::leave` for callers that want restoration errors
  surfaced, and `restore_terminal` for manual best-effort cleanup. Both are
  re-exported through the prelude.
- **`runner` feature** (off by default, implies `lifecycle`): a small fallback
  CLI shell for consumers without their own argument parser. Parses the global
  envelope only (`--help`/`-h`, `--version`/`-V`, `--theme`, `--no-tui`,
  `--config`) plus first-level command selection via the zero-dependency
  `lexopt`, handing command-specific arguments to the consumer's
  `TerminalCli::parse_command` verbatim. `launch_cli` drives the fallback path;
  `launch_with` / `launch_with_args` support bring-your-own-parser consumers
  (clap, argh, hand-rolled) that still want the runner's lifecycle/theme
  integration. Ships with `examples/runner_shell.rs`.

### Changed

- README feature table now documents the `lifecycle`, `runner`, and
  `json-render` rows, and the runner module rustdoc covers the
  bring-your-own-parser escape hatch.

> **Note:** these features were briefly visible in repository docs and the
> public mirror under the `0.3.0` version number, but were never part of the
> published `0.3.0` crate on crates.io. `0.4.0` is their first release.

## [0.3.0] - 2026-06-08

### Breaking

- **`json-render` feature API contract expanded from parse/validation to
  rendering.** Consumers that enabled the experimental `json-render` feature in
  0.2.4 should treat the module as a new rendering engine surface: it now
  exports data binding, sanitisation, responsive helpers, component renderers, a
  base registry, and `render_spec`. Code that wrapped or re-exported the
  previous parser-only module should review its public API and feature
  documentation.

### Added

- **JSON render component catalogue and renderer.** The `json-render` feature
  now includes terminal renderers for dashboard-style components including
  alerts, badges, cards, grids, headings, metrics, progress, separators, stacks,
  status badges, tables, text, and line/bar/sparkline charts.
- **Binding and catalogue sync support.** JSON render specs can bind component
  props to external data and validate canonical component names against the
  shared catalogue fixtures.
- **Responsive layout and sanitisation helpers.** The renderer now normalises
  unsafe text and adapts dashboard output to terminal constraints.

### Changed

- Expanded the `json-render` README and fixture coverage so downstream dashboard
  consumers can validate rendered terminal output against the current component
  catalogue.

## [0.2.4] - 2026-06-01

### Added

- **`json-render` feature** (off by default): a `json_render` module that parses
  the `@json-render/core` flat element spec format into typed `RenderSpec` /
  `Element` / `PropValue` structures and validates a spec against a component
  `Catalog` (unknown component types, dangling/cyclic `children` references,
  missing root). `serde`/`serde_json` are optional and only enter the dependency
  graph when the feature is enabled, so the core widget library stays
  serde-free. This is the parser foundation for rendering json-render dashboard
  specs in a terminal (TUIDASH).

### Fixed

- **`TreeNode` now drops iteratively** so dropping a deeply-nested tree cannot
  overflow the stack on teardown. The compiler-derived recursive drop blew the
  stack on a long chain — fatal on Windows' 1 MiB default thread stack (it
  survived Linux's 8 MiB), crashing deep-tree teardown with
  `STATUS_STACK_OVERFLOW`. The render walker was already iterative; teardown is
  now bounded to a single stack frame too.

## [0.2.3] - 2026-05-27

First release published from the canonical source in the Anvil monorepo
(`crates/eddacraft-tui/`). `eddacraft/eddacraft-tui` is now a read-only mirror
of that source; releases publish from the monorepo. **No public API changes** —
the relocation is byte-equivalent at the API surface. External consumers keep
depending on the crates.io release (`eddacraft-tui = "0.2"`); see the mirror
banner for the consumption contract.

### Internal

- **Canonical source relocated** into the Anvil monorepo. The public repository
  becomes a read-only, automation-mirrored copy; contributions and releases flow
  through the monorepo.
- **Mechanical clippy cleanups** to satisfy the stricter workspace `-D warnings`
  gate: `map_or` for `map_unwrap_or`, backticked identifiers in doc comments,
  `let ... else`, inlined format args, and a test-only `repeat_words` helper in
  place of a `format`-collect. No behaviour or public API change; the crate's
  `[lints]` block still treats `clippy::pedantic` as `warn` for downstream
  consumers building from crates.io.

## [0.2.2] - 2026-05-08

Hotfix for a release-build compile error introduced in 0.2.0.

### Fixed

- **`Tree::new` no longer fails to compile in release builds.** The helper
  `widgets::tree::ids_are_unique` was guarded by `#[cfg(debug_assertions)]` but
  called inside `debug_assert!` from `Tree::new`. `debug_assert!` only skips
  _evaluation_ in release builds — name resolution still runs — so any consumer
  building this crate with `debug_assertions = false` (`--release`,
  `--release-napi`, etc.) hit
  `error[E0425]: cannot find function ids_are_unique in this scope`. The
  cfg-gate has been removed; the helper now exists in every profile (it's still
  only called by `debug_assert!`, so it's a no-op at runtime in release builds).

### Internal

- **CI now runs `cargo check --release` per feature row.** Catches the exact
  regression class above: code that compiles cleanly in dev but breaks in
  release because of `debug_assertions`-gated definitions used inside
  `debug_assert!`.

Thanks for the report on issue #29.

## [0.2.1] - 2026-05-08

Brand-alignment patch release. No public Rust API changes; only visual output
and brand-text casing.

### Changed

- **Brand glyphs.** `ShellBranding::Anvil` now renders `[‡]` instead of the
  crossed-hammers `[⚒]`, matching the actual anvil logo (an I-beam silhouette in
  the eddacraft brackets). The spinner anvil preset's frames are now
  `["-", "=", "I", "‡"]` — a four-step build-up that resolves to the brand mark
  — replacing the previous hammer/tools sequence.
- **Brand text casing.** `eddacraft` and `anvil` are spelled lowercase
  throughout user-facing copy: `Cargo.toml` description and repository URL,
  `LICENSE` attribution, `README` brand links, theme doc comments, shell test
  header strings, and the regenerated `shell_chrome` snapshot. Rust identifiers
  (`EddaCraftTheme`, `ShellBranding::EddaCraft`, `SpinnerPreset::EddaCraft`)
  retain their PascalCase form for now — a follow-up rename to `Eddacraft` is
  tracked for a future breaking release.

### Note for downstream consumers

If you snapshot-test the rendered shell chrome under `ShellBranding::Anvil`, or
assert the spinner uses the previous hammer glyphs (`⚒`, `🔨`, `🛠`), your
snapshots/assertions will need updating. No source-code changes are needed; the
public Rust API is unchanged.

## [0.2.0] - 2026-05-07

A widget-suite expansion plus supply-chain and release hygiene. This release
contains breaking changes — see **Breaking** below.

### Added

#### Widgets

- `Modal` — bordered overlay dialog with severity styling shared with `Toast`.
- `Toast` and `ToastStack` — transient notifications with placement anchors and
  unicode-width-aware height calculation.
- `OverlayStack` and `Layer` — layered overlay primitive (modal/toast/popover
  foundation) with shared scrim, configurable `Placement` (Fill, Center,
  CenterPercent), and both frame-based (`render_to_frame`) and buffer-only
  (`Widget::render`) entrypoints.
- `DataTable` and `DataTableState` — selectable, scrollable table with
  `SortIndicator` and `SortDirection`.
- `Tree`, `TreeNode`, and `TreeState` — collapsible tree view with iterative
  visible-node walker and persistence helpers (`from_expanded`, `expanded_ids`).
- `HelpBar` — auto-rendering help bar driven by `KeyHandler` bindings.
- `BigBanner` — feature-gated (`big-text`) ASCII-art title widget backed by
  `tui-big-text`.
- `ImagePane` — feature-gated (`image`) inline image rendering backed by
  `ratatui-image`.
- `PretextWidget` and `PretextState` — text widget on top of the new two-phase
  `pretext` layout engine.
- Wrapper widgets `Hideable`, `Disableable`, and `Padded` for composing any
  widget without re-implementing visibility/dim/inset behaviour.

#### Modules

- `pretext` — two-phase prepare/layout text engine (measure once, lay out
  cheaply). Public types: `PreparedText`, `LayoutResult`, `PositionedWord`,
  `ExclusionZone`.
- `animation` — thin shim over the internal animation runtime so the prelude
  surface stays stable when the underlying engine changes. Public functions:
  `animate_tick`, `is_animating`.

#### Theme

- `Role` palette enum re-exported from the prelude — semantic colour roles
  (Primary, Accent, Surface, etc.) for theme implementors.

#### Spinner

- `SpinnerPreset` enum (`EddaCraft`, `Anvil`) plus `FrameSet` type and
  `eddacraft()` / `anvil()` builders. New `Spinner::with_preset`,
  `Spinner::preset`, `Spinner::eddacraft`, `Spinner::anvil`, and
  `SpinnerState::tick_with`.

#### Shell

- `ShellBranding` enum (`EddaCraft`, `Anvil`, `None`) with `mark()` and
  `footer_wordmark()` helpers.

#### Keyboard

- `Binding` struct re-exported from the prelude — declarative key→action binding
  consumed by `HelpBar`.

#### Status

- `BadgeStatus::severity_style(&theme) -> Style` — shared severity → style
  resolver, used by `Modal` and `Toast`.

#### Cargo

- Optional `image` feature → enables `ImagePane`. Pulls a substantial transitive
  graph (`ratatui-image`, `image`, `rayon`, the `windows` family, `icy_sixel`);
  only enable when needed.
- Optional `big-text` feature → enables `BigBanner`. Pulls `tui-big-text`.
- `test-utils` feature flag → public `test_utils` snapshot helpers.
- `package.metadata.docs.rs` configuration so docs.rs builds with all features
  and renders `doc(cfg(...))` badges for feature-gated items.
- `rust-version = "1.88"` declared; CI verifies via the `MSRV (1.88.0)` job.

#### Tooling

- `deny.toml` — cargo-deny configuration (advisory db, license allowlist, yanked
  = "deny", wildcard ban).
- `.oxfmtrc.json` — oxfmt configuration applied to markdown and YAML.
- `LICENSE` (Apache-2.0), `CONTRIBUTING.md`, `SECURITY.md` (with vulnerability
  reporting policy and dependency trust tiers).
- `docs/animations.md` — animation system guide.
- `docs/council-review-issues.md` — record of council review findings for the
  v0.2.0 cycle.

### Changed

#### Breaking

- **`render_shell` signature changed.** Two new positional parameters:
  `branding: ShellBranding` (after `area`) and `version: &str` (last). Every
  call site must be updated. The footer now renders the branded wordmark and
  version.
- **`#[non_exhaustive]` added to existing public enums.** Downstream code that
  exhaustively matches on these enums must add a wildcard arm:
  - `keyboard::Action`
  - `widgets::status_badge::BadgeStatus`
  - `widgets::spinner::SpinnerPreset` (introduced in this release; documented
    here so downstreams know to expect the same hygiene going forward)
- **`#[non_exhaustive]` added to
  `widgets::parallel_progress::ParallelProgressState`.** Struct-literal
  construction (`ParallelProgressState { … }`) is no longer permitted from
  outside this crate; use `ParallelProgressState::default()` or the new
  builders.
- **`#[non_exhaustive]` added to `widgets::progress_bar::ProgressBarState`.**
  Construct via `ProgressBarState::default()` and mutate the public `current` /
  `total` fields; the smoothed `display_fraction` is read via the new
  `display_fraction()` accessor (the underlying field is now crate-private to
  avoid leaking the internal `AnimatedF64` alias).
- **`#[non_exhaustive]` added to `widgets::spinner::SpinnerState`.** A new
  private `preset` field tracks the preset configured via
  [`SpinnerState::with_preset`] so subsequent `tick()` calls advance against the
  right frame count. Use `SpinnerState::default()` or `with_preset(p)`; direct
  struct-literal construction (`SpinnerState { frame: 0 }`) is no longer
  permitted from outside this crate.

#### Non-breaking

- `widgets::overlay::OverlayStack` exposes a `Widget` trait impl in addition to
  the inherent `render_to_frame` method, so it composes with `Hideable`,
  `Padded`, and ratatui layout helpers.
- Toast height calculation switched to `unicode-width` for correct sizing of
  multi-cell graphemes; bottom-anchor overflow fixed.
- Tree visible-node walker is now iterative (was recursive), avoiding stack
  pressure on deep trees.
- `TreeState` field visibility tightened to crate-private; access via the public
  methods (`cursor()`, `expanded_ids()`, `from_expanded()`, etc.).
- `DataTable` state encapsulated behind constructors and accessors.
- `widgets::wrappers::Disableable` now delegates dimming to the shared
  `dim_buffer` helper for consistent behaviour with `OverlayStack` scrim.
- `parallel_progress` gained an animated overall-progress fraction and
  spinner-frame indication for running checks.
- `progress_bar` integrates the new `animation` shim for smooth fraction
  transitions.
- README and module docs updated to advertise the new widgets and optional
  features. Crate doc rendering uses the `eddacraft` lowercase brand
  consistently.

### Fixed

- **`SpinnerState::with_preset` now stores the preset.** Previously the argument
  was discarded, so `tick()` always wrapped against the default preset's frame
  count — non-default presets like `Anvil` produced wrong frame indices when
  ticked via `tick()` (the workaround was `tick_with`). `tick()` now uses the
  stored preset.
- README usage and feature snippets pin the correct version (`0.2`).
  Acknowledgements now credit `vyfor/animate` for the animation runtime; the
  `rattles` reference (no longer a dependency) was removed.
- `CONTRIBUTING.md` local-check checklist matches CI
  (`cargo publish --dry-run --all-features`); required-status-checks list
  updated to reflect the matrix-style CI.
- `pretext`: preserve leading whitespace as indent (#16 follow-up).
- Spinner: addressed council review feedback on bracket-syntax frames and preset
  interval semantics.
- Shell: brand alignment fixes; address review feedback on rendering order.
- Wrapper dimming: use shared `dim_buffer` so disabled widgets render
  identically to scrimmed overlays.
- CI: workflow-level `permissions: contents: read` block added to satisfy CodeQL
  "Workflow does not contain permissions" advisories on `check`, `msrv`, and
  `audit` jobs.
- CI: MSRV aligned with the actual dependency tree (1.85 → 1.88) and
  `--all-features` dropped from the MSRV job (feature-gated deps need a newer
  toolchain; covered by the `Check (all-features)` row on stable).
- Various clippy warnings: `too_many_arguments`, `len_without_is_empty`, import
  ordering.

### Removed

- `rattles` dependency removed from spinner; replaced by static `FrameSet`
  tables. No public API removal — the existing `Spinner::new` /
  `SpinnerState::tick` surface is preserved.

### Internal

- CI now runs a 3-row feature matrix (`default`, `all-features`,
  `no-default-features`) plus dedicated `MSRV (1.88.0)` and
  `Supply chain (audit + deny)` jobs. `cargo publish --dry-run --all-features`
  runs on the all-features row to validate the release tarball every PR.
- Release workflow uses `cargo pkgid` for robust version extraction.

### Acknowledgements

- `vyfor/animate` powers the new animation runtime — credited in README.
- `pretext-tui` provides the layout engine integrated as the `pretext` module.

[Unreleased]: https://github.com/eddacraft/eddacraft-tui/compare/eddacraft-tui-v0.4.0...HEAD
[0.4.0]: https://github.com/eddacraft/eddacraft-tui/compare/eddacraft-tui-v0.3.0...eddacraft-tui-v0.4.0
[0.3.0]: https://github.com/eddacraft/eddacraft-tui/compare/eddacraft-tui-v0.2.4...eddacraft-tui-v0.3.0
[0.2.1]: https://github.com/eddacraft/eddacraft-tui/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/eddacraft/eddacraft-tui/compare/v0.1.0...v0.2.0
