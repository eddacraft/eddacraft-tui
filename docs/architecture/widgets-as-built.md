# Widget Catalogue — As-Built

| Type     | Authority | Owner | Status | Freshness                                                                                                                                                                                                                    |
| -------- | --------- | ----- | ------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| As-built | Derived   | RATS  | Live   | Last reviewed 2026-07-02 (targeted delta review: keyboard `Action` non-exhaustive + variant count) against main `d1fded280`; prior delta review 2026-06-10 against `45dd1047a`; full review 2026-05-07 against `v0.6.0-beta` |

| Upstream                                   | Downstream                                                                                                                           |
| ------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------ |
| `crates/anvil-tui`, `crates/eddacraft-tui` | all surfaces in anvil-tui (audit, browser, doctor, gate, init, onboarding, status, tutorial, watch, welcome, wizard), CLI TUI runner |

> **Status:** Live (beta) **Last reviewed:** 2026-07-02 (targeted delta review)
> against main `d1fded280`; prior delta review 2026-06-10 against `45dd1047a`;
> full review 2026-05-07 against `v0.6.0-beta` slate (HEAD `cf7ca040`) **Crates
> / locations:** `crates/anvil-tui/src/widgets/` (anvil-specific) +
> `crates/eddacraft-tui` v0.3.0 (in-monorepo path crate, ADR-047; crates.io
> publication is a mirror — see "Crate resolution") **Module owner (APS):** RATS
> (Ratatui surfaces — Complete 7/7, `plans/archive/modules/ratatui-tui.aps.md`);
> the upstream extraction was tracked under TUIEXTRACT (Complete 7/7,
> `plans/archive/modules/eddacraft-tui-shared.aps.md`). The widget catalogue is
> infrastructure under those modules rather than its own module. **Used by:**
> every surface in `crates/anvil-tui/src/surfaces/` (audit / browser / doctor /
> gate / init / onboarding / status / tutorial / watch / welcome / wizard) plus
> the CLI TUI runner (`crates/anvil-cli/src/tui.rs`) which owns the
> `EddaCraftTheme` instance and the `KeyHandler::map` dispatch loop.

## Overview

The widget catalogue is the shared Ratatui widget vocabulary spanning two
crates: `eddacraft-tui` (general-purpose widgets, theme contract, keyboard
handler, shell chrome, and snapshot test helpers) and `anvil-tui::widgets`
(anvil-specific composite widgets — `quick_wins_panel` and `results_dashboard`).
Surfaces in `anvil-tui::surfaces` consume both: they pull primitives like
`TextInput`, `Editor`, `Header`, `Spinner`, etc. from the upstream library, and
they embed the anvil-specific composites in post-init / first-run flows. The CLI
runner (`crates/anvil-cli/src/tui.rs`) constructs the single `EddaCraftTheme`
instance and threads a borrow of it through `Surface::render` so every widget
paints with the same brand palette.

This doc dives into each widget — its construction, render contract, and which
surfaces consume it. For the surface-level lifecycle (event loop, shell chrome,
surface inventory) see [`tui-as-built.md`](./tui-as-built.md). The "Shared
widget vocabulary" section there is the high-level table; this doc is the deep
dive.

**Scope.** This doc covers the widget catalogue plus the theme / keyboard
contracts and snapshot pinning. The `eddacraft-tui::json_render` engine — the
TUIDASH spec→widget renderer at `crates/eddacraft-tui/src/json_render`, gated
behind the `json-render` feature (`crates/eddacraft-tui/src/lib.rs:43`) — is a
sibling module, not a widget. It is documented in
[`tui-as-built.md`](./tui-as-built.md) and intentionally out of scope here.

## Crate resolution

The live `eddacraft-tui` is the **in-monorepo path crate at
`crates/eddacraft-tui`, version 0.3.0** (ADR-047 path consumption) — not the
published crates.io artefact. Confirmed three ways:

- `Cargo.toml:74` declares `eddacraft-tui = { path = "crates/eddacraft-tui" }`
  in `[workspace.dependencies]`.
- `Cargo.lock:1597-1598` resolves `eddacraft-tui v0.3.0` with **no registry
  source** — a path dependency, not a checksum-pinned download.
- `crates/anvil-tui/Cargo.toml:16` consumes it as
  `eddacraft-tui = { workspace = true, features = ["json-render"] }`; the
  dev-dependency at `:27` adds the `test-utils` feature. Workspace inheritance
  keeps a single resolved source.

crates.io publication exists (tags `eddacraft-tui-v0.2.x`, `-v0.3.0`) but is the
**mirror**, not the consumed artefact — the workspace builds against the path
crate. File references in this doc to upstream code use the
`crates/eddacraft-tui/src/...` path prefix.

`anvil-archive/eddacraft-tui-local/` is the **historical** local fork, kept
read-only for reference. It was the pre-TUIEXTRACT in-monorepo crate before the
upstream moved to a separate repository (`eddacraft/eddacraft`) and was first
published on crates.io
(`plans/archive/modules/eddacraft-tui-shared.aps.md:10-12`); the crate has since
returned to the monorepo as a path crate (ADR-047). The archive diverges from
the live crate — most notably it does **not** contain the `editor` widget, which
was added after extraction. Treat the archive as a git fossil; the path crate is
the source of truth.

## Architecture diagram

```text
                    ┌──────────────────────────────────┐
                    │  crates/anvil-cli/src/tui.rs     │
                    │   - owns EddaCraftTheme          │
                    │   - drives KeyHandler::map loop  │
                    │   - calls render_shell + Surface │
                    └─────────────┬────────────────────┘
                                  │ &theme, action
                                  ▼
   ┌─────────────────────────────────────────────────────┐
   │  anvil-tui surfaces  (audit / browser / doctor /    │
   │  gate / init / onboarding / status / tutorial /     │
   │  watch / welcome / wizard)                          │
   └────┬───────────────────────────────────┬────────────┘
        │ composite widgets                 │ primitives
        ▼                                   ▼
   ┌────────────────────────┐      ┌─────────────────────────────┐
   │ anvil-tui::widgets     │      │ eddacraft-tui::widgets       │
   │  QuickWinsPanel        │      │  confirm    container        │
   │  ResultsDashboard      │──────│  divider    editor           │
   │   (embeds Header,      │      │  header     log_panel        │
   │    QuickWinsPanel)     │      │  parallel_progress           │
   └────────────────────────┘      │  progress_bar  select        │
                                   │  spinner    status_badge     │
                                   │  status_bar text_input       │
                                   │  …+9 new (22 total; see      │
                                   │   §Upstream widgets)         │
                                   └────────────┬────────────────┘
                                                │
                          ┌─────────────────────▼──────────────────┐
                          │ eddacraft-tui::theme::Theme trait       │
                          │   + EddaCraftTheme impl                 │
                          │   (bg/fg/accent/success/error/warning/  │
                          │    muted/border + derived styles)       │
                          └─────────────────────────────────────────┘

   ┌────────────────────────────────────┐    ┌─────────────────────────────┐
   │ eddacraft-tui::keyboard::KeyHandler │    │ eddacraft-tui::test_utils    │
   │   crossterm KeyEvent → Action enum  │    │   buffer_to_string           │
   └────────────────────────────────────┘    │   (style-aware insta snaps)  │
                                              └─────────────────────────────┘

   Snapshot pinning layer wraps all rendering. Two snapshot dirs:
   - crates/eddacraft-tui/src/snapshots/                  (1 file: shell chrome)
   - crates/anvil-tui/src/**/snapshots/            (41 files across surfaces)
```

The diagram matches the lifecycle: the CLI runner instantiates a theme, selects
a surface, and pumps keys through `KeyHandler::map` to `Surface::handle_key`.
The surface composes upstream primitives and anvil-specific composites; both
pull styles exclusively from the shared `Theme` trait. The snapshot
infrastructure wraps the entire render path — every widget that renders into a
`TestBackend` gets serialised through `buffer_to_string` for
cell-and-style-aware diffing.

## Theme contract (`eddacraft-tui::theme`)

The theme contract is a single `Theme` trait with eight required colour hooks,
ten default-implemented derived styles, and a `Role` enum with a
`role_style(role) -> Style` dispatcher. The `EddaCraftTheme` struct provides the
canonical brand-palette implementation. All widgets in both crates are generic
over `T: Theme`, so a downstream product can swap in a different palette without
forking the widget code.

### `Theme` trait

`crates/eddacraft-tui/src/theme/traits.rs:38-46` declares the eight required
colour hooks. The implementor contract doc-comment (`traits.rs:26-37`) requires
every style method to return a `Style` with `fg` (and, where semantically
meaningful, `bg`) explicitly populated — internal widget tests rely on e.g.
`theme.status_error().fg.unwrap()`. Required methods (each returning a
`ratatui::style::Color`):

| Method      | Semantic role                                               |
| ----------- | ----------------------------------------------------------- |
| `bg()`      | Surface background                                          |
| `fg()`      | Default foreground                                          |
| `accent()`  | Brand accent (used for focused borders, titles, highlights) |
| `success()` | Pass / OK                                                   |
| `error()`   | Fail / Error                                                |
| `warning()` | Warn / Caution                                              |
| `muted()`   | De-emphasised foreground (help text, hints)                 |
| `border()`  | Default unfocused border                                    |

Derived styles (ten, default-implemented on the trait at
`crates/eddacraft-tui/src/theme/traits.rs:48-100` — never override unless the
palette demands it):

- `base()` — `fg()` over `bg()`. The default span style.
- `highlighted()` — `bg()` over `accent()`, BOLD. Selected list rows.
- `highlight_inactive()` — `fg()` over `border()`, BOLD. Selected-but-unfocused
  rows (`traits.rs:59`).
- `title()` — `accent()` BOLD. Block titles and labels.
- `border_focused()` — `accent()`. Focused panel borders.
- `border_unfocused()` — `border()`. Unfocused panel borders.
- `status_ok()` — `success()` BOLD.
- `status_error()` — `error()` BOLD.
- `status_warning()` — `warning()` BOLD.
- `disabled()` — `muted()`. Help text, hints, placeholders.

The `Role` enum (`traits.rs:13-24` — `Primary` / `Secondary` / `Accent` /
`Highlight` / `HighlightInactive` / `Success` / `Warning` / `Error` /
`BorderSubtle` / `BorderEmphasis`) plus the `role_style(role) -> Style`
dispatcher (`traits.rs:102`) let widgets reference _what a colour means_
centrally instead of binding to an individual style helper; built-in widgets
still resolve styles directly, but new widgets are encouraged to go through
`role_style` so downstream themes can override roles in one place.

### `EddaCraftTheme` palette

`crates/eddacraft-tui/src/theme/eddacraft.rs:12-55` ships the canonical palette
with brand-vocabulary names:

| Brand name  | Role      | RGB               |
| ----------- | --------- | ----------------- |
| The Void    | `bg`      | `(13, 13, 15)`    |
| Off-White   | `fg`      | `(235, 235, 235)` |
| Anvil Ember | `accent`  | `(204, 85, 0)`    |
| Edda Growth | `success` | `(46, 139, 87)`   |
| Brick Red   | `error`   | `(201, 74, 74)`   |
| Dull Amber  | `warning` | `(208, 140, 56)`  |
| Ghost Grey  | `muted`   | `(133, 133, 138)` |
| Structure   | `border`  | `(42, 42, 46)`    |

The `theme_colours_are_distinct` test
(`crates/eddacraft-tui/src/theme/eddacraft.rs:62-82`) pins that all eight
colours are pairwise distinct — this is a load-bearing invariant for snapshot
diffing because two semantic roles collapsing to the same RGB would silently
mask regressions.

### How surfaces consume the theme

The CLI runner constructs `EddaCraftTheme` once per surface session and borrows
it into the render path:

- `crates/anvil-cli/src/tui.rs:39, 41` — `let theme = EddaCraftTheme;` followed
  by `surface_loop(&mut terminal, &mut state, &theme)`.
- `Surface::render` takes `theme: &T` where `T: Theme` defaults to
  `EddaCraftTheme` (`crates/eddacraft-tui/src/surface.rs:13, 32`). Surfaces
  forward the borrow into widget constructors and inline styling.

Convention: surfaces construct ad-hoc `Style`s only as
`Style::default().fg(theme.<role>())` — the colour value always comes from the
trait, never from a `Color::Rgb(…)` literal. A grep of `crates/anvil-tui/src`
for `Color::` returns zero hits at the time of review — the contract holds.

## Keyboard handler (`eddacraft-tui::keyboard`)

A two-piece module: a flat `Action` enum and a stateless `KeyHandler::map`
function that translates a single `crossterm::event::KeyEvent` into one
`Action`.

### `Action` enum

`crates/eddacraft-tui/src/keyboard/handler.rs:4-22` — 16 variants:
`Up / Down / Left / Right / Select / Toggle / Back / Quit / Character(char) / Backspace / Delete / Home / End / PageUp / PageDown / None`.
The enum is `#[non_exhaustive]` (`handler.rs:4`), so downstream `match` arms
must carry a wildcard and new navigation actions can be added without a breaking
change.

`None` is the explicit "key event the handler does not recognise" sentinel —
surfaces match against it as `_ => {}` (or ignore it implicitly). The enum is
`Copy + Eq + Hash`, which keeps surface key handlers branch-free.

### `KeyHandler::map` contract

`crates/eddacraft-tui/src/keyboard/handler.rs:93-119`. The mapping is fully
static — there is no chord support, no rebinding, no stateful machine. The
module now also exports a `Binding` descriptor alongside `Action` / `KeyHandler`
(`crates/eddacraft-tui/src/keyboard/mod.rs:3`, prelude `lib.rs:64`): a
`{ keys, action, label }` record that `KeyHandler::default_bindings()` exposes
as the curated key-hint list consumed by `HelpBar`. `Binding` is display
metadata for help text, **not** a rebinding surface — `map` does not consult it.
The two hard-coded conventions are:

1. **`Ctrl+C` is `Action::Quit`** unconditionally (`handler.rs:28-32`). Every
   other Control-modified key returns `Action::None`. This means surfaces cannot
   bind `Ctrl+<anything-else>` without working around the handler.
2. **Vim and arrow keys are coalesced** into the same actions
   (`handler.rs:35-38`): `j/Down → Down`, `k/Up → Up`, `h/Left → Left`,
   `l/Right → Right`. This is why the watch / status / audit surfaces never have
   to switch on raw `KeyCode` for navigation.

The escape-and-quit convention is also baked in: `Esc → Back`, `q → Quit`,
`Enter → Select`, `Space → Toggle`. Surfaces never see those raw keycodes — they
only react to the mapped `Action`.

### Global vs surface-local

There is no global / local split inside the handler — every surface gets the
same `Action` for the same key. The split lives on the surface side:
`Surface::handle_key(&mut self, action: Action)` is the only override point
(`crates/eddacraft-tui/src/surface.rs:19`). Surfaces that need character-level
input (text inputs, search prompts) match on `Action::Character(c)` and forward
into their own state (e.g. `TextInputState::insert` at
`crates/eddacraft-tui/src/widgets/text_input.rs:39-42`).

The "search mode" / "fix" / "zoom" affordances on surfaces like gate / audit /
watch are implemented entirely in `handle_key` on the surface — the keyboard
module itself has no concept of them. This is intentional flatness: the upstream
library hands the surface a vocabulary, not a state machine.

## Upstream widgets (`eddacraft-tui::widgets`)

The live path crate (v0.3.0) ships **22 widgets** declared in
`crates/eddacraft-tui/src/widgets/mod.rs:9-33`; `big_banner` and `image_pane`
sit behind the `big-text` / `image` cargo features (`mod.rs:7-8, 17-18`). The
lib-level prelude (`crates/eddacraft-tui/src/lib.rs:72-101`) re-exports them
flat for downstream ergonomics. The `wrappers` module (`Hideable` /
`Disableable` / `Padded`, `widgets/wrappers.rs:32`) ships composition
combinators alongside the widgets and is not counted as a widget.

The 13 original widgets each have a deep-dive subsection below (ordering is
alphabetical). The 9 widgets added since the 2026-05-07 full review are
summarised in the following table; their deep dives are owed to the next full
review (Known gaps G-07). Pins are relative to `crates/eddacraft-tui/src/`.

| Widget       | Public types                                                       | Pin                         |
| ------------ | ------------------------------------------------------------------ | --------------------------- |
| `big_banner` | `BigBanner`                                                        | `widgets/big_banner.rs:31`  |
| `data_table` | `DataTable` / `DataTableState` / `SortDirection` / `SortIndicator` | `widgets/data_table.rs:142` |
| `help_bar`   | `HelpBar`                                                          | `widgets/help_bar.rs:22`    |
| `image_pane` | `ImagePane`                                                        | `widgets/image_pane.rs:44`  |
| `modal`      | `Modal`                                                            | `widgets/modal.rs:44`       |
| `overlay`    | `Layer` / `OverlayStack` / `Placement`                             | `widgets/overlay.rs:81`     |
| `pretext`    | `PretextWidget` / `PretextState`                                   | `widgets/pretext.rs:17`     |
| `toast`      | `Toast` / `ToastStack` / `ToastPlacement`                          | `widgets/toast.rs:43`       |
| `tree`       | `Tree` / `TreeNode` / `TreeState`                                  | `widgets/tree.rs:158`       |

### `confirm`

**Purpose.** Inline yes/no confirmation prompt. Single-line widget that draws
`<message> Yes / No (y/n)` with the highlighted button styled by
`title()`-with-bold. `crates/eddacraft-tui/src/widgets/confirm.rs:9-104`.

**Constructor.** `Confirm::new(message, theme).block(block?)`. Stateful via
`ConfirmState { selected: bool, confirmed: Option<bool> }` — `selected` defaults
to `true`, `toggle()` flips it, `confirm() / confirm_yes() / confirm_no()`
resolve, `reset()` clears (`confirm.rs:21-55`).

**Render.** `StatefulWidget`. Renders a single line into `inner`; renders
nothing if `inner.height == 0` or `inner.width == 0` (`confirm.rs:80-82`). The
"No" branch uses `status_error` BOLD when selected, `disabled` when unselected.

**Notable invariants.** No allocation on the render path beyond the formatted
message string. Pure single-frame paint — no animation.

**Source.** `crates/eddacraft-tui/src/widgets/confirm.rs`.

**Anvil consumers.** `surfaces/onboarding/hooks.rs` uses `HooksPhase::Confirm`
semantically — but it draws the confirm prompt itself rather than instantiating
the widget (search for `Confirm` returns zero direct uses outside the crate
tests). At time of review the upstream `Confirm` widget is **defined but not
consumed** in `anvil-tui`.

### `container`

**Purpose.** Themed `ratatui::Block` factory with three variants — `Primary`
(double border, focused colour, title), `Secondary` (plain border, focused
colour, title), `Subtle` (rounded border, unfocused colour, disabled title).
`crates/eddacraft-tui/src/widgets/container.rs:8-79`.

**Constructor.** `Container::new(theme).title(title?).variant(variant?)`.
`to_block() -> Block<'a>` returns a configured `Block` for direct render;
`inner(area)` returns the post-block content rect.

**Render.** Both `Widget` (which renders the block) and direct `to_block()`
extraction so callers can compose it into a stateful render.

**Notable invariants.** The variant choice is pure data — no theme call out
beyond the four pre-bound combinations (`container.rs:44-72`). `Primary` is the
focused-with-emphasis case; `Subtle` is the de-emphasised background frame.

**Source.** `crates/eddacraft-tui/src/widgets/container.rs`.

**Anvil consumers.** Now consumed at **21 sites** across the dashboard surface
family (e.g. `crates/anvil-tui/src/surfaces/plan_dashboard/render.rs:105` —
`Container::new(theme).title("Summary").variant(ContainerVariant::Primary)`).
The 2026-05-07 review's "not directly consumed" status is obsolete — see the
rewritten Known gaps G-02. Older surfaces (audit, watch) still build their own
`ratatui::widgets::Block` instances with theme-derived border styles.

### `divider`

**Purpose.** Single-line horizontal rule. Two variants — `Heavy` (`━`,
`border_focused`) and `Light` (`─`, `border_unfocused`). Optional custom
character override (`character: Option<char>`) replaces the variant glyph
without affecting the style. `crates/eddacraft-tui/src/widgets/divider.rs:8-58`.

**Constructor.** `Divider::new(theme).variant(variant?).character(c?)`.

**Render.** `Widget`. Repeats the chosen character `area.width` times, renders
into the first row only. No-ops when area is zero (`divider.rs:45-46`).

**Source.** `crates/eddacraft-tui/src/widgets/divider.rs`.

**Anvil consumers.** Not directly consumed in `anvil-tui`. Inlined as ad-hoc
`Line::styled("─".repeat(width), theme.border_unfocused())` in several surfaces.

### `editor`

**Purpose.** Multi-line text editor with cursor, scroll, selectable editable
region, dirty/saved flags, and read-only context windows. The heaviest widget in
the catalogue at 1005 lines.
`crates/eddacraft-tui/src/widgets/editor.rs:1-1005`.

**Constructor / config.** `Editor::new(theme).block(block?)`. Stateful via
`EditorState`, which is the load-bearing type:

- `EditorState::new()` — empty single-line buffer (`editor.rs:41-51`).
- `EditorState::from_string(content)` — load from string, all lines editable
  (`editor.rs:94-115`).
- `EditorState::from_file(path)` — load from disk (`editor.rs:54-59`).
- `EditorState::from_file_with_context(path, focus_line, editable_above, editable_below)`
  — load with a read-only context window around a focus line. This is the
  variant the tutorial-fix surface uses (`editor.rs:66-91`).

State exposes navigation
(`home / end / move_left / move_right / move_up / move_down / page_up / page_down`),
editing (`insert / backspace / delete`, gated by `is_editable`), and persistence
(`dirty`, `saved`, `file_path`).

**Render.** `StatefulWidget` — paints lines with the editable lines styled
normally, the read-only context lines styled with `disabled()`, and the cursor
as a `REVERSED` modifier on the cursor cell.

**Notable invariants.** `is_editable` gates all mutation (`editor.rs:120-123`);
read-only lines silently reject `insert` / `backspace` / `delete`. The
`editable_range` is `(start, end)` with `start` inclusive and `end` exclusive
(`editor.rs:21`). `clamp_cursor_to_line` keeps the cursor byte offset on a char
boundary after vertical movement (`editor.rs:156-161`).

**Source.** `crates/eddacraft-tui/src/widgets/editor.rs`.

**Anvil consumers.** `crates/anvil-tui/src/surfaces/tutorial/fix.rs:54` and
`fix_render.rs:163` — the in-tutorial fix surface uses
`EditorState::from_file_with_context` to render a focus window around an
auto-fixable finding line, with the surrounding lines as read-only context. This
is the **only** upstream widget that owns persistent multi-line state in
`anvil-tui`.

### `header`

**Purpose.** Three-row branded header — separator rule + uppercased title (with
optional version suffix in `disabled` style) + optional subtitle in `disabled`
style. `crates/eddacraft-tui/src/widgets/header.rs:8-67`.

**Constructor.** `Header::new(title, theme).subtitle(subtitle?).version(v?)`.

**Render.** `Widget`. Row 0 is the `━` separator, row 1 is the title, row 2 is
the optional subtitle. Bails early if `area.height` is too small
(`header.rs:48-50, 60-61`).

**Source.** `crates/eddacraft-tui/src/widgets/header.rs`.

**Anvil consumers.** `crates/anvil-tui/src/widgets/results_dashboard.rs:7, 112`
— the `ResultsDashboard` composite uses
`Header::new("Initial Analysis", self.theme).subtitle(self.results.project_root.as_str())`
as its top row. **Only consumer** at time of review.

### `log_panel`

**Purpose.** Filterable log viewer with level checkboxes
(`Error / Warn / Info / Debug`), text search, auto-scroll, and selection.
Carries 561 lines of state machine — the second-heaviest widget after `editor`.
`crates/eddacraft-tui/src/widgets/log_panel.rs:1-561`.

**Constructor.**
`LogPanel::new(entries, theme) .block(block?).max_visible(N).title("…").show_filter(bool).show_search(bool).focused(bool)`.
Stateful via
`LogPanelState { selected_index, scroll_offset, filter, search_mode, search_input, auto_scroll, last_entry_count }`.

**Render.** `StatefulWidget`. Reserves a header row for filter checkboxes, an
optional search prompt row, and the entries list. Each entry renders as
`[<timestamp>] <LEVEL> <message>  <source>` with the level coloured by
`status_*` styles. Auto-scroll snaps to the newest entry when `auto_scroll` is
set and a new entry has arrived since the last paint.

**Source.** `crates/eddacraft-tui/src/widgets/log_panel.rs`.

**Anvil consumers.** Not directly consumed in `anvil-tui` at time of review. The
watch surface's queue / history panels render their own notification rows rather
than going through `LogPanel`. Candidate upstream-widget for unused inventory —
see Known gaps G-02.

### `parallel_progress`

**Purpose.** Multi-row parallel-task progress dashboard with per-task status
(`Pending / Running / Passed / Failed / Skipped / Cached`), per-task progress
percent, ETA estimation, and overall aggregate. Uses unicode 1/8th block
characters (`▏▎▍▌▋▊▉█`) for sub-cell progress fidelity.
`crates/eddacraft-tui/src/widgets/parallel_progress.rs:1-428`.

**Constructor.** `ParallelProgress::new(theme).block(block?).title(title?)`.
Stateful via
`ParallelProgressState { checks: Vec<CheckProgress>, start_time: Option<Instant> }`.

**Free functions.** `calculate_overall_progress(checks)` returns the mean
progress as a `u8` (`parallel_progress.rs:59-73`);
`calculate_eta(checks, elapsed)` extrapolates remaining time
(`parallel_progress.rs:76-89`); `format_duration(ms)` formats human durations.

**Notable invariants.** `Cached` checks count as 100% complete in the overall
aggregate. ETA returns `None` when progress is 0 or 100, so the UI never shows a
meaningless "ETA: 0s" or "ETA: ∞".

**Source.** `crates/eddacraft-tui/src/widgets/parallel_progress.rs`.

**Anvil consumers.** Not directly consumed in `anvil-tui` at time of review. The
gate surface renders its check list manually rather than going through
`ParallelProgress`. Candidate upstream-widget for inventory tracking — see Known
gaps G-02.

### `progress_bar`

**Purpose.** Single-line progress bar with `current / total` state and an
optional label. Uses `█` / `░` block characters; renders as
`<label>: ████░░░░ 50%`.
`crates/eddacraft-tui/src/widgets/progress_bar.rs:1-86`.

**Constructor.** `ProgressBar::new(theme).block(block?).label(label?)`. Stateful
via `ProgressBarState { current: u64, total: u64 }` with a `fraction()` helper
that clamps to `[0.0, 1.0]` and returns `0.0` for zero totals
(`progress_bar.rs:21-28`).

**Source.** `crates/eddacraft-tui/src/widgets/progress_bar.rs`.

**Anvil consumers.** Not directly consumed. The `QuickWinsPanel`
(`anvil-tui::widgets`) draws its own `[#---] N/M (X%)` ASCII progress line via
`render_progress` rather than instantiating `ProgressBar`
(`crates/anvil-tui/src/widgets/quick_wins_panel.rs:177-193`). This is deliberate
— the `QuickWinsPanel` ASCII-only choice mirrors the watch action footer
ASCII-only pin (cross-link `tui-as-built.md#G-06`).

### `select`

**Purpose.** Vertical list selector with optional per-item description.
Wrap-around navigation, scroll offset, focused-row highlight via `highlighted()`
style. `crates/eddacraft-tui/src/widgets/select.rs:1-212`.

**Constructor.** `Select::new(items, theme).block(block?)` where
`items: IntoIterator<Item: Into<SelectItem>>`.
`SelectItem { label, description: Option<String> }` accepts `String` / `&str`
via `From` for ergonomic call sites.

Stateful via `SelectState { selected, offset }` with
`next(item_count) / previous(item_count)` cycling helpers (`select.rs:53-67`).

**Render.** `StatefulWidget`. Highlights the selected row; if the item has a
description, renders the description on a second row in `disabled` style.

**Source.** `crates/eddacraft-tui/src/widgets/select.rs`.

**Anvil consumers.** Not directly consumed at time of review. Surfaces like
welcome / wizard / browser / init render their selectable lists inline, often
with surface-specific decorations (status icons, badges) that don't fit the
`SelectItem` shape. Candidate upstream-widget for review when surface lists are
next refactored — see Known gaps G-02.

### `spinner`

**Purpose.** Single-cell animated spinner with optional label, advanced via
`SpinnerState::tick()`. The frame set is no longer fixed: `SpinnerPreset`
(`widgets/spinner.rs:69`) selects between the `eddacraft()` braille frame set
(`spinner.rs:52`) and the `anvil()` frame set (`spinner.rs:60`), each a
`FrameSet { frames, interval }`. `crates/eddacraft-tui/src/widgets/spinner.rs`.

**Constructor.** `Spinner::new(theme).label(label?)`. Stateful via
`SpinnerState { frame: usize }`. `tick()` advances and wraps.

**Source.** `crates/eddacraft-tui/src/widgets/spinner.rs`.

**Anvil consumers.** Not directly consumed. The watch surface's `Running` status
uses a static glyph rather than the upstream spinner. Candidate upstream-widget
for inventory tracking — see Known gaps G-02.

### `status_badge`

**Purpose.** Single-line status pill — `<icon> <label>` pair. Six statuses:
`Success (◆ Passed)`, `Error (✖ Failed)`, `Warning (◈ Warning)`,
`Info (◇ Info)`, `Running (● Running)`, `Skipped (○ Skipped)`. Each maps to a
`status_*` or `disabled` style.
`crates/eddacraft-tui/src/widgets/status_badge.rs:9-68`.

**Constructor.** `StatusBadge::new(status, theme).label(label?)`. The `label`
override falls back to a status-specific default (`status_badge.rs:40-49`).

**Notable invariants.** Glyphs are unicode but in the geometric-shape plane,
which renders cleanly on most terminals. The watch action footer
(`tui-as-built.md` watch deep dive) uses ASCII-only `[*]` / `[x]` / `[!]` glyphs
instead because the watch dashboard is the demo path — `StatusBadge` is for the
in-app contexts that aren't subject to that constraint.

**Source.** `crates/eddacraft-tui/src/widgets/status_badge.rs`.

**Anvil consumers.** Now consumed at **3 sites** in the dashboard surface family
(e.g. `crates/anvil-tui/src/dashboard_catalog/gate_result.rs:56`; also
`dashboard_catalog/suppression.rs` and `surfaces/plan_dashboard/render.rs`).
Status glyphs in the older gate / doctor / status surfaces are still rendered
inline with the surface's own icon helpers.

### `status_bar`

**Purpose.** Two-section bottom-of-surface status strip — `left` items
(left-aligned) and `right` items (right-aligned). Each item has a `StatusKind`
(`Normal / Success / Error / Warning / Muted`) that selects the styling.
`crates/eddacraft-tui/src/widgets/status_bar.rs:8-88`.

**Constructor.** `StatusBar::new(theme).left(items).right(items)` where items
are `StatusItem { label, kind }`.

**Render.** `Widget`. Splits the area 50/50 horizontally and renders each
section. Sets `theme.base()` over the entire area first so the bar has a solid
background.

**Source.** `crates/eddacraft-tui/src/widgets/status_bar.rs`.

**Anvil consumers.** Not directly consumed at time of review. The shell chrome
footer (`crates/anvil-tui/src/shell.rs` re-exports +
`crates/eddacraft-tui/src/shell.rs:42-71`) renders its own help-text + watermark
layout rather than going through `StatusBar`. Candidate upstream-widget for
inventory tracking — see Known gaps G-02.

### `text_input`

**Purpose.** Single-line text input with cursor positioning, char-boundary
safety, and placeholder text.
`crates/eddacraft-tui/src/widgets/text_input.rs:1-291`.

**Constructor.** `TextInput::new(theme).block(block?).placeholder(s)`. Stateful
via `TextInputState { value: String, cursor: usize }` (cursor is private;
`cursor()` and `set_cursor(pos)` provide the API). `set_cursor` clamps to
`value.len()` and snaps to the nearest valid char boundary
(`text_input.rs:29-37`).

State exposes
`insert(c) / backspace() / delete() / move_left() / move_right() / home() / end()`
— all char-boundary-safe and UTF-8-aware (`text_input.rs:39-89`).

**Render.** `StatefulWidget`. Renders `value` (or `placeholder` in `disabled`
style if empty), with the cursor cell styled `REVERSED`.

**Source.** `crates/eddacraft-tui/src/widgets/text_input.rs`.

**Anvil consumers.** `crates/anvil-tui/src/surfaces/init/mod.rs:4, 142, 170` —
init wizard "Directory" step uses `TextInputState` for the directory path field.
`crates/anvil-tui/src/surfaces/wizard/mod.rs:4, 80, 107` — `anvil new` wizard
"ProjectName" step uses `TextInputState` for the project name field. Both
surfaces hold the state and let the upstream widget render it.

## anvil-specific widgets (`anvil-tui::widgets`)

Two composite widgets, both anvil-product-specific. Declared in
`crates/anvil-tui/src/widgets/mod.rs:1-2`:

```rust
pub mod quick_wins_panel;
pub mod results_dashboard;
```

Both consume the upstream `Theme` trait and (in the case of `ResultsDashboard`)
the upstream `Header` widget.

### `QuickWinsPanel`

**Purpose.** Batched warning-suppressions panel for the post-init analysis flow
(LAUNCH-006 onboarding). Shows a progress bar of `suppressable / total_warnings`
warnings, the top 5 batch groups by count, type icons (`T/D/C/G/M/P/L` for
`TestFile / TypeDefinition / ConfigFile / GeneratedCode / Migration / ThirdParty / LegacyCode`),
and an empty-state line when no batch groups exist.
`crates/anvil-tui/src/widgets/quick_wins_panel.rs:9-152`.

**Constructor.**
`QuickWinsPanel::new(analysis, theme).block(block?).focused(bool)` where
`analysis: &QuickWinsAnalysis`. Stateful via `QuickWinsPanelState` (currently a
unit struct — placeholder for future selection / scroll state).

**Render contract.** `StatefulWidget`. Reserves three vertical sections: a 1-row
progress bar, a flexible-height batch group list (capped at top 5), and a 1-row
tip footer ("Tip: batch suppressions by pattern for fastest clean-up").

The progress line is intentionally ASCII-only — `[####----] 12/20 (60%)` — see
`render_progress` at `crates/anvil-tui/src/widgets/quick_wins_panel.rs:177-193`.
This mirrors the watch action footer ASCII-only pin
(`tui-as-built.md#cross-cutting-concerns`).

**Theme usage.** Title via `theme.title()`, base text via `theme.base()`, batch
group counts via `theme.title()`, tip via `theme.disabled()`. Type icons use
semantic colour mapping via `type_style` (`quick_wins_panel.rs:167-175`):

- `TestFile` → `theme.success()` (green — testing harness, low risk)
- `TypeDefinition` → `theme.accent()` (ember — opinionated)
- `ConfigFile / Migration` → `theme.warning()` (amber — needs review)
- `GeneratedCode / ThirdParty` → `theme.disabled()` (grey — out of scope)
- `LegacyCode` → `theme.error()` (red — flagged)

Theme drift check: zero `Color::` literals in the file. All colour comes from
the theme contract.

**Source.** `crates/anvil-tui/src/widgets/quick_wins_panel.rs`.

**Anvil consumers.**

- Embedded inside `ResultsDashboard` as the "Quick Wins" panel
  (`crates/anvil-tui/src/widgets/results_dashboard.rs:121-124`).
- Used by `surfaces/onboarding/init_complete.rs` indirectly via the dashboard
  wrapper.

### `ResultsDashboard`

**Purpose.** Post-init analysis dashboard for the `anvil init` / welcome-flow
`init_complete` surface. Renders framework, project root, file count, monorepo
flag, TS strictness, analysis summary, and embeds `QuickWinsPanel`. Adds a
Historical panel (commits / violations / top pattern) and a Next Steps panel.
`crates/anvil-tui/src/widgets/results_dashboard.rs:11-131`.

**Constructor.**
`ResultsDashboard::new(results, theme).block(block?).focused(bool)` where
`results: &InitAnalysisResults`. Stateful via `ResultsDashboardState` (unit
struct — placeholder).

**Data model.** `InitAnalysisResults` carries
`{ framework, project_root, size, file_count, monorepo, ts_strictness, analysis_summary: Vec<(String, usize)>, quick_wins: QuickWinsAnalysis, historical: HistoricalAnalysis, config_path, sample_files: Vec<String> }`
(`results_dashboard.rs:19-32`).

**Render contract.** `StatefulWidget`. Reserves seven vertical sections with
fixed heights — a 3-row Header, a 1-row "Analysis completed successfully" status
line in `status_ok`, a 6-row Metrics panel, an 8-row Quick Wins panel (delegated
to the embedded `QuickWinsPanel`), a 5-row Historical panel, a 3-row Next Steps
panel, and a 1-row footer ("[Enter] continue [q] close" in `disabled` style).
See `results_dashboard.rs:101-130`.

The Header is the only place an upstream widget is composed in directly:
`Header::new("Initial Analysis", self.theme).subtitle(project_root)`
(`results_dashboard.rs:112-114`).

**Notable invariants.** Snapshot stability — the dashboard renders into
fixed-row sections so snapshot pinning (currently
`onboarding/snapshots/anvil_tui__surfaces__onboarding__init_complete__render__tests__snapshot_init_complete_default.snap`,
see `tui-as-built.md` snapshot inventory) doesn't drift on data changes.

**Source.** `crates/anvil-tui/src/widgets/results_dashboard.rs`.

**Anvil consumers.**

- `crates/anvil-tui/src/surfaces/onboarding/init_complete.rs` — the post-init
  summary surface in the welcome / onboarding flow. **Only consumer** at time of
  review. Cross-reference `tui-as-built.md#onboarding`.

## Snapshot pinning

Two snapshot directories ship widget / chrome pins:

### Upstream — `crates/eddacraft-tui/src/snapshots/`

Exactly **one** pinned snapshot:
`eddacraft_tui__shell__tests__snapshot_shell_chrome.snap` — 60x10 buffer of the
shell chrome (`Anvil > Gate` header, `j/k navigate  enter expand  q quit`
footer + `eddacraft v<VERSION>` watermark). Pinned by
`crates/eddacraft-tui/src/shell.rs:130-151`.

Coverage gaps in upstream: the original 13 widgets all have Rust unit tests
(`#[cfg(test)] mod tests`) but **none** are pinned with insta snapshots. The
unit tests assert on individual cell symbols (e.g. `buf[(0, 0)].symbol() == "╔"`
for `Container::Primary`) rather than the full buffer. This is by design at the
upstream layer — the widgets are generic over `Theme`, and a buffer snapshot
would lock the `EddaCraftTheme` colour values into upstream test fixtures,
coupling upstream regression tests to the brand palette. The composition
snapshots live downstream in `anvil-tui` instead.

### Downstream — `crates/anvil-tui/**/snapshots/`

The TUI as-built (`tui-as-built.md#snapshot-infrastructure`) covers the 41
surface-level pins. Two are widget-specific:

- `anvil_tui__widgets__results_dashboard__tests__renders_with_full_data` /
  `…__renders_with_minimal_data` — the unit tests in
  `crates/anvil-tui/src/widgets/results_dashboard.rs:243-310` assert on
  individual cell symbols rather than the full buffer, matching the upstream
  pattern.
- `quick_wins_panel` tests assert on individual cells too
  (`crates/anvil-tui/src/widgets/quick_wins_panel.rs:202-235`).

### How upstream and downstream snapshots compose

The composition is one-way: downstream surface snapshots pin the **whole
buffer** including widget output, so any upstream widget regression that changes
a cell or style would surface as a downstream snapshot diff. The canonical chain
is:

1. Surface uses `eddacraft_tui::test_utils::snapshot::buffer_to_string`
   (`crates/eddacraft-tui/src/test_utils.rs:54-66`, re-exported via
   `crates/anvil-tui/src/test_utils.rs`) to serialise the rendered buffer.
2. `buffer_to_string` includes both the cell symbol and a style annotation
   (`<symbol>[fg:…,bg:…,bold,…]`), so colour or modifier swaps are caught even
   when symbols don't change.
3. `insta::assert_snapshot!` compares against the pinned `.snap`.

Cells with no styling emit only the symbol — keeps diffs noise-free.

## Cross-cutting concerns

### Determinism in rendering

Same data + same theme + same terminal size → same output. Required for snapshot
stability. Concrete consequences for widgets:

- The `Theme` trait returns `Color` values directly — no env-var lookup, no
  time-of-day toggle, no random gradient. The `theme_colours_are_distinct` test
  (`crates/eddacraft-tui/src/theme/eddacraft.rs:62-82`) pins this.
- `KeyHandler::map` is a `match` over `KeyEvent` — no internal state, no side
  effects. Same key → same action.
- The `parallel_progress` widget uses `Instant::now()`-derived elapsed values
  (`parallel_progress.rs:55, 97`), which means snapshot tests that exercise it
  must inject fixed `Instant`s. This is a known pattern for animated widgets —
  the tests use deterministic timestamps.
- Spinner frames advance via explicit `tick()` calls rather than wall-clock —
  snapshot tests pin a specific `frame: usize` value (`spinner.rs:46`).

### Theme as the only style source

Surfaces should not construct ad-hoc `Style` from `Color::Rgb(…)` or any
`Color::<NamedColour>` literal — every styling decision should pull through the
`Theme` trait. Verification: at time of review, a case-sensitive grep of
`crates/anvil-tui/src` for `Color::` returns **zero** hits. The only colour
values flowing through anvil-tui are the ones the trait returns. Theme drift is
not present.

(One nuance: many surfaces construct `Style::default().fg(theme.muted())` inline
rather than calling `theme.disabled()`. This produces the same foreground colour
but loses the trait's intent semantics, and it can diverge if a downstream theme
overrides `disabled()` without overriding `muted()`. See Known gaps G-04.)

### Zoom controls (`v0.5.1-beta`)

Three surfaces ship zoom — watch, status, audit. None of the upstream widgets
implement zoom themselves; zoom is a per-surface render-time collapse from grid
to single-panel. The keybinding pattern is:

- `z` toggles zoom on the currently focused panel.
- `esc` exits zoom on first press, navigates back on second.

The implementation lives in surface render code (e.g. `watch/render.rs:25-31`)
and surface state (`zoomed: bool`). The widget layer is unaware. See
`tui-as-built.md#zoom-controls` and `RELEASE-PLAN.md` ll. 16 for the
release-history pin.

### Mouse / unicode support

The `KeyHandler::map` accepts only `crossterm::event::KeyEvent` — mouse events
are not part of the action vocabulary. The CLI runner only polls `Event::Key`
(`crates/anvil-cli/src/tui.rs:185-244` — `tui-as-built.md` deep dive). Mouse
support is **not implemented** at the widget or surface layer.

Unicode: most widgets emit unicode glyphs freely (`spinner` braille,
`status_badge` geometric shapes, `parallel_progress` 1/8th block characters,
`divider` `━` / `─`). Two intentional ASCII-only zones:

- The watch action footer (`watch/render.rs:53-68`) uses `[*]` / `[x]` / `[!]`
  glyphs because Windows legacy code-pages and CI log captures may not handle
  wider unicode and the watch dashboard is the demo path.
- The `QuickWinsPanel` progress bar uses `#` / `-` characters
  (`quick_wins_panel.rs:191`) for the same first-run-terminal robustness
  rationale.

There is no documented graceful-degradation matrix for legacy or limited-unicode
terminals — see Known gaps G-03.

## Known gaps

### G-01: `editor` widget added post-extraction; no archive parity

The live `eddacraft-tui` (path crate, v0.3.0) includes
`crates/eddacraft-tui/src/widgets/editor.rs` (1005 lines), but the
`anvil-archive/eddacraft-tui-local/` fork does **not** contain an `editor.rs`
file — confirmed via `find` over the archive tree. The TUIEXTRACT module is
marked Complete (`plans/archive/modules/eddacraft-tui-shared.aps.md:10-12`), but
the archive snapshot pre-dates the editor's introduction. Future readers diffing
the archive against the live crate will see this as a real divergence, not just
whitespace drift.

**Risk:** Low — the live path crate is the source of truth; the archive is not
built. **Fix:** when the archive is next pruned (it's read-only historical
reference now), this gap closes naturally. No tracked work item.

### G-02: Several upstream widgets are unconsumed in `anvil-tui`

The consumption picture has partially inverted since the 2026-05-07 review: the
dashboard surface family now consumes `Container` (21 sites, e.g.
`crates/anvil-tui/src/surfaces/plan_dashboard/render.rs:105`), `DataTable` (4
sites, e.g. `crates/anvil-tui/src/surfaces/dashboard/architecture.rs:217`), and
`StatusBadge` (3 sites, e.g.
`crates/anvil-tui/src/dashboard_catalog/gate_result.rs:56`), alongside the
pre-existing consumers: `Editor` / `EditorState` (tutorial fix), `TextInput` /
`TextInputState` (init wizard, anvil-new wizard), `Header` (results dashboard).

The still-unconsumed set is `Divider`, `LogPanel`, `ParallelProgress`, `Select`,
`Spinner`, `StatusBar`, `Confirm`, `ProgressBar`, plus the newer `Modal` /
`Toast` / `Tree` / `HelpBar` / `Overlay` / `BigBanner` / `ImagePane` /
`Pretext`. This is partly intentional (the upstream library targets
`eddacraft/eddacraft` projects more broadly than just anvil) and partly
opportunity — surfaces that build their own list / progress-bar / status-bar
equivalents inline are candidates for refactor onto the upstream widget. None of
the unused widgets are deprecated.

**Risk:** Low — zero-cost in the binary because Rust dead-code-eliminates unused
trait method calls per crate. **Fix:** when surfaces are next refactored (e.g.
as part of TUIDASH json-render rollup,
`plans/archive/modules/tui-dashboard-render.aps.md`), reach for the upstream
widget first. No tracked work item.

### G-03: No documented unicode / terminal-compat matrix for widgets

The widget catalogue uses unicode freely (braille spinners, geometric status
badges, 1/8th block progress bars, double / rounded / heavy borders in
`Container`). Only two zones are explicitly ASCII-only — the watch action footer
and the `QuickWinsPanel` progress bar. Other widgets (`status_badge` ◆ ✖ ◈ ◇ ●
○, `spinner` ⠋⠙⠹…, `parallel_progress` ▏▎▍▌▋▊▉█) assume the terminal can render
the relevant unicode planes.

There is no tested-terminals matrix, no fallback for Windows legacy console, and
no automated graceful-degradation. The `compat::validate_minimum_size` function
(`crates/eddacraft-tui/src/compat.rs:27-50`) only checks dimensions, not unicode
capability.

**Risk:** Medium for users on legacy Windows terminals or limited CI
environments — `status_badge` / `spinner` / `parallel_progress` would display as
`?` glyphs. The fallback path is "use a modern terminal." **Fix:** ADR plus a
tested-terminals matrix; possible per-widget `ascii_only(bool)` toggle in a
future release. Cross-link `tui-as-built.md#G-06` (the same gap surfaced at the
surface level).

### G-04: Surfaces construct ad-hoc `Style::default().fg(theme.<role>())`

instead of calling theme helper methods

A grep of `crates/anvil-tui/src` for `Style::default()` returns hits in
shell.rs, audit/render.rs, and several other surfaces. Each one threads a theme
accessor (`theme.muted()`, `theme.error()`, `theme.fg()`, etc.) so zero
hardcoded colours leak in — but the construction style bypasses the trait's
derived-style helpers (`disabled()`, `status_error()`, `base()`, etc.). The
semantic intent is intact ("muted text"), but the binding to the trait is via
the colour primitive rather than the role helper. Concretely, if a downstream
theme overrode `disabled()` to add an italic modifier without changing
`muted()`, those surfaces would not pick up the modifier.

**Risk:** Low for `EddaCraftTheme` (the only ship-time theme). Could become
medium if a second theme palette ever lands. **Fix:** non-issue unless we add
theme-override flexibility. No tracked work item.

### G-05: `editor` and `text_input` lack pinned snapshot tests

Both upstream widgets have unit tests that assert on individual cells
(`text_input.rs` tests at lines 200-291 — character-boundary safety; `editor.rs`
tests — large suite over the 1005-line file). Neither pins a full-buffer
snapshot. The downstream consumers (init wizard, anvil-new wizard, tutorial fix)
**do** pin full-surface snapshots that include the widget output, so the widget
output is implicitly covered — but a widget-level regression (e.g. cursor-cell
`REVERSED` modifier dropped) would not surface as a widget test failure, only as
a surface snapshot diff.

**Risk:** Low — the surface-level snapshots catch regressions, just one layer
further out than ideal. **Fix:** add insta snapshots to the upstream widget
tests as part of the next eddacraft-tui release. No tracked work item — the
upstream crate is on its own release cadence.

### G-06: `confirm` widget is defined but unused

`Confirm` / `ConfirmState` is the only upstream widget for which a clear
consumer should exist (the `surfaces/onboarding/hooks.rs` flow uses a
`HooksPhase::Confirm` state and prompts the user for yes/no), but the hooks
surface implements its own confirm-prompt rendering inline rather than reaching
for the upstream widget. This isn't a bug — the hooks surface has
surface-specific layout demands — but it is the most suspicious-looking
inventory orphan in the catalogue.

**Risk:** Low. **Fix:** evaluate during the next onboarding refactor; if the
inline confirm prompt has diverged from the widget contract, decide whether to
inline-the-widget-back-into-the-surface or extend the widget to support the
surface needs. No tracked work item.

### G-07: new widgets lack deep dives; original pins predate the re-pin

The 9 widgets added since the 2026-05-07 full review (`big_banner`,
`data_table`, `help_bar`, `image_pane`, `modal`, `overlay`, `pretext`, `toast`,
`tree`) are recorded only in the summary table in §Upstream widgets — the
2026-06-10 targeted pass deliberately did not author deep-dive subsections for
them. In addition, the line pins inside the original 13 deep-dive subsections
were taken against the published `0.1.0` crate and predate the path-crate re-pin
to `crates/eddacraft-tui` v0.3.0; they have not been re-verified line-by-line.
Pins corrected in the targeted pass (crate resolution, catalogue declarations,
prelude, theme contract, keyboard exports, spinner presets, snapshot counts,
container / status_badge consumers) were verified against main `45dd1047a`.

**Risk:** Low — the prose contracts still describe the live widgets; only
line-number drift is at stake. **Fix:** both items are owed to the next full
review.

## Source references

### Upstream — `crates/eddacraft-tui` v0.3.0 (in-monorepo path crate)

| File                                                                                         | Role                                                                       |
| -------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------- |
| `crates/eddacraft-tui/Cargo.toml`                                                            | Package manifest; depends on crossterm, ratatui, unicode-width             |
| `crates/eddacraft-tui/src/lib.rs`                                                            | Module root; `prelude` re-exports                                          |
| `crates/eddacraft-tui/src/compat.rs`                                                         | `TerminalInfo`, `detect_terminal`, `validate_minimum_size` (80x24 minimum) |
| `crates/eddacraft-tui/src/keyboard/mod.rs`                                                   | Re-exports `Action`, `Binding`, `KeyHandler`                               |
| `crates/eddacraft-tui/src/keyboard/handler.rs`                                               | `Action` enum (16 variants, `#[non_exhaustive]`), `KeyHandler::map`        |
| `crates/eddacraft-tui/src/shell.rs`                                                          | `render_shell` chrome (header + footer + watermark)                        |
| `crates/eddacraft-tui/src/surface.rs`                                                        | `Surface` trait                                                            |
| `crates/eddacraft-tui/src/test_utils.rs`                                                     | `snapshot::buffer_to_string`, `style_annotation`                           |
| `crates/eddacraft-tui/src/theme/mod.rs`                                                      | Re-exports `Theme`, `EddaCraftTheme`                                       |
| `crates/eddacraft-tui/src/theme/traits.rs`                                                   | `Theme` trait (8 colour hooks + 10 derived styles + `Role`/`role_style`)   |
| `crates/eddacraft-tui/src/theme/eddacraft.rs`                                                | `EddaCraftTheme` brand palette                                             |
| `crates/eddacraft-tui/src/widgets/mod.rs`                                                    | Widget module declarations + internal `render_block` helper                |
| `crates/eddacraft-tui/src/widgets/confirm.rs`                                                | `Confirm`, `ConfirmState`                                                  |
| `crates/eddacraft-tui/src/widgets/container.rs`                                              | `Container`, `ContainerVariant`                                            |
| `crates/eddacraft-tui/src/widgets/divider.rs`                                                | `Divider`, `DividerVariant`                                                |
| `crates/eddacraft-tui/src/widgets/editor.rs`                                                 | `Editor`, `EditorState` (multi-line, read-only context)                    |
| `crates/eddacraft-tui/src/widgets/header.rs`                                                 | `Header` (separator + uppercase title + subtitle)                          |
| `crates/eddacraft-tui/src/widgets/log_panel.rs`                                              | `LogPanel`, `LogPanelState`, `LogEntry`, `LogLevel`, `LogFilter`           |
| `crates/eddacraft-tui/src/widgets/parallel_progress.rs`                                      | `ParallelProgress`, `CheckProgress`, `CheckStatus`, ETA helpers            |
| `crates/eddacraft-tui/src/widgets/progress_bar.rs`                                           | `ProgressBar`, `ProgressBarState`                                          |
| `crates/eddacraft-tui/src/widgets/select.rs`                                                 | `Select`, `SelectItem`, `SelectState`                                      |
| `crates/eddacraft-tui/src/widgets/spinner.rs`                                                | `Spinner`, `SpinnerState` (10-frame braille)                               |
| `crates/eddacraft-tui/src/widgets/status_badge.rs`                                           | `StatusBadge`, `BadgeStatus` (6 statuses)                                  |
| `crates/eddacraft-tui/src/widgets/status_bar.rs`                                             | `StatusBar`, `StatusItem`, `StatusKind`                                    |
| `crates/eddacraft-tui/src/widgets/text_input.rs`                                             | `TextInput`, `TextInputState` (UTF-8 cursor, char-boundary)                |
| `crates/eddacraft-tui/src/snapshots/eddacraft_tui__shell__tests__snapshot_shell_chrome.snap` | Shell chrome 60x10 pin                                                     |

### Downstream — `crates/anvil-tui` widgets

| File                                                | Role                                                                                                                 |
| --------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------- |
| `crates/anvil-tui/Cargo.toml`                       | Consumes `eddacraft-tui = { workspace = true, features = ["json-render"] }` (`:16`; dev-dep `:27` adds `test-utils`) |
| `crates/anvil-tui/src/widgets/mod.rs`               | Module root (2 lines: re-exports `quick_wins_panel`, `results_dashboard`)                                            |
| `crates/anvil-tui/src/widgets/quick_wins_panel.rs`  | `QuickWinsPanel`, `QuickWinsPanelState`, `QuickWinsAnalysis`, `BatchGroup`, `QuickWinType`                           |
| `crates/anvil-tui/src/widgets/results_dashboard.rs` | `ResultsDashboard`, `ResultsDashboardState`, `InitAnalysisResults`, `HistoricalAnalysis`                             |

### Live-crate consumers (selected)

| File                                                          | Use                                                           |
| ------------------------------------------------------------- | ------------------------------------------------------------- |
| `crates/anvil-cli/src/tui.rs:11-12, 39, 41`                   | Owns the `EddaCraftTheme` instance; threads `KeyHandler::map` |
| `crates/anvil-tui/src/shell.rs`                               | Re-uses upstream `Theme` trait for shell chrome rendering     |
| `crates/anvil-tui/src/surfaces/init/mod.rs:4, 142, 170`       | `TextInputState` consumer (init directory step)               |
| `crates/anvil-tui/src/surfaces/wizard/mod.rs:4, 80, 107`      | `TextInputState` consumer (anvil-new project name step)       |
| `crates/anvil-tui/src/surfaces/tutorial/fix.rs:3, 54`         | `EditorState` consumer (tutorial fix surface)                 |
| `crates/anvil-tui/src/surfaces/tutorial/fix_render.rs:2, 163` | `Editor` consumer (tutorial fix render)                       |
| `crates/anvil-tui/src/surfaces/onboarding/init_complete.rs`   | `ResultsDashboard` consumer (post-init flow)                  |

## Related docs

- [`tui-as-built.md`](./tui-as-built.md) — surface-level consumer view; surface
  inventory; snapshot infrastructure overview; watch / tutorial / doctor /
  status / audit / gate deep dives.
- [`tutorial-as-built.md`](./tutorial-as-built.md) — heavy widget consumer
  (`EditorState` for the in-tutorial fix surface).
- [`activation-as-built.md`](./activation-as-built.md) — vocabulary
  cross-referenced from the tutorial ProtectionLoop; not a widget consumer
  itself, but the tutorial widget consumer pins the activation-state literals in
  copy.
- `RELEASE-PLAN.md` — `v0.5.1-beta` zoom-controls history (zoom is a
  surface-level affordance, not a widget primitive).
- `plans/archive/modules/eddacraft-tui-shared.aps.md` — TUIEXTRACT (Complete
  7/7); the extraction history that made `eddacraft-tui` a separately published
  crate.
- `plans/archive/modules/ratatui-tui.aps.md` — RATS (Complete 7/7); the Ratatui
  surfaces module that owns this widget infrastructure.
