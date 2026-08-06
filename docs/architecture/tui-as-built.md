# anvil-tui — As-Built

| Type     | Authority | Owner | Status | Freshness                                                                                                                                                                                                                                                                                    |
| -------- | --------- | ----- | ------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| As-built | Derived   | RATS  | Live   | Last reviewed 2026-07-02 (targeted delta review: CLI-runner cross-ref re-anchor, snapshot count) against main `d1fded280`; prior delta review 2026-06-10 (TUIDASH/TDASH dashboard family, plan_dashboard, snapshot counts) against `a1c41e284`; full review 2026-05-07 against `v0.6.0-beta` |

| Upstream                                   | Downstream                                                                                                       |
| ------------------------------------------ | ---------------------------------------------------------------------------------------------------------------- |
| `crates/anvil-tui`, `crates/eddacraft-tui` | CLI commands (watch, status, audit, doctor, tutorial, welcome, init, gate), widget catalogue (RATS / TUIEXTRACT) |

> **Status:** Live (beta) **Last reviewed:** 2026-07-02 (targeted delta review:
> CLI-runner cross-ref re-anchor, snapshot count) against main `d1fded280`;
> prior delta review 2026-06-10 (TUIDASH/TDASH dashboard family, plan_dashboard,
> snapshot counts) against `a1c41e284`; full review 2026-05-07 against
> `v0.6.0-beta` slate (HEAD `d223b8d9`) **Crate / location:** `crates/anvil-tui`
> **Module owner (APS):** RATS (Ratatui surfaces — complete, archived to
> `plans/archive/modules/ratatui-tui.aps.md`), PORT (Ink-to-Ratatui port —
> complete, archived to `plans/archive/modules/ink-to-ratatui-port.aps.md`),
> TUIDASH (`plans/archive/modules/tui-dashboard-render.aps.md`, Complete 13/13 —
> json-render engine + dashboard surface family shipped 2026-06-02,
> Released/Shipped in `v0.8.0-beta`), TDASH (native per-domain dashboards —
> complete, archived to `plans/archive/modules/native-tui-dashboards.aps.md`),
> APSCAN (`plan_dashboard` — complete, archived to
> `plans/archive/modules/aps-canonical-alignment.aps.md`) **Used by:** every
> interactive `anvil` CLI command that renders a TUI — `anvil watch`,
> `anvil status`, `anvil audit`, `anvil doctor`, `anvil tutorial`,
> `anvil welcome`, `anvil init` wizard, `anvil new` (template wizard),
> `anvil gate` interactive mode, `anvil dashboard` (picker + spec / native
> dashboards), `anvil plan dashboard`

## Overview

`anvil-tui` is the Ratatui-based terminal UI surface library that backs every
interactive `anvil` command. It owns the surface inventory (status, audit,
doctor, gate, watch, tutorial, welcome, wizard, init, browser, onboarding,
dashboard, plan_dashboard) and re-exports the `Surface` trait, terminal-compat
helpers, and snapshot utilities that come from the upstream `eddacraft-tui`
crate (`crates/anvil-tui/src/lib.rs:1-18`).

The crate is intentionally narrow: rendering and state, no I/O. The terminal
session, event poll loop, raw-mode setup, and shell chrome are owned by
`crates/anvil-cli/src/tui.rs` (the CLI side `run_surface` / `run_watch` /
`run_tutorial` runners — see `crates/anvil-cli/src/tui.rs:111-453`). Each
surface implements `Surface::render`, `Surface::handle_key`, and
`Surface::should_quit` / `should_back`; the runner spins the loop, polls
crossterm events, and dispatches keys via `eddacraft_tui::keyboard::KeyHandler`.
Inside `anvil-tui` itself, the only top-level "app" type is `TuiApp`
(`crates/anvil-tui/src/app.rs:34-77`), which is a thin lifecycle wrapper around
the watch surface for the `anvil watch --tui=ratatui` migration path.

The crate ships with style-aware insta snapshots for every surface. The snapshot
infrastructure is the load-bearing internal pattern: render a surface into a
`TestBackend` of a fixed size and compare cell-and-style serialisation against a
pinned `.snap` (`crates/anvil-tui/src/test_utils.rs` →
`eddacraft_tui::test_utils::snapshot::buffer_to_string`,
`crates/eddacraft-tui/src/test_utils.rs:54-66`).

## Architecture diagram

```text
                 ┌──────────────────────────────────────┐
   anvil <cmd>   │ crates/anvil-cli/src/commands/<cmd>  │
                 │  - builds surface state              │
                 │  - chooses runner                    │
                 └─────────────────┬────────────────────┘
                                   │
                                   ▼
                 ┌──────────────────────────────────────┐
                 │ crates/anvil-cli/src/tui.rs          │
                 │  run_surface  / run_surface_in       │
                 │  run_watch    / run_watch_in         │
                 │  run_tutorial / run_watch_demo       │
                 │   - enable raw mode                  │
                 │   - alternate screen                 │
                 │   - poll crossterm events            │
                 │   - call render_shell + S::render    │
                 └─────────────────┬────────────────────┘
                                   │ Surface trait
                                   ▼
   ┌────────────────────────────────────────────────────┐
   │ anvil-tui surfaces  (per-screen state + render)    │
   │   welcome / wizard / init / onboarding / tutorial  │
   │   watch / status / audit / doctor / gate / browser │
   │   dashboard / plan_dashboard                       │
   └─────────────┬───────────────────────────────┬──────┘
                 │ widgets                       │
                 ▼                               │
   ┌──────────────────────────┐                  │
   │ anvil-tui widgets        │                  │
   │  quick_wins_panel        │                  │
   │  results_dashboard       │                  │
   └─────────┬────────────────┘                  │
             │                                   │
             ▼                                   │
   ┌──────────────────────────┐                  │
   │ eddacraft-tui            │                  │
   │  Surface trait, theme,   │                  │
   │  KeyHandler, widgets,    │                  │
   │  snapshot test utils     │                  │
   └──────────────────────────┘                  │
                                                 │
                                                 │  EngineEvent (kernel)
                              ┌──────────────────┴────────────────┐
                              │ surfaces/watch/event_adapter.rs   │
                              │ (the only surface that consumes a │
                              │  live mpsc::Receiver<EngineEvent>)│
                              └───────────────────────────────────┘
```

The synchronous-data path applies to every surface except watch: the CLI command
builds a state object once, hands it to `run_surface`, and the user exits with
`Quit` or `Back`. The watch surface is the side-arm that consumes kernel events
live, drains a channel each loop iteration, and re-renders only when state is
dirty.

## Crate layout

```text
crates/anvil-tui/
├── Cargo.toml                      # eddacraft-anvil-tui (lib name: anvil_tui)
├── src/
│   ├── lib.rs                      # 18 lines; module root + sanitize re-export
│   ├── app.rs                      # TuiApp + TuiAppConfig (watch lifecycle)
│   ├── shell.rs                    # render_shell, inset_content, OUTER_*_MARGIN
│   ├── surface.rs                  # `pub use eddacraft_tui::surface::Surface;`
│   ├── compat.rs                   # `pub use eddacraft_tui::compat::*;`
│   ├── dashboard_catalog/          # domain components + GATE_SUMMARY_SPEC asset
│   ├── dashboard_context.rs        # $data context from .anvil/ state (TUIDASH-008)
│   ├── fileio.rs                   # read_capped bounded .anvil/ reads
│   ├── migration.rs                # TuiBackend (Ink|Ratatui), select_backend
│   ├── test_utils.rs               # `pub use eddacraft_tui::test_utils::snapshot;`
│   ├── snapshots/                  # crate-level shell chrome snapshot
│   ├── widgets/
│   │   ├── mod.rs
│   │   ├── quick_wins_panel.rs     # batched warning suppressions panel
│   │   └── results_dashboard.rs    # post-init analysis dashboard
│   └── surfaces/
│       ├── mod.rs                  # surface module declarations
│       ├── fix_request.rs          # FixRequest enum (cross-surface)
│       ├── notifications.rs        # NotificationSource trait
│       ├── update_hint.rs          # UpdateHint shared DTO (DISTRIB-002)
│       ├── audit/      browser/    dashboard/  doctor/   gate/
│       ├── init/       onboarding/ plan_dashboard/       status/
│       └── tutorial/   watch/      welcome/    wizard/
```

The surface modules use a consistent shape: `mod.rs` holds state types, the
`Surface` impl, and `handle_key`; `render.rs` holds the Ratatui rendering;
`snapshots/` holds insta `.snap` files. Larger surfaces split the renderer
across files (`tutorial/discovery_render.rs`, `tutorial/fix_render.rs`,
`tutorial/watch_demo_render.rs`, `onboarding/welcome_render.rs`,
`onboarding/hooks_render.rs`).

## App loop and surface dispatcher

Inside the crate the only "app" type is `TuiApp`
(`crates/anvil-tui/src/app.rs:34-77`). It wires:

- `TuiBackend` selection (Ink rejected with `BackendNotSupported` —
  `app.rs:51-55`; Ink lives in the Node.js process).
- Terminal size validation via `validate_minimum_size` re-exported from
  `eddacraft-tui::compat` (`compat.rs:1-3`, `app.rs:45-49`).
- A `WatchEventAdapter` and `WatchState` paired with an
  `mpsc::Receiver<EngineEvent>`.
- `TuiApp::drain_events` — a non-blocking `try_recv` loop that pushes kernel
  events into the watch state (`app.rs:80-84`).

The actual event-poll / draw loop is on the CLI side
(`crates/anvil-cli/src/tui.rs:185-244`; the caller acquires the terminal via
`TerminalGuard::enter()` first):

1. `TerminalGuard::enter()` — `terminal::enable_raw_mode()`,
   `EnterAlternateScreen`, and the panic-restore hook.
2. Call `render_shell(frame, area, surface_name, help_text, theme)` which
   reserves the top header (`Anvil > <surface>`) and the footer (help text +
   `eddacraft v<VERSION>` watermark) and returns the inner content `Rect`
   (`crates/anvil-tui/src/shell.rs:52-109`).
3. Call `state.render(frame, content, theme)` — the surface paints into the
   inner rect.
4. `event::poll(Duration::from_millis(100))` and dispatch via
   `KeyHandler::map(key)` to `Surface::handle_key`. `Resize` events flag dirty
   so the next iteration redraws.
5. After every key, check `should_quit` / `should_back` and tear down.

`render_shell` also pads each surface with `OUTER_H_MARGIN = 2` columns and
`OUTER_TOP_MARGIN = 1` row of breathing room — every onboarding / tutorial
surface routes its incoming `area` through `inset_content` so first-run flows
have a consistent gutter (`shell.rs:14-47`).

The dispatcher pattern for surface transitions (welcome → tutorial → wizard;
welcome → gate / audit / doctor sub-surfaces) is owned on the CLI side via
`run_surface_in` against a single shared `Terminal`, so transitions never tear
the alternate screen (`crates/anvil-cli/src/commands/welcome.rs:200-1030`). This
lets the welcome hub launch any sub-surface without resetting the user's
terminal — the alt-screen stays alive across `run_surface_in` calls.

## Surfaces

The thirteen primary surfaces, grouped by lifecycle. Each subsection lists the
purpose, the CLI command that mounts it, the key state type, and any notable
invariants. Detail goes into the deep-dive sections below for watch, tutorial,
welcome / wizard / onboarding, doctor, status, audit, gate.

### audit

`anvil audit` interactive mode. Renders a four-panel layout (Project / Issues /
Historical / Next Steps) with `j/k` navigation, expand-on-enter for issue
detail, and an `f`-to-fix path that emits `FixRequest::AuditConsoleStatement`
for the auto-fixable `console.log` / `console.error` removal flow
(`crates/anvil-tui/src/surfaces/audit/mod.rs:70-90`,
`crates/anvil-tui/src/surfaces/audit/render.rs:1-50`). Severity vocabulary is
`CRIT / HIGH / MED / LOW / INFO` (`mod.rs:48-58`). Zoom support landed in
`v0.5.1-beta` for dense-output inspection. The `.env` / env-template skip filter
that keeps audit aligned with the CLI behaviour lives upstream in
`crates/anvil-cli/src/commands/audit.rs:186-212`, not in the TUI.

### browser

Template browser for the `anvil new` flow. Three views — Categories → Templates
→ Detail — with forward / backward navigation
(`crates/anvil-tui/src/surfaces/browser/mod.rs:6-31`). Carries a search term and
search mode (`/`-to-search), and exits with `chosen: Option<String>` holding a
template id. The post-template-selection path hands the chosen id to the wizard
surface (`crates/anvil-cli/src/commands/new.rs:62`).

### dashboard

The `anvil dashboard` family
(`crates/anvil-tui/src/surfaces/dashboard/mod.rs:13-17`). Five sub-surfaces:
`list` — a two-pane picker, dashboards on the left and a live json-render
mini-preview of the highlighted spec on the right; native dashboards show a
description card instead, and `enter` records the chosen name for the CLI to
open full-screen (`dashboard/list.rs:1-30`, TUIDASH-012). `spec` — renders a
saved json-render dashboard spec (`.anvil/dashboards/<name>.json`) through the
eddacraft-tui engine; the surface owns `$data` binding against `.anvil/` state
and `r`-to-refresh re-binding without re-reading the spec
(`dashboard/spec.rs:1-60`, TUIDASH-009). `architecture` / `drift` /
`suppressions` — fixed native per-domain dashboards (TDASH). Spec rendering uses
the Anvil-domain registry: the generic base components plus `GateResultCard` /
`WarningList` / `DriftIndicator` / `PlanCard` / `SuppressionRequest` /
`EvidenceEntry` (`crates/anvil-tui/src/dashboard_catalog/mod.rs:42-63`). The
gate-summary spec ships as a crate asset — `GATE_SUMMARY_SPEC`
(`dashboard_catalog/mod.rs:38-39`, an `include_str!` of
`assets/dashboards/gate-summary.dashboard.json`) — seeded by `anvil init` and
served as an embedded zero-write fallback when no saved spec shadows it
(UJ-009).

### doctor

`anvil doctor` interactive. Nine diagnostic checks defined in
`crates/anvil-cli/src/commands/doctor.rs::collect_checks` (git available, git
repo, config exists, config valid, `.anvil/` dir, `.anvil/` writable, plans dir,
hooks installed, registry patterns compile). Four are auto-fixable; press `f` on
a non-passing fixable check to emit `FixRequest::DoctorCheck { index }`, which
the host applies via `apply_fix_request` and re-enters the TUI with a
`FixOutcomeBanner` (`crates/anvil-tui/src/surfaces/doctor/mod.rs:108-178`,
`crates/anvil-cli/src/commands/doctor.rs:45`). Layered display: list view +
expand-on-select detail with the `Remediation` block (`summary`, optional
`command`, optional `doc_url`) rendered structurally instead of from prose
(`mod.rs:40-56`). The help text changes when the selected check is fixable —
adds `f fix` (`mod.rs:186-196`).

### gate

`anvil gate` interactive. Filter ribbon (`a/p/f/s/w` for All / Passed / Failed /
Skipped / Warning), `/` search, `n` / `N` to jump to next / previous failure,
`enter` to expand a check (`crates/anvil-tui/src/surfaces/gate/mod.rs:60-115`).
The `event_adapter.rs` sub-module is the gate-result-from-events bridge for
cases where gate runs stream their results in (`gate/event_adapter.rs`).
Cross-link [`checks-as-built.md`](./checks-as-built.md) §"CLI surfaces" for
upstream contract.

### init

`anvil init` wizard. Five-step progression — Mode (New / Existing / Minimal),
Format (YAML / JSON / TOML; YAML default), Directory (text input), Checks
(toggle list), Summary (`crates/anvil-tui/src/surfaces/init/mod.rs:6-58`).
Default config format is YAML (`mod.rs:101`). Distinct from the `wizard`
surface, which scaffolds a project from a template; `init` writes
`.anvil.{yaml,json,toml}` against an existing repo. The post-init analysis
result UI is the `widgets/results_dashboard.rs` widget rendered by the welcome
hub flow.

### onboarding

First-run flow. Four sub-surfaces — `welcome` (the three-choice selector: Set up
/ Tutorial / Skip — `onboarding/welcome.rs:5-40`), `hooks` (1331-line hook
manager — Husky / Lefthook / pre-commit / Git config hooks detection,
`onboarding/hooks.rs:30-60`), `init_complete` (post-init summary), `complete`
(end-of-onboarding summary). Each implements `Surface` and is launched via
`run_surface_in` from the welcome command. The `config_exists_in` helper
(`onboarding/mod.rs:31-35`) gates whether onboarding runs the init step or skips
straight to the menu — zero-byte `.anvilrc` is treated as absent
(`onboarding/mod.rs:56-65`).

### plan_dashboard

The APS / plan rollup surface (APSCAN — complete, archived), mounted by
`anvil plan dashboard` (`crates/anvil-cli/src/commands/plan.rs:75`).
`PlanDashboardSnapshot` carries module rows, work-item rows, and warning rows
with branch / sha provenance; on top of the snapshot the surface adds a filter
mode, detail and help toggles, and a rescan request flag
(`crates/anvil-tui/src/surfaces/plan_dashboard/mod.rs:11-72`).

### status

`anvil status` interactive. Three panels — Hooks / Profile / Results — with
`tab` / shift-tab to switch panels and `j/k` to navigate items inside the
focused panel (`crates/anvil-tui/src/surfaces/status/mod.rs:42-79`). Zoom
support (`z` toggles single-panel mode) landed in `v0.5.1-beta` to mirror the
watch surface affordance (`status/mod.rs:73-77`). The activation diagnostic
(`anvil status --verify`) is rendered by `crates/anvil-cli/src/activation/` not
this TUI surface; cross-link
[`activation-as-built.md`](./activation-as-built.md).

### tutorial

`anvil tutorial`. Five paths — `ProtectionLoop` (LAUNCH-014 default — see deep
dive), `Policy`, `Architecture`, `Drift`, `CI`
(`crates/anvil-tui/src/surfaces/tutorial/mod.rs:36-44`). Three phases —
`PathSelect`, `Running`, `Complete` (`mod.rs:88-92`). Each step can carry an
optional shell command, an optional verifier (`Verify::FileExists`,
`Verify::ExitCode`), an optional `watch_path` for live re-verification, and a
`watch_demo` flag that exits the loop and hands control to the watch demo
surface (`mod.rs:96-128`). Static-mode fallback when the kernel watcher is
unavailable disables command execution and sets a notice
(`STATIC_MODE_WATCHER_UNAVAILABLE`, `mod.rs:24-25`, `mod.rs:148-152`). The
progress file persists `completed_paths` across sessions (`mod.rs:153-156`).

### watch

The live-event surface. 2x2 grid (Status / Queue / History / Stats) with a 1-row
action footer when an `--action` outcome is present
(`crates/anvil-tui/src/surfaces/watch/render.rs:10-50`). Drains kernel
`EngineEvent`s through `WatchEventAdapter` each loop iteration. Zoom support
(`z`), grid traversal with edge-spillover (arrow keys move within the focused
list until the edge, then spill to the panel above or below the row), and a
`take_dirty()` paint gate keep the watch surface efficient
(`watch/mod.rs:271-341`). See deep dive below.

### welcome

The post-onboarding menu (also: post-LAUNCH-006, `start` is its own command and
no longer a clap alias of `welcome`). Seven options — Run gate / Watch / Audit /
Doctor / Tutorial / View docs / Restart onboarding
(`crates/anvil-tui/src/surfaces/welcome/mod.rs:7-50`). `enter` exits the surface
with the chosen option in `WelcomeState::chosen`; the welcome command in
`crates/anvil-cli/src/commands/welcome.rs:958-1030` pattern-matches on the
chosen variant and launches the sub-surface with `run_surface_in`.

### wizard

`anvil new` template wizard. Four steps — TemplateSelect / ProjectName /
Configure / Summary (`crates/anvil-tui/src/surfaces/wizard/mod.rs:14-43`). The
template list is loaded from the catalog and threaded in via `WizardState::new`.
Configure step toggles `enable_watch` and `enable_hooks` (`mod.rs:65-71`).

### Smaller surface modules

`surfaces/fix_request.rs` (15 lines) — declares `FixRequest`, the deterministic
cross-surface fix envelope. Three variants: `DoctorCheck { index }`,
`AntiPatternWarning { file, line, warning_id }`,
`AuditConsoleStatement { file, line }`. Surfaces emit a `FixRequest` and the
host (CLI command) applies it via
`services::interactive_fix::apply_fix_request`, re-entering the TUI with a
`FixOutcomeBanner` afterwards.

`surfaces/notifications.rs` (55 lines) — declares the `NotificationSource`
trait. Surfaces that carry user-facing notices (static-mode fallbacks, resume
hints, install errors) implement it so renderers, telemetry, and future daemon
subscribers consume notifications through a canonical `Notification` envelope
(`anvil_kernel_types::Notification`) instead of surface-specific wording. The
`watch` and `tutorial` surfaces are the current implementers
(`watch/mod.rs:344-369`, `tutorial/mod.rs` static-mode notice).

`surfaces/update_hint.rs` — the DISTRIB-002 "update available" hint as a shared
DTO, re-exported at `surfaces/mod.rs:18` and rendered identically by the status
and watch surfaces (`UpdateHint::render_line` keeps the wording from drifting
between the two).

## Watch dashboard (deep dive)

The watch surface is the only one that consumes kernel events live. It sits
behind `anvil watch --tui=ratatui` and the post-onboarding `Watch checks live`
welcome option. The shape of the watch dashboard is the canonical TUIDASH-009
inheritance seam — its `WatchStats` contract is the named contract the
json-render dashboard surface (now shipped — see the dashboard subsection above)
inherits.

### Data model

`WatchData` (`crates/anvil-tui/src/surfaces/watch/mod.rs:135-144`) carries:

- `status: WatchStatus` — `Idle | Running | Passing | Failing` (icon + label
  helpers at `mod.rs:34-52`).
- `queue: VecDeque<QueuedNotification>` — capped at `MAX_QUEUE_LEN = 200`
  (`watch/event_adapter.rs:11`).
- `history: Vec<RunHistory>` — capped at `MAX_HISTORY_LEN = 100`
  (`event_adapter.rs:13`).
- `stats: WatchStats` — total runs, pass rate (rolling, over `history.len()` not
  `total_runs`), avg duration ms, files watched (`mod.rs:73-78`,
  `event_adapter.rs:227-244`).
- `last_action: Option<ActionResultLine>` — the most recent `--action` outcome
  (LAUNCH-002), surfaced as a single-line footer below the grid. Single-writer
  invariant: only `WatchEventAdapter::handle_action_result` writes it
  (`mod.rs:80-114`).

### Event adapter (KERN-033 protocol consumer)

`WatchEventAdapter` (`watch/event_adapter.rs:18-246`) handles four event
payloads:

- `Progress { phase, current, total }` — flips status to `Running`; on
  completion (`current >= total`) sets `Passing` / `Failing` based on
  `violation_count + error_count` but does not record a history entry. The
  Snapshot is the authoritative end-of-cycle marker.
- `Snapshot { node_count, edge_count, files_watched }` — the end-of-cycle
  marker. Records a `RunHistory`, increments `total_runs`, recomputes pass rate
  and avg duration over the rolling history window, resets per-cycle counters
  (`event_adapter.rs:114-155`).
- `Violation { policy_id, file, symbol, message }` — increments violation count,
  sets status to `Failing`, pushes a `Notification::Finding` / `High` priority
  into the queue with `context.source = "watch"` (`event_adapter.rs:157-181`).
- `Error(ErrorPayload)` — increments error count, sets `Failing`, pushes a
  `Notification::Failure` / `High` priority (`event_adapter.rs:183-209`).

The contract is that every completed Progress sequence is followed by a Snapshot
— if a Snapshot never arrives (kernel crash, channel disconnect), the cycle's
history entry is lost. This is documented as acceptable
(`event_adapter.rs:36-41`).

### Render and interaction

`render::render` (`watch/render.rs:10-51`) splits the area for the optional
action footer first, then renders the 2x2 grid (or the focused panel fullscreen
when `state.zoomed`). Each panel uses `panel_block(title, focused, theme)` — a
focused panel uses `BorderType::Double` with the theme accent colour, an
unfocused panel uses `BorderType::Plain` with the muted border. Arrow-key
navigation has edge-spillover semantics: scrolling within the Queue or History
panel moves the selected item until you hit the edge of the list, at which point
a further press spills to the panel above or below in the row
(`watch/mod.rs:271-310`). The `z` key zooms the focused panel to fullscreen;
`esc` exits zoom on the first press, navigates back on the second
(`watch/mod.rs:321-334`).

Animation is driven by `animate::Once<f64, ...>` on `pass_rate` and
`avg_duration_ms` for smooth interpolation when Snapshot updates land
(`watch/mod.rs:12-23`, `200-225`). The render loop on the CLI side
(`crates/anvil-cli/src/tui.rs:455-553`) uses a `take_dirty()` gate so the
surface only paints when state has actually changed, with a 16 ms cap while
animating to avoid busy-spin.

### Action footer (LAUNCH-002)

When `data.last_action` is `Some`, render reserves a 1-row footer below the grid
(`watch/render.rs:69-96`). Three glyphs, ASCII-only:

- `[*] <action> (Xs)` — child exited 0 (theme success).
- `[x] <action> (Xs, exit N)` — child exited non-zero (theme error).
- `[!] <action> (<cause>)` — child did not run to a recorded exit; the cause is
  one of `spawn failed: <io>`, `cancelled`, `wait failed: <reason>`, rendered
  verbatim. This replaces the older "spawn failed: …" prefix that lied about
  cancellations and signal-kills (#1279 review).

ASCII-only glyphs are deliberate — Windows legacy code-pages and CI log captures
may not handle the broader Unicode the rest of the TUI uses, and the watch
dashboard is the demo path so a mojibake'd footer would be visible at exactly
the wrong moment (`watch/render.rs:53-68`).

### NotificationSource

`WatchState` implements `NotificationSource` (`watch/mod.rs:344-369`). The
implementation defensively backfills
`notification.context.source = Some("watch")` so any future producer path that
constructs `QueuedNotification` directly can't leak a `source: None` event
through this surface — the telemetry-stream contract requires
`correlation.source == notification.context.source`, and the test
`notification_source_backfills_source_context` (`watch/mod.rs:756-772`) pins
that invariant.

## Tutorial surface (deep dive)

The tutorial is the LAUNCH-014 ProtectionLoop default. Five steps in
`paths::protection_loop_steps` (`tutorial/paths.rs:107-136`):

1. _"Anvil's protection loop in 60 seconds"_ — narrative framing.
2. _"What we'll check"_ — the fixture (TypeScript with `@ts-ignore` and
   `: any`).
3. _"Run the check (simulated)"_ — fixture findings, no real scan.
4. _"What protection actually means here"_ — the activation vocabulary
   (`protecting`, `ready_restart_required`, `watching`, `needs_action`,
   `unsupported`).
5. _"Activate in this repo"_ — runs `anvil start --verify` (read-only).

Two test pins enforce the copy invariants:

- `protection_loop_copy_uses_activation_state_vocabulary`
  (`tutorial/mod.rs:882-909`) — the path body must reference all five activation
  state literals so users recognise them when `anvil status --verify` prints
  one. This protects the cross-surface vocabulary contract with
  [`activation-as-built.md`](./activation-as-built.md).
- `protection_loop_copy_does_not_claim_pre_write_protection`
  (`tutorial/mod.rs:912-945`) — rejects phrases like "you are now protected",
  "your repo is protected", "pre-write validation enabled". The tutorial does
  not have activation evidence, so its copy must not promise pre-write
  protection or call the user's repo "protected".

The four legacy paths (`Policy`, `Architecture`, `Drift`, `CI`) remain as the
deeper-learning track for users who want the full taxonomy walk
(`tutorial/paths.rs:138-587`). Each is a sequence of `TutorialStep`s with
optional commands and verifiers.

The tutorial surface is the only non-watch surface that consumes a live event
channel — file-change events from the kernel watcher trigger automatic
re-verification on steps that declare a `watch_path`
(`crates/anvil-cli/src/tui.rs:265-299`). Static-mode kicks in when the kernel
watcher is unavailable; the surface flips `static_mode = true`, attaches the
`STATIC_MODE_WATCHER_UNAVAILABLE` notice, and disables command execution so all
steps become press-enter-to-continue (`tutorial/mod.rs:24-25, 199-202`).

The `watch_demo` sub-surface (LAUNCH-014 / WELCOME-014) is launched when a
tutorial step's `watch_demo: true` flag fires Enter — the tutorial loop exits,
the CLI command transitions to `run_watch_demo`
(`anvil-cli/src/tui.rs:304-404`), and the `WatchDemoState` consumes engine
events with progressive overlay hints.

## Welcome / wizard / onboarding

These compose to form the first-run + steady-state launcher. `welcome` is the
post-onboarding menu surface (after LAUNCH-006 promoted `start` away from being
a clap alias of `welcome`); `wizard` runs the `anvil new` template flow;
`onboarding` handles first-run.

### Onboarding

Triggered when `config_exists_in(cwd)` returns false
(`onboarding/mod.rs:31-35`). Four sub-surfaces:

- `OnboardingWelcomeState` — "Set up this project" / "Choose a learning path" /
  "Go to command menu" three-choice selector (`onboarding/welcome.rs:5-40`).
- `HooksState` — hook installation; detects `Husky`, `Lefthook`, `pre-commit`,
  Git 2.54 native config-mode hooks (`onboarding/hooks.rs:30-42`), and
  `is_anvil_managed_command` to avoid duplicate installs.
- `InitCompleteState` — post-init summary with the `widgets/results_dashboard`
  widget showing framework, project root, file count, monorepo flag, TS
  strictness, analysis summary, and `QuickWinsAnalysis`
  (`widgets/results_dashboard.rs:11-66`, `widgets/quick_wins_panel.rs:9-69`).
- `CompletionState` — end-of-onboarding summary.

The dispatcher is the `welcome` command in
`crates/anvil-cli/src/commands/welcome.rs:200-1030`, which composes the four
sub-surfaces with `run_surface_in` against a single shared terminal so the
alt-screen never tears between transitions.

### Wizard (`anvil new`)

Four-step template scaffolder (`wizard/mod.rs:14-43`): TemplateSelect →
ProjectName (text input) → Configure (`enable_watch`, `enable_hooks` toggles) →
Summary. Exits with `WizardState::confirmed = true` and the assembled
`WizardConfig`. The browser surface feeds template ids into this wizard.

### Welcome (post-onboarding menu)

Seven options (`welcome/mod.rs:7-50`). The launcher logic in
`crates/anvil-cli/src/commands/welcome.rs:958-1030` pattern-matches on
`QuickStartOption` and launches the corresponding sub-surface with
`run_surface_in` (gate / audit / doctor / tutorial / watch / docs / restart
onboarding).

## Doctor surface

Nine diagnostic checks, four auto-fixable. The check list is built in
`crates/anvil-cli/src/commands/doctor.rs::collect_checks`; the surface only
renders. List view + expand-on-select detail with the `Remediation` block
rendered structurally (summary line, optional command line, optional doc URL
line) instead of as free-form prose.

`f` to fix the selected check fires when `auto_fixable && status != Pass`
(`doctor/mod.rs:160-168`). Pressing `f` sets
`pending_fix = Some(FixRequest::DoctorCheck { index })`; `should_quit()` then
returns true, the runner returns the state, the CLI command applies the fix via
`apply_fix_request(&request, Some(&mut state.checks))`, builds a
`FixOutcomeBanner::{Applied|Refused|Failed}`, sets it on the state, and
re-enters the TUI (`crates/anvil-cli/src/commands/doctor.rs:45`). The banner
clears on the next user action so it stays transient.

The expanded-with-remediation snapshot
(`doctor/snapshots/anvil_tui__surfaces__doctor__render__tests__snapshot_expanded_with_remediation.snap`)
pins the structural rendering.

## Status surface

Three panels (Hooks / Profile / Results), tab-style navigation, j/k inside the
focused panel, `z` to zoom (`status/mod.rs:42-79`). The activation diagnostic
renderer for `anvil status --verify` lives in `crates/anvil-cli/src/activation/`
not this surface; cross-link
[`activation-as-built.md`](./activation-as-built.md). The TUI status surface is
the human-rendered config / hooks / recent-runs view, not the activation
verifier.

## Audit surface

Four panels (Project / Issues / Historical / Next Steps),
`AuditPanel::next/prev` cycles between them, `j/k` scrolls inside Issues,
`enter` expands an issue to detail, `f` fires
`FixRequest::AuditConsoleStatement` for `console.*` removal
(`audit/mod.rs:70-120`). The five-level severity vocabulary
(`CRIT / HIGH / MED / LOW / INFO`) is consistent with the audit JSON contract.
Zoom support landed in `v0.5.1-beta`.

The `.env` / env-template skip filter that keeps the audit TUI in line with the
audit CLI behaviour is owned upstream — `is_env_template_filename` lives in
`crates/anvil-cli/src/commands/audit.rs:186-212`. The TUI just renders whatever
issues the CLI passes in.

## Gate surface

The workflow judgement layer for `anvil gate` interactive mode. Filter ribbon
(`a/p/f/s/w`), `/` search (with `n/N` to jump between matches), `enter` to
expand a check (`gate/mod.rs:60-115`). Cross-link
[`checks-as-built.md`](./checks-as-built.md) §"CLI surfaces" for the upstream
check dispatch; the gate TUI is purely the human renderer for a `GateResult`.

## Browser surface

Template browser for `anvil new`. Three sequential views — Categories →
Templates → Detail — with forward/backward navigation and a `/` search mode
(`browser/mod.rs:6-31, 61-74`). Exit semantics: the surface sets
`chosen: Option<String>` to the selected template id; the calling command hands
that id to the wizard.

## Init surface

The post-init analysis renderer (IFR-003). The init wizard itself is the
five-step `init/mod.rs` surface; the post-init analysis dashboard is the
`widgets/results_dashboard.rs` widget mounted by the welcome / init_complete
surfaces. The Rust port covers both the wizard and the post-init render — there
is no separate init TUI surface beyond what's already in `init/` and
`widgets/results_dashboard.rs`.

## Shared widget vocabulary

Two widgets ship inside `anvil-tui`; the bulk of the widget vocabulary lives
upstream in `eddacraft-tui` (`crates/eddacraft-tui/src/widgets/` — `data_table`,
`parallel_progress`, `image_pane`, `text_input`, etc.) and is re-used through
the `Surface` trait.

| Widget             | File                           | Used by                       | Notes                                                                                                                                                      |
| ------------------ | ------------------------------ | ----------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `QuickWinsPanel`   | `widgets/quick_wins_panel.rs`  | init/onboarding init_complete | Batched warning suppressions panel. Shows progress bar, top 5 batch groups by count, type icons (T/D/C/G/M/P/L). Renders empty state when no batch groups. |
| `ResultsDashboard` | `widgets/results_dashboard.rs` | onboarding init_complete      | Post-init analysis dashboard. Shows framework, project root, file count, monorepo flag, TS strictness, analysis summary, embeds `QuickWinsPanel`.          |

Shared affordances across surfaces:

- **Zoom controls** (`v0.5.1-beta`) — `z` to toggle; available on watch, status,
  audit. The convention is: `z` toggles, `esc` exits zoom on first press and
  navigates back on second.
- **Search** — `/` enters search mode; type to filter; `enter` confirms; `esc`
  cancels. Available on gate, browser. `n` / `N` jumps between matches on gate.
- **Fix** — `f` on a fixable item emits a `FixRequest` and exits the TUI.
  Available on doctor, audit, tutorial (per-step fix), welcome (sub-surface fix
  dispatch).
- **Help text** — every surface implements `Surface::help_text()` returning a
  one-line key map; the shell footer renders this with the
  `eddacraft v<VERSION>` watermark on the right (`shell.rs:79-105`).

## Snapshot infrastructure

The crate uses insta-style snapshot tests pinned per surface. The pattern is:

1. Build a `Buffer` of fixed size via `TestBackend::new(width, height)`
   (`shell.rs:121-130` is the canonical example).
2. Render the surface (or just the shell) into the backend.
3. Serialise the resulting buffer with
   `eddacraft_tui::test_utils::snapshot::buffer_to_string`
   (`crates/eddacraft-tui/src/test_utils.rs:54-66`). Each cell is rendered as
   `<symbol>[<style>]` where the style annotation includes foreground colour,
   background colour, and modifier flags (`bold`, `dim`, `italic`, `underlined`,
   `reversed`, `crossed_out`). Cells with no styling emit only the symbol —
   keeps snapshots noise-free.
4. `insta::assert_snapshot!(...)` against a pinned `.snap` file.

The crate ships **41 snapshot files** (40 surface + 1 shell chrome) across the
surfaces:

```
audit/snapshots/        5 .snap files (default, expanded scroll, issues expanded, issues focused, last item expanded)
browser/snapshots/      3 .snap files (categories, detail, templates view)
doctor/snapshots/       3 .snap files (default, expanded, expanded with remediation)
gate/snapshots/         2 .snap files (default, with filter)
init/snapshots/         5 .snap files (mode, format, directory, checks, summary steps)
onboarding/snapshots/   1 .snap file (init_complete default)
status/snapshots/       2 .snap files (default, results focused)
tutorial/snapshots/    10 .snap files (path select + 3 sizes, running phase + 3 sizes, complete phase + variants, running static)
watch/snapshots/        3 .snap files (default, idle empty, queue focused)
welcome/snapshots/      2 .snap files (default, second item selected)
wizard/snapshots/       4 .snap files (template, name, configure, summary steps)
src/snapshots/          1 .snap file (shell chrome)
```

The `dashboard` and `plan_dashboard` surfaces currently ship no snapshot
coverage — see G-01.

The tutorial surface specifically pins narrow (40x10) and tiny (20x10) terminal
sizes alongside the default 80x24 to guard against narrow-terminal regressions
in the path selector and running phase — this is the surface most exposed to
first-run terminals where defaults are unpredictable.

The shell chrome itself has a snapshot
(`src/snapshots/anvil_tui__shell__tests__snapshot_shell_chrome.snap`) pinned at
60x10 to lock the `Anvil > <surface>` header layout and the
`eddacraft v<VERSION>` watermark padding.

`buffer_to_string` is style-aware on purpose — many regressions in TUI rendering
are colour / modifier swaps without symbol changes (focused vs unfocused panel
borders, status-line glyph colour swap on `Passing → Failing`). Symbol-only
snapshots would silently miss these.

## Cross-cutting concerns

### Determinism in rendering

Same data + same terminal size → same output. This is the load-bearing invariant
for snapshot stability. Notable practical consequences:

- The shell chrome footer derives the version from `env!("CARGO_PKG_VERSION")`
  at compile time, but the snapshot test asserts shape (`v<digit>.…`) rather
  than a specific minor (`shell.rs:185-202`) so the snapshot does not need
  touching on every release.
- Animated fields (watch `pass_rate`, `avg_duration_ms`) use snapshot fixtures
  that observe the surface at a specific animation frame — the `take_dirty()`
  paint gate keeps animation idempotent (`watch/mod.rs:200-261`).
- ASCII-only glyphs in load-bearing places (the watch action footer) rule out
  terminal-charset-dependent rendering drift.

### Compatibility shims (`compat.rs`)

`crates/anvil-tui/src/compat.rs` is a 3-line re-export:

```rust
pub use eddacraft_tui::compat::{TerminalInfo, detect_terminal, validate_minimum_size};
```

The actual terminal probing (kitty / iTerm2 / Windows Terminal / xterm-256color
detection, truecolor support, minimum size enforcement) lives in the upstream
eddacraft-tui crate (extracted via TUIEXTRACT). `TuiApp::new` calls
`validate_minimum_size` unless `skip_terminal_check` is set — the default
minimum is enforced for live runs, opted out only by tests (`app.rs:45-49`).

### Migration (`migration.rs`)

`migration.rs` is the Ink-to-Ratatui migration shim. `TuiBackend` enum defaults
to `Ink`; `--tui=ratatui` opts in. `TuiApp::new` rejects `TuiBackend::Ink` with
`BackendNotSupported` because Ink is handled by the Node.js process, not the
Rust crate (`app.rs:51-55`). The PORT module is complete (`plans/index.aps.md`:
15/15) — the Ratatui surfaces have full parity with the legacy Ink surfaces; Ink
remains the default TUI backend during the migration window so existing scripts
don't break.

### Zoom controls

`v0.5.1-beta` shipped zoom on three surfaces — watch, status, audit. The
convention: `z` toggles; `esc` exits zoom on first press and navigates back on
second. Affordance lives behind a `zoomed: bool` flag on the surface state; the
renderer collapses the multi-panel layout to the focused panel filling the area
(`watch/render.rs:25-31`, `status/mod.rs:73-77`).

## Known gaps

### G-01: dashboard surfaces ship without snapshot coverage

The json-render interpreter this gap originally tracked has shipped:
TUIDASH-003..-013 merged 2026-06-02 via PRs #2229 / #2246, landing the engine
(eddacraft-tui, feature-gated per ADR-054), the Anvil-domain catalogue, and the
`anvil dashboard` surface family — see the dashboard subsection above. Custom
`.anvil/dashboards/*.json` specs now render in the TUI. The remaining gap is
snapshot coverage: the `dashboard` and `plan_dashboard` surfaces ship no `.snap`
files, leaving the two newest surface families outside the style-aware snapshot
net every other surface sits inside.

**Risk:** Low–Medium — rendering regressions in the dashboard family would not
be caught by the snapshot suite. **Fix:** add insta snapshots for the list /
spec / native dashboards and the plan dashboard, matching the per-surface
`snapshots/` convention.

### G-02: Ink remains the default TUI backend

`TuiBackend::default() == Ink` (`migration.rs:7-9`). Ratatui only renders when
the user passes `--tui=ratatui`. The PORT module is complete and the Ratatui
surfaces have parity, but the default has not been flipped because flipping it
is itself a release-gate decision (existing scripts pinning to Ink output would
need to opt in).

**Risk:** Low — both backends ship and work. **Fix:** flip the default in a
future release once the deprecation window closes; no tracked work item yet.

### G-03: Watch surface drops the cycle's history entry on kernel crash

If a `Progress` sequence is not followed by a `Snapshot` (kernel crash, channel
disconnect), the cycle never records to history
(`watch/event_adapter.rs:36-41`). This is documented as acceptable because the
watch session is already terminated in that scenario, but it does mean the last
cycle before a crash silently goes missing from `WatchData.history`.

**Risk:** Low — the kernel crash itself is the primary signal; missing history
is secondary. **Fix:** record a partial entry on channel close, or have the
kernel emit a "cycle aborted" event.

### G-04: Action footer is single-entry, not historical

`WatchData.last_action` is `Option<ActionResultLine>` — only the most recent
action is retained. Richer history (`Vec<ActionRun>`) is documented as deferred
to LAUNCH-002b against the TUIDASH-009 inheritance seam (`watch/mod.rs:80-84`);
the TUIDASH-009 spec surface has since shipped, so the seam is live rather than
pending.

**Risk:** Low. **Fix:** tracked as LAUNCH-002b; the TUIDASH dashboard surface it
was deferred against has shipped.

### G-05: Watch fallback liveness not yet wired in tutorial step 5

The ProtectionLoop tutorial's final step honestly says "Watch-fallback liveness
probing is not yet wired; the verifier reports `watch: not requested` until a
future PR introspects a running watcher" (`tutorial/paths.rs:131`). Tracked
against the activation orchestrator, not this crate, but visible to the tutorial
reader.

**Risk:** Low — the verifier reports the limitation honestly. **Fix:**
activation orchestrator follow-up; cross-link
[`activation-as-built.md`](./activation-as-built.md) gaps.

### G-06: No unicode / truecolor fallback policy documented

`anvil-tui` uses unicode-width and the eddacraft-tui themes assume terminal
truecolor. There's no documented graceful-degradation matrix for terminals that
don't support truecolor, mouse, or wider unicode. The watch action footer is the
only place that explicitly enforces ASCII-only (`watch/render.rs:53-68`); the
rest of the surfaces lean on the terminal / crossterm fallbacks.

**Risk:** Medium for users on legacy Windows terminals or limited CI
environments. **Fix:** document a tested-terminals matrix and downgrade
strategy; no tracked work item yet.

### G-07: Browser surface lacks variable-step interaction

`BrowserState` carries a `var_selected: usize` field (`browser/mod.rs:67`) but
the rendered Detail view does not yet allow the user to interact with template
variables — the wizard surface owns variable input. The browser is purely
informational.

**Risk:** Low — wizard handles the actual variable input. **Fix:** non-issue
unless we want to merge browser and wizard surfaces, which is a larger UX
decision.

## Source references

| File                                                            | Role                                                                                                                                                                  |
| --------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `crates/anvil-tui/Cargo.toml`                                   | Dependency surface (eddacraft-tui, ratatui, crossterm, animate, anvil-kernel-types, unicode-width)                                                                    |
| `crates/anvil-tui/src/lib.rs`                                   | Module root; pub mod app/compat/dashboard_catalog/dashboard_context/fileio/migration/shell/surface/surfaces/widgets; re-exports `json_render::sanitize` (`lib.rs:15`) |
| `crates/anvil-tui/src/app.rs`                                   | `TuiApp` (watch lifecycle wrapper); `TuiAppConfig`; `TuiError`                                                                                                        |
| `crates/anvil-tui/src/shell.rs`                                 | `render_shell` (header + footer); `inset_content`; `OUTER_H_MARGIN`/`OUTER_TOP_MARGIN`                                                                                |
| `crates/anvil-tui/src/surface.rs`                               | Re-exports `eddacraft_tui::surface::Surface`                                                                                                                          |
| `crates/anvil-tui/src/compat.rs`                                | Re-exports `TerminalInfo`, `detect_terminal`, `validate_minimum_size`                                                                                                 |
| `crates/anvil-tui/src/dashboard_catalog/mod.rs`                 | `anvil_registry` / `anvil_catalog` (base + domain components); `GATE_SUMMARY_SPEC` crate asset                                                                        |
| `crates/anvil-tui/src/dashboard_context.rs`                     | `load_context` — json-render `DataContext` from `.anvil/` state (TUIDASH-008)                                                                                         |
| `crates/anvil-tui/src/fileio.rs`                                | `read_capped` — bounded reads of untrusted `.anvil/` content                                                                                                          |
| `crates/anvil-tui/src/migration.rs`                             | `TuiBackend` enum (Ink default); `select_backend`                                                                                                                     |
| `crates/anvil-tui/src/test_utils.rs`                            | Re-exports `snapshot::buffer_to_string`                                                                                                                               |
| `crates/anvil-tui/src/widgets/quick_wins_panel.rs`              | `QuickWinsPanel` widget — batched suppressions                                                                                                                        |
| `crates/anvil-tui/src/widgets/results_dashboard.rs`             | `ResultsDashboard` widget — post-init analysis                                                                                                                        |
| `crates/anvil-tui/src/surfaces/mod.rs`                          | Surface module declarations                                                                                                                                           |
| `crates/anvil-tui/src/surfaces/fix_request.rs`                  | `FixRequest` enum (DoctorCheck / AntiPatternWarning / AuditConsoleStatement)                                                                                          |
| `crates/anvil-tui/src/surfaces/notifications.rs`                | `NotificationSource` trait + `surface_notification` builder                                                                                                           |
| `crates/anvil-tui/src/surfaces/audit/mod.rs`                    | `AuditState`, `AuditPanel`, `IssueSeverity`, `AuditFixKind`                                                                                                           |
| `crates/anvil-tui/src/surfaces/audit/render.rs`                 | Audit render                                                                                                                                                          |
| `crates/anvil-tui/src/surfaces/browser/mod.rs`                  | `BrowserState`, `BrowserView`, `TemplateCategory`, `TemplateEntry`                                                                                                    |
| `crates/anvil-tui/src/surfaces/browser/render.rs`               | Browser render                                                                                                                                                        |
| `crates/anvil-tui/src/surfaces/dashboard/mod.rs`                | Dashboard surface module declarations (list / spec / architecture / drift / suppressions)                                                                             |
| `crates/anvil-tui/src/surfaces/dashboard/list.rs`               | `DashboardListState`, `ListEntry` — two-pane picker with live previews (TUIDASH-012)                                                                                  |
| `crates/anvil-tui/src/surfaces/dashboard/spec.rs`               | `SpecDashboardState` — json-render spec surface, `$data` binding + refresh (TUIDASH-009)                                                                              |
| `crates/anvil-tui/src/surfaces/dashboard/architecture.rs`       | Native architecture-health dashboard (TDASH-002)                                                                                                                      |
| `crates/anvil-tui/src/surfaces/dashboard/drift.rs`              | Native drift-snapshots dashboard (TDASH-003)                                                                                                                          |
| `crates/anvil-tui/src/surfaces/dashboard/suppressions.rs`       | Native suppressions-overview dashboard (TDASH-004)                                                                                                                    |
| `crates/anvil-tui/src/surfaces/doctor/mod.rs`                   | `DoctorState`, `DiagnosticCheck`, `Remediation`, `FixOutcomeBanner`                                                                                                   |
| `crates/anvil-tui/src/surfaces/doctor/render.rs`                | Doctor render                                                                                                                                                         |
| `crates/anvil-tui/src/surfaces/gate/mod.rs`                     | `GateState`, `GateCheck`, `GateResult`, `FilterStatus`                                                                                                                |
| `crates/anvil-tui/src/surfaces/gate/event_adapter.rs`           | Gate event adapter                                                                                                                                                    |
| `crates/anvil-tui/src/surfaces/gate/render.rs`                  | Gate render                                                                                                                                                           |
| `crates/anvil-tui/src/surfaces/init/mod.rs`                     | `InitState`, `InitStep`, `InitMode`, `ConfigFormat`, `AvailableCheck`                                                                                                 |
| `crates/anvil-tui/src/surfaces/init/render.rs`                  | Init wizard render                                                                                                                                                    |
| `crates/anvil-tui/src/surfaces/onboarding/mod.rs`               | `config_exists_in`; module declarations                                                                                                                               |
| `crates/anvil-tui/src/surfaces/onboarding/welcome.rs`           | First-run welcome (3 choices)                                                                                                                                         |
| `crates/anvil-tui/src/surfaces/onboarding/welcome_render.rs`    | First-run welcome render                                                                                                                                              |
| `crates/anvil-tui/src/surfaces/onboarding/hooks.rs`             | Hook installation surface (Husky / Lefthook / pre-commit / config)                                                                                                    |
| `crates/anvil-tui/src/surfaces/onboarding/hooks_render.rs`      | Hook installation render                                                                                                                                              |
| `crates/anvil-tui/src/surfaces/onboarding/init_complete.rs`     | Post-init summary state                                                                                                                                               |
| `crates/anvil-tui/src/surfaces/onboarding/complete.rs`          | End-of-onboarding summary state                                                                                                                                       |
| `crates/anvil-tui/src/surfaces/plan_dashboard/mod.rs`           | `PlanDashboardState`, `PlanDashboardSnapshot`, module / work-item / warning rows (APSCAN)                                                                             |
| `crates/anvil-tui/src/surfaces/plan_dashboard/event_adapter.rs` | Plan dashboard key/action handling (filter mode, toggles, rescan)                                                                                                     |
| `crates/anvil-tui/src/surfaces/plan_dashboard/render.rs`        | Plan dashboard render                                                                                                                                                 |
| `crates/anvil-tui/src/surfaces/status/mod.rs`                   | `StatusState`, `StatusPanel`, `HookStatus`, `ProfileInfo`, `GateRunResult`                                                                                            |
| `crates/anvil-tui/src/surfaces/status/render.rs`                | Status render                                                                                                                                                         |
| `crates/anvil-tui/src/surfaces/tutorial/mod.rs`                 | `TutorialState`, `TutorialPath`, `TutorialPhase`, `TutorialStep`, `STATIC_MODE_WATCHER_UNAVAILABLE`                                                                   |
| `crates/anvil-tui/src/surfaces/tutorial/paths.rs`               | `protection_loop_steps`, `policy_steps`, `architecture_steps`, `drift_steps`, `ci_steps`                                                                              |
| `crates/anvil-tui/src/surfaces/tutorial/discovery.rs`           | Tutorial discovery (scan results, finding severity)                                                                                                                   |
| `crates/anvil-tui/src/surfaces/tutorial/discovery_render.rs`    | Discovery render                                                                                                                                                      |
| `crates/anvil-tui/src/surfaces/tutorial/executor.rs`            | Tutorial command executor                                                                                                                                             |
| `crates/anvil-tui/src/surfaces/tutorial/fix.rs`                 | Tutorial in-step fix surface                                                                                                                                          |
| `crates/anvil-tui/src/surfaces/tutorial/fix_render.rs`          | Fix render                                                                                                                                                            |
| `crates/anvil-tui/src/surfaces/tutorial/render.rs`              | Tutorial main render (path select / running / complete phases)                                                                                                        |
| `crates/anvil-tui/src/surfaces/tutorial/showcase.rs`            | Tutorial showcase content                                                                                                                                             |
| `crates/anvil-tui/src/surfaces/tutorial/verify.rs`              | `Verify::FileExists`, `Verify::ExitCode`, `VerifyResult`                                                                                                              |
| `crates/anvil-tui/src/surfaces/tutorial/watch_demo.rs`          | Watch demo surface (LAUNCH-014 / WELCOME-014)                                                                                                                         |
| `crates/anvil-tui/src/surfaces/tutorial/watch_demo_render.rs`   | Watch demo render                                                                                                                                                     |
| `crates/anvil-tui/src/surfaces/update_hint.rs`                  | `UpdateHint` shared DTO (DISTRIB-002); re-exported at `surfaces/mod.rs:18`                                                                                            |
| `crates/anvil-tui/src/surfaces/watch/mod.rs`                    | `WatchState`, `WatchData`, `WatchStatus`, `WatchPanel`, `ActionResultLine`, `RunHistory`, `WatchStats`                                                                |
| `crates/anvil-tui/src/surfaces/watch/event_adapter.rs`          | `WatchEventAdapter` (kernel events → WatchData)                                                                                                                       |
| `crates/anvil-tui/src/surfaces/watch/render.rs`                 | Watch 2x2 grid render + action footer                                                                                                                                 |
| `crates/anvil-tui/src/surfaces/welcome/mod.rs`                  | `WelcomeState`, `QuickStartOption`                                                                                                                                    |
| `crates/anvil-tui/src/surfaces/welcome/render.rs`               | Welcome render                                                                                                                                                        |
| `crates/anvil-tui/src/surfaces/wizard/mod.rs`                   | `WizardState`, `WizardStep`, `WizardConfig`, `Template`                                                                                                               |
| `crates/anvil-tui/src/surfaces/wizard/render.rs`                | Wizard render                                                                                                                                                         |

External references (consumers in `anvil-cli`):

- `crates/anvil-cli/src/tui.rs` — terminal session lifecycle, `run_surface`,
  `run_watch`, `run_tutorial`, `run_watch_demo`, `run_*_in` variants.
- `crates/anvil-cli/src/commands/welcome.rs` — composes onboarding → welcome →
  sub-surfaces with shared terminal session.
- `crates/anvil-cli/src/commands/doctor.rs` — `collect_checks` (the nine
  checks); `apply_fix_request` integration.
- `crates/anvil-cli/src/commands/audit.rs` — `is_env_template_filename` (the
  env-template skip filter).
- `crates/anvil-cli/src/services/interactive_fix.rs` — `apply_fix_request`,
  `FixOutcome`.

## Related docs

- [`kernel-as-built.md`](./kernel-as-built.md) — upstream `EngineEvent` protocol
  consumed by the watch surface (KERN-033).
- [`activation-as-built.md`](./activation-as-built.md) — `ProtectionState`
  vocabulary referenced by the tutorial ProtectionLoop path; renderer for
  `anvil status --verify` (lives outside `anvil-tui`).
- [`checks-as-built.md`](./checks-as-built.md) — gate / audit upstream dispatch;
  the gate and audit TUI surfaces are pure renderers over check results.
- [`intercept-as-built.md`](./intercept-as-built.md) — daemon protocol;
  `anvil-tui` does not consume the daemon directly, but the watch surface's
  `--action` footer reflects daemon-backed validation outcomes.
- `RELEASE-PLAN.md` — `v0.5.1-beta` zoom controls, `v0.6.0-beta` slate.
- `CHANGELOG.md` ll. 134-168 — TUI hotfix history (zoom controls, doctor /
  tutorial papercuts, audit env-template filtering).
- `plans/archive/modules/tui-dashboard-render.aps.md` — TUIDASH (Complete 13/13;
  Released/Shipped in `v0.8.0-beta`).
- `plans/archive/modules/native-tui-dashboards.aps.md` — TDASH (Complete,
  archived).
- `plans/archive/modules/ratatui-tui.aps.md` — RATS (Complete, archived).
- `plans/archive/modules/ink-to-ratatui-port.aps.md` — PORT (Complete,
  archived).
