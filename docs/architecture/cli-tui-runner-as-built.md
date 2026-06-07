# CLI TUI Runner — As-Built

| Type     | Authority | Owner | Status | Freshness                                                                                 |
| -------- | --------- | ----- | ------ | ----------------------------------------------------------------------------------------- |
| As-built | Derived   | RATS  | Live   | Last reviewed 2026-05-07 against `v0.6.0-beta` and `crates/anvil-cli`, `crates/anvil-tui` |

| Upstream                                                      | Downstream                                                                                          |
| ------------------------------------------------------------- | --------------------------------------------------------------------------------------------------- |
| `crates/anvil-cli`, `crates/anvil-tui`, `crates/anvil-kernel` | all interactive anvil commands (watch, tutorial, welcome, status, doctor, audit, init, new, wizard) |

> **Status:** Live (beta) **Last reviewed:** 2026-05-07 against `v0.6.0-beta`
> slate (HEAD `cf7ca040`) **File / location:** `crates/anvil-cli/src/tui.rs`
> (495 lines) **Module owner (APS):** RATS (Ratatui surfaces — Complete 7/7,
> archived to `plans/archive/modules/ratatui-tui.aps.md`); the runner is the
> CLI-side counterpart that those surfaces are mounted through. **Used by:**
> every interactive `anvil` command that mounts a TUI surface — `anvil watch`,
> `anvil tutorial`, `anvil welcome`, `anvil status`, `anvil doctor`,
> `anvil audit`, `anvil init`, `anvil new` (template browser hand-off),
> `anvil wizard`. Twelve call sites across nine command modules.

## Overview

`crates/anvil-cli/src/tui.rs` is the CLI-side bridge between an `anvil`
interactive command and an `anvil-tui` `Surface` instance. It owns the terminal
session lifecycle (raw mode, alternate screen), the per-surface event-poll /
draw loop, the animation tick, and the watch-loop dirty-paint gate. The file is
495 lines and exports nine public entry points plus one enum (`SurfaceExit`).

This is the dispatcher pair to the in-crate `Surface` trait. `anvil-tui` defines
what a surface is — render, handle_key, should_quit, should_back. The runner
here defines how a surface gets driven: where the `Terminal` comes from, when
the alt-screen tears down, when the next frame paints, and how kernel events
feed surfaces that have a live event source.

For the in-crate side of the dispatcher (`Surface` trait, the per-surface state
machines, snapshot infrastructure), see [`tui-as-built.md`](./tui-as-built.md).
The two as-builts are intended to be read together: `tui-as-built.md` covers the
surfaces; this one covers the shell that mounts them.

## Architecture diagram

```text
   anvil <cmd>                                        kernel watcher
       │                                                    │
       ▼                                                    │ EngineEvent
 ┌────────────────────────────┐                            │
 │ commands/<cmd>.rs          │                            │
 │   build Surface state      │                     ┌──────┴──────┐
 │   choose runner            │                     │ mpsc channel│
 └──────────────┬─────────────┘                     └──────┬──────┘
                │                                          │
                ▼                                          │
 ┌─────────────────────────────────────────────────────────┴────┐
 │ crates/anvil-cli/src/tui.rs                                  │
 │                                                              │
 │  setup_terminal ─── enable_raw_mode + EnterAlternateScreen   │
 │                                                              │
 │  run_surface  ────┐                                          │
 │  run_surface_in ──┼─▶ surface_loop  (key-driven)             │
 │                   │     • render only when dirty             │
 │                   │     • poll 100 ms                        │
 │                   │     • re-render after key for ack frame  │
 │                                                              │
 │  run_tutorial ────┐                                          │
 │  run_tutorial_in ─┼─▶ tutorial_loop (key + file events)      │
 │                                                              │
 │  run_watch_demo ──┐                                          │
 │  run_watch_demo_in┼─▶ watch_demo_loop (key + engine events)  │
 │                   │     + animation tick + dirty gate        │
 │                                                              │
 │  run_watch ───────┐                                          │
 │  run_watch_in  ───┼─▶ watch_loop    (key + engine events)    │
 │                   │     + animation tick + dirty gate        │
 │                   │     + LAUNCH-002 action-result drain     │
 │                   │     + optional SIGINT shutdown flag      │
 │                                                              │
 │  draw_loading       (transient frame between sub-surfaces)   │
 │                                                              │
 │  teardown_terminal  disable_raw_mode + LeaveAlternateScreen  │
 └──────────────────────────────────────────────────────────────┘
                │
                ▼ Surface trait (render / handle_key / should_*)
 ┌──────────────────────────────────────────────────────────────┐
 │ anvil-tui surfaces — see tui-as-built.md                     │
 └──────────────────────────────────────────────────────────────┘
```

The standard path (welcome, status, doctor, audit, init, wizard, browser) flows
through `surface_loop`: pure key-driven, render only on dirty, no animation
tick. The tutorial path adds an optional file-change channel. The watch and
watch-demo paths add the kernel `EngineEvent` channel, the `animate::tick`
driver, and the explicit `take_dirty()` paint gate. All four paths share
`render_shell` for chrome and `KeyHandler::map` for keyboard mapping.

## `SurfaceExit` contract

`SurfaceExit` is the two-variant enum returned by every loop variant that hosts
a surface in a shared terminal session (`crates/anvil-cli/src/tui.rs:22-28`):

```rust
pub enum SurfaceExit {
    /// User wants to quit the program.
    Quit,
    /// User wants to go back to the previous screen.
    Back,
}
```

`Quit` propagates up through the welcome hub and exits the entire interactive
session — the alternate screen tears down and the process returns to the shell.
`Back` returns to the previous surface in the call stack: e.g., the welcome hub
catches `Back` from `Audit` and re-mounts `Welcome`
(`crates/anvil-cli/src/commands/welcome.rs:958-1030`). The tutorial and
watch-demo loops do not return `SurfaceExit`; they return `()` because they are
always entered from a known parent that handles its own re-entry.

The mapping is direct: `Surface::should_quit()` true → `Quit`,
`Surface::should_back()` true → `Back`. `surface_loop` checks both immediately
after dispatching a key so quit/back is responsive without waiting for the next
100 ms poll (`crates/anvil-cli/src/tui.rs:129-147`).

## `surface_loop` (the standard render loop)

`surface_loop` is the workhorse. Two entry points reach it:

- `run_surface<S: Surface>(mut state: S) -> anyhow::Result<S>` — owns the
  terminal session for a single surface (`crates/anvil-cli/src/tui.rs:33-47`).
- `run_surface_in<S: Surface>(...) -> anyhow::Result<SurfaceExit>` — reuses an
  already-initialised terminal so a parent (the welcome hub) can swap
  sub-surfaces without tearing the alt-screen
  (`crates/anvil-cli/src/tui.rs:51-57`).

Both delegate to `fn surface_loop` (`crates/anvil-cli/src/tui.rs:97-156`).
Lifecycle, line-pinned:

1. Enable raw mode, swap to the alternate screen, build a
   `Terminal<CrosstermBackend<Stdout>>` (`tui.rs:34-38` for `run_surface`;
   `setup_terminal` for the shared-terminal path, `tui.rs:61-67`).
2. Construct an `EddaCraftTheme` (zero-arg, `tui.rs:39`).
3. Enter the loop with `dirty = true` so the first frame always renders
   (`tui.rs:103-104`).
4. If dirty, draw: call
   `render_shell(frame, area, surface_name, help_text, theme)` for the chrome,
   then `state.render(frame, content, theme)` for the surface body, then
   `dirty = false` (`tui.rs:107-115`).
5. `event::poll(Duration::from_millis(100))` — block at most 100 ms waiting for
   a terminal event (`tui.rs:117`).
6. On a `Press` key event: map via `KeyHandler::map(key)` and dispatch to
   `state.handle_key(action)`. Set `dirty = true` (`tui.rs:119-122`).
7. Immediately check `should_quit` / `should_back` — if either fires, draw the
   post-key acknowledgement frame first (so any "Applying auto-fix..." text the
   surface added is actually seen) and then return `SurfaceExit::Quit` or
   `SurfaceExit::Back` (`tui.rs:129-147`).
8. On `Resize`, set `dirty = true` so the next iteration redraws
   (`tui.rs:149-151`).
9. On exit, `disable_raw_mode` and `LeaveAlternateScreen` — for `run_surface`,
   this is in-line teardown; for `run_surface_in`, the parent calls
   `teardown_terminal` itself (`tui.rs:43-45`, `tui.rs:89-95`).

Two invariants worth pinning:

- **Dirty-gate paint** — `surface_loop` does not redraw on every loop iteration;
  only when the surface dispatched a key, when the terminal resized, or on the
  very first frame. This is cheap-enough for key-driven surfaces but the watch
  path needs a richer gate (see below).
- **Acknowledgement frame** — pressing `f` on the doctor surface to apply a fix
  sets `should_quit = true` immediately, but the runner draws once more before
  tearing down so the user sees the "Applying..." line. This is documented in
  the source as a deliberate ordering choice (`tui.rs:124-141`).

## Animation tick

Surfaces that animate (watch and watch-demo, currently) drive interpolation
through the upstream `animate` crate. The runner integrates it via
`tick_animations` (`crates/anvil-cli/src/tui.rs:475-495`):

```rust
fn tick_animations<F>(last_tick: &mut Instant, already_dirty: bool, mut mark_dirty: F)
where F: FnMut() {
    let now = Instant::now();
    let elapsed = now.saturating_duration_since(*last_tick);
    let ms = elapsed.as_millis().min(usize::MAX as u128) as usize;
    if ms > 0 {
        *last_tick += Duration::from_millis(ms as u64);
        animate::tick(ms);
    }
    if !already_dirty && is_animating() {
        mark_dirty();
    }
}
```

Three behaviours:

1. **Whole-millisecond accounting** — `last_tick` advances only by the integer
   millisecond portion consumed; the sub-millisecond remainder accumulates into
   the next iteration so animations still progress when loop iterations run
   faster than 1 ms (`tui.rs:485-490`).
2. **Pull-based** — the runner doesn't know what's animating; it asks
   `animate::is_animating()` and only marks the surface dirty if something is in
   flight and the surface isn't already dirty (`tui.rs:492-494`).
3. **Frame-budget cap** — the watch and watch-demo loops cap `event::poll` at 16
   ms (~60 fps) when dirty _and_ animating, so the loop doesn't busy-spin
   between animation frames (`tui.rs:260-268`, `tui.rs:422-430`).

The `surface_loop` standard path does not call `tick_animations` — key-driven
surfaces don't animate.

## Watch loop (kernel-event-driven path)

`watch_loop` is the only loop where the event source is a kernel
`mpsc::Receiver<EngineEvent>` rather than just terminal events
(`crates/anvil-cli/src/tui.rs:376-473`). Two entry points:

- `run_watch(state, event_rx, action_link, shutdown)` — owns the terminal
  session for `anvil watch --tui=ratatui` (`tui.rs:330-356`).
- `run_watch_in(terminal, state, event_rx)` — runs inside an already-initialised
  terminal for the welcome hub's "Watch" option; always passes `None` for
  `action_link` and `shutdown` (`tui.rs:362-374`).

Per loop iteration, in order (`crates/anvil-cli/src/tui.rs:391-470`):

1. **Drain engine events** — `event_rx.try_recv()` in a tight loop until the
   channel returns `Empty`. Every event flows through
   `WatchEventAdapter::handle_event(&engine_event, &mut state.data)`, which is
   the canonical kernel-event-to-surface-state translator (`tui.rs:392-404`).
   Each drained event marks the surface dirty.
2. **LAUNCH-002 snapshot dispatcher** — if there is an `action_link`, every
   event whose type is `Snapshot` increments `snapshot_count`. The first
   snapshot (the initial scan) is suppressed; from the second onwards,
   `link.dispatcher.on_snapshot()` fires the configured `--action` command. This
   mirrors the gating in the non-TUI watch branch (`tui.rs:386-389`,
   `tui.rs:395-403`).
3. **Drain pending action results** — if there is an `action_link`,
   `action_rx.try_recv()` is drained. Each result is folded into
   `state.data.last_action` via `WatchEventAdapter::handle_action_result`. The
   single-writer invariant (only the adapter writes `last_action`) is documented
   in `tui-as-built.md` §"Action footer (LAUNCH-002)" (`tui.rs:407-413`).
4. **Animation tick** — `tick_animations` advances `animate::tick` and marks the
   surface dirty if any animation is active. After that,
   `state.sync_animations()` reads the animated values back into the render-time
   fields on `WatchData` (`tui.rs:415-418`).
5. **Compute poll timeout**:
   - Dirty + animating → 16 ms (≈60 fps frame budget)
   - Dirty + not animating → 0 ms (drain immediately, paint, loop)
   - Not dirty → 50 ms (idle wait) (`tui.rs:421-430`).
6. **Drain terminal events** — `event::poll` then a tight loop on `event::read`
   while `event::poll(ZERO)` keeps returning true. This is deliberate: a burst
   of key events should not push a `Resize` behind the next paint, so the loop
   drains _all_ pending terminal events before redrawing (`tui.rs:434-450`).
   This drain pattern is unique to `watch_loop` — `surface_loop` reads at most
   one event per iteration.
7. **Dirty-gate paint** — `state.take_dirty()` (consume + reset). Only if it was
   true does the loop call `terminal.draw` (`tui.rs:453-460`).
8. **Exit checks** — `should_quit`, `should_back`, or the optional
   `shutdown: &Arc<AtomicBool>` flag (`tui.rs:462-469`).

The `shutdown` parameter is a SIGINT bridge from the CLI command. Raw mode
normally swallows Ctrl-C as a key event, but some terminal multiplexers forward
the SIGINT signal anyway, so the watch command installs a handler that sets the
flag, and the loop checks it once per iteration as belt-and-braces
(`tui.rs:328-330`).

`watch_demo_loop` (`tui.rs:238-299`) follows the same shape minus the
action-link arms and the resize-drain inner loop.

## `tutorial_loop` (file-change-driven path)

The tutorial loop is a hybrid: key-driven like `surface_loop`, but with an
optional file-change channel (`crates/anvil-cli/src/tui.rs:180-214`). Each
iteration drains `Receiver<ChangeBatch>` (when present), passes the changed
paths to `state.handle_file_change(&paths)`, then renders unconditionally and
polls for a key event. There is no dirty gate — the tutorial paints every
iteration. The loop exits on `should_quit`, `should_back`, or `wants_watch_demo`
(the ProtectionLoop transition to the watch-demo sub-surface).

This is the contract that LAUNCH-014 / WELCOME-013 rely on: when a tutorial step
has a `watch_path`, file changes trigger automatic re-verification without the
user pressing a key.

## Shared-terminal pattern

The welcome hub composes multiple sub-surfaces inside one terminal session
(`crates/anvil-cli/src/commands/welcome.rs:118-1030`). The pattern is:

```text
setup_terminal ─────▶ run_surface_in (onboarding/welcome)
                ─────▶ run_surface_in (init)
                ─────▶ run_surface_in (discovery)
                ─────▶ run_tutorial_in   (tutorial path)
                ─────▶ run_watch_demo_in (watch demo)
                ─────▶ run_surface_in (welcome menu) ── loops on Back ──┐
                ─────▶ run_watch_in    (watch dashboard)                │
                ─────▶ run_surface_in (audit / doctor / gate sub-surfaces)
                ─────▶ teardown_terminal
```

Lifecycle invariants for the shared-terminal path:

- The alt-screen never tears between sub-surfaces. `run_*_in` variants take
  `&mut Terminal<...>` and `&EddaCraftTheme` rather than constructing their own.
- `draw_loading(terminal, surface_name, message, theme)` (`tui.rs:70-86`) paints
  a transient frame inside the shell chrome between sub-surfaces — this is what
  produces the "Running quality checks..." flash before gate launches and the
  "Starting file watcher..." flash before watch
  (`crates/anvil-cli/src/commands/welcome.rs:966`, `welcome.rs:977`,
  `welcome.rs:994`, `welcome.rs:1024`).
- The teardown is idempotent and the welcome command captures the teardown
  result separately from any sub-surface error so a sub-surface failure doesn't
  abort the alt-screen restoration (`welcome.rs:153`).

The eight commands that own their own terminal session (`run_surface` /
`run_watch` / `run_tutorial` / `run_watch_demo`) call `enable_raw_mode` /
`EnterAlternateScreen` themselves (`tui.rs:33-47, 161-178, 219-236, 330-356`).

## Error handling

Terminal initialisation errors propagate through `?` in the public entry points
(`tui.rs:34-38`, `tui.rs:165-169`, etc.): if `enable_raw_mode` fails (not a TTY,
no ANSI support), the function returns `Err(io::Error)` before any state is
constructed. The CLI `commands/<cmd>.rs` layer is responsible for reporting that
error in human-readable form and choosing whether to fall back to a non-TUI path
(e.g., `anvil watch` falls back to streaming JSON when stdout is not a TTY — see
[`activation-as-built.md`](./activation-as-built.md) for the watch-fallback
contract).

There is **no panic safety**. Terminal teardown is a plain function call
sequence in each `run_*` wrapper, not a `Drop` guard. If the surface, adapter,
or render code panics inside the loop, `disable_raw_mode` and
`LeaveAlternateScreen` are not run, and the user's terminal is left in raw mode
with the alternate screen active. This is recorded in §"Known gaps" below as
G-01.

The kernel-event channel is consumed via `try_recv` only — a disconnected
channel (kernel watcher has died) returns `Err(Disconnected)` from `try_recv`,
which is silently absorbed by the `while let Ok(...)` pattern (`tui.rs:248-249`,
`tui.rs:392-394`). The watch dashboard then sits idle with no further state
changes; the parent CLI command is responsible for detecting the dead watcher
via its handle (`handle.stop()` returns the error,
`crates/anvil-cli/src/commands/watch.rs:880`).

## Cross-cutting concerns

### Panic safety

There is no `Drop` guard for the terminal in the runner. Every `run_*` wrapper
is a flat sequence:
`enable_raw_mode → EnterAlternateScreen → loop → disable_raw_mode → LeaveAlternateScreen`.
The `disable_raw_mode` / `LeaveAlternateScreen` calls execute only on the happy
path (`tui.rs:43-45, 174-175, 232-233, 352-354`). Tracked as G-01 below.

### Thread model

The runner is single-threaded. Every loop variant runs entirely on the calling
thread. Kernel events arrive via `mpsc::Receiver<EngineEvent>` from the
kernel-watcher thread (which lives in the calling command, e.g.,
`crates/anvil-cli/src/commands/watch.rs::run`), and the runner pulls them out
with `try_recv` — never blocking, never spawning a thread of its own. The render
loop, the event drain, and the action-result drain all run sequentially in the
same iteration.

### Determinism

Same surface state + same input sequence → same render history. The runner
provides three load-bearing pieces of that determinism:

- The render is gated on dirty — a redundant key event that doesn't change state
  does not produce a new frame (the surface controls dirty itself).
- The poll timeout is bounded — 100 ms in the standard path, 16 / 0 / 50 ms in
  the watch path — so the loop's wall-clock cadence is bounded above and below.
- Animation tick is whole-millisecond accounted; the sub-ms residual is
  accumulated, not dropped, so animations advance reproducibly even on fast
  machines (`tui.rs:485-490`).

The render output itself — the buffer comparison — lives in the surface crate's
snapshot tests (see [`tui-as-built.md`](./tui-as-built.md) §"Snapshot
infrastructure"). The runner's contribution is providing a deterministic frame
loop on top of which those snapshots are stable.

### No global state

Every `run_*` invocation is self-contained. The runner constructs its own
`EddaCraftTheme` (zero-arg), takes the surface state by value or `&mut`, takes
the `Terminal` by value (owning entry points) or `&mut` (shared- terminal entry
points), and returns. There is no module-level state, no static, no
lazy-initialised singleton. This matters for the welcome hub's shared-terminal
pattern: a parent can run any number of `run_surface_in` calls in sequence and
each is fully independent except for the explicitly threaded `Terminal`
reference.

## Known gaps

### G-01: No panic-safe terminal restoration

The runner has no `Drop` guard for raw mode or the alternate screen. If a
surface, adapter, or render panics inside any loop variant, the
`disable_raw_mode` / `LeaveAlternateScreen` calls at the bottom of the `run_*`
wrapper are skipped, and the user is left at a shell prompt with raw mode still
enabled and the alt-screen still active.

This is straightforward to fix: introduce a `TerminalGuard` newtype that runs
`disable_raw_mode` and `LeaveAlternateScreen` in `Drop`, and have each `run_*`
wrapper construct one before entering the loop. The Rust ecosystem convention
here is well-established (`ratatui` examples ship a `restore_terminal` helper
plus a panic hook).

**Risk:** Medium. A panic inside the watch loop is improbable today but the cost
when it happens is high (the user's terminal is unusable until they `reset(1)`
or open a new shell), and the kernel-event-driven paths push more code through
the loop than the standard surface path. **Fix:** introduce `TerminalGuard` +
`panic::set_hook`. No tracked APS work item as of `v0.6.0-beta`.

### G-02: No `EnableMouseCapture`

The runner does not enable mouse capture. Surfaces that would benefit from
scroll-wheel scrolling (audit issue list, watch history panel) are
keyboard-only. `crossterm::event::EnableMouseCapture` is not invoked in `tui.rs`
and there are no `MouseEvent` arms in any of the loop matches
(`grep -rn "EnableMouseCapture\|MouseEvent" crates/anvil-cli crates/anvil-tui`
returns no hits as of HEAD `cf7ca040`).

This is deliberate at the moment — mouse capture interferes with terminal
text-selection, which users rely on for copying error output to issues — but the
trade-off is undocumented in code and the gap is invisible to maintainers who
haven't read this doc. The watch surface in particular would benefit from
optional mouse scroll on the history panel.

**Risk:** Low — keyboard navigation is comprehensive. **Fix:** if introduced,
gate behind a `--mouse` flag or `ANVIL_TUI_MOUSE=1` environment opt-in to
preserve text-selection by default.

### G-03: Animation tick busy-spin when dirty + not animating

The watch and watch-demo loops set `poll_timeout = Duration::ZERO` when the
surface is dirty but no animation is active (`tui.rs:264, 426-427`). This is
correct behaviour — drain whatever caused the dirty flag and paint immediately —
but in pathological cases (a high-rate engine event source that keeps marking
dirty without ever draining) the loop can busy-spin. There is no upper bound on
iteration rate, only on poll wait.

In practice the kernel watcher's emit rate is bounded by file-system event
coalescing, so this hasn't been observed in the wild. The mitigation when it
would matter is the 16 ms cap that kicks in once an animation starts.

**Risk:** Low for `v0.6.0-beta`. **Fix:** add a minimum frame interval (e.g., 4
ms) when dirty but not animating. Not tracked.

### G-04: Tutorial loop has no dirty gate

`tutorial_loop` paints unconditionally every iteration (`tui.rs:196-200`). The
tutorial surface is fast to render (mostly text) and the 100 ms poll keeps the
iteration rate bounded, so this isn't a performance issue. But it's an
inconsistency with `surface_loop` and `watch_loop` — same project, three
different paint policies.

**Risk:** Low. **Fix:** unify on the dirty-gate pattern in a future pass. Not
tracked.

### G-05: `WatchEventAdapter` channel-disconnect is silent

`while let Ok(engine_event) = event_rx.try_recv()` (`tui.rs:248`, `tui.rs:393`)
absorbs both `Empty` and `Disconnected`. If the kernel watcher's event-emission
thread dies, the receiver returns `Disconnected` forever and the watch dashboard
sits with stale state, no notification. The parent CLI command detects the dead
watcher when it calls `handle.stop()` at teardown, but during the live session
the surface gives no signal.

**Risk:** Low — kernel-watcher death is rare and the surface remains responsive
to keyboard input (so the user can quit). **Fix:** match `try_recv` explicitly
and on `Disconnected` push a `Notification::Failure` into the watch queue. Not
tracked.

### G-06: Windows console behaviour is implicit

The runner uses `crossterm`'s portable abstractions, so it works on Windows in
modern Terminal app builds. But there is no explicit Windows-console test, no
documented minimum console version, and the watch action footer's "ASCII-only"
rule (documented in `tui-as-built.md` §"Action footer") is the only place the
codebase acknowledges Windows code-page constraints. The `anvil-intercept-win32`
crate is the documented Windows-target dependency for the daemon path; the TUI
runner has no Windows-specific code.

**Risk:** Low — modern Windows Terminal supports everything the runner uses.
**Fix:** document a tested-terminals matrix when broader Windows support is
requested. Tracked tangentially in [`tui-as-built.md`](./tui-as-built.md) G-06.

## Source references

`tui.rs` exports nine functions plus `SurfaceExit`:

| Export              | Purpose                                                                | Lines            |
| ------------------- | ---------------------------------------------------------------------- | ---------------- |
| `SurfaceExit` enum  | `Quit` / `Back` exit contract                                          | `tui.rs:22-28`   |
| `run_surface<S>`    | Standard surface, owns terminal session                                | `tui.rs:33-47`   |
| `run_surface_in<S>` | Standard surface, shared terminal                                      | `tui.rs:51-57`   |
| `setup_terminal`    | Build a `Terminal` for the shared-terminal pattern                     | `tui.rs:61-67`   |
| `draw_loading`      | Transient loading frame inside shell chrome                            | `tui.rs:70-86`   |
| `teardown_terminal` | Restore terminal mode                                                  | `tui.rs:89-95`   |
| `run_tutorial`      | Tutorial surface, owns terminal, optional file channel                 | `tui.rs:161-178` |
| `run_tutorial_in`   | Tutorial surface, shared terminal                                      | `tui.rs:305-312` |
| `run_watch_demo`    | Watch demo, owns terminal, kernel events                               | `tui.rs:219-236` |
| `run_watch_demo_in` | Watch demo, shared terminal                                            | `tui.rs:315-322` |
| `run_watch`         | Watch dashboard, owns terminal, kernel events + action link + shutdown | `tui.rs:330-356` |
| `run_watch_in`      | Watch dashboard, shared terminal (no action / no shutdown)             | `tui.rs:362-374` |

Internal:

- `surface_loop` (`tui.rs:97-156`) — standard render loop
- `tutorial_loop` (`tui.rs:180-214`) — tutorial render loop
- `watch_demo_loop` (`tui.rs:238-299`) — watch-demo render loop
- `watch_loop` (`tui.rs:376-473`) — watch render loop with adapter, action link,
  shutdown
- `tick_animations` (`tui.rs:475-495`) — animation-tick driver

Call sites (twelve direct invocations across nine command modules):

| Caller                           | Function called                                                                                                                 | Notes                                                             |
| -------------------------------- | ------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------- |
| `commands/audit.rs:29`           | `tui::run_surface(state)`                                                                                                       | `anvil audit` interactive                                         |
| `commands/doctor.rs:45`          | `tui::run_surface(state)`                                                                                                       | `anvil doctor` interactive (re-entered after `apply_fix_request`) |
| `commands/init.rs:98`            | `tui::run_surface(state)`                                                                                                       | `anvil init` wizard                                               |
| `commands/new.rs:62`             | `tui::run_surface(state)`                                                                                                       | `anvil new` template browser                                      |
| `commands/status.rs:40`          | `tui::run_surface(state)`                                                                                                       | `anvil status` interactive                                        |
| `commands/wizard.rs:56`          | `tui::run_surface(state)`                                                                                                       | `anvil wizard`                                                    |
| `commands/tutorial.rs:78`        | `tui::run_tutorial(state, file_rx.as_ref())`                                                                                    | `anvil tutorial`                                                  |
| `commands/tutorial.rs:87`        | `tui::run_tutorial(state, file_rx.as_ref())`                                                                                    | tutorial re-entry after fix                                       |
| `commands/tutorial.rs:133`       | `tui::run_watch_demo(state, &event_rx)`                                                                                         | watch-demo from tutorial                                          |
| `commands/watch.rs:867`          | `tui::run_watch(state, &event_rx, link.as_ref(), Some(&shutdown))`                                                              | `anvil watch --tui=ratatui`                                       |
| `commands/welcome.rs` (multiple) | `setup_terminal`, `draw_loading`, `teardown_terminal`, `run_surface_in`, `run_tutorial_in`, `run_watch_demo_in`, `run_watch_in` | welcome hub composes everything                                   |

The welcome command (`crates/anvil-cli/src/commands/welcome.rs`) is the heaviest
consumer — it owns the shared-terminal pattern across onboarding, init,
discovery, tutorial, watch demo, the welcome menu, and every menu-launched
sub-surface (gate, audit, doctor, watch, tutorial, restart-onboarding). Lines
118-1030 of that file are essentially a state machine driven by `SurfaceExit`
returns.

## Related docs

- [`tui-as-built.md`](./tui-as-built.md) — the in-crate side of the dispatcher:
  `Surface` trait, surface inventory, snapshot infrastructure, watch / tutorial
  / welcome / wizard / onboarding deep dives. Read this alongside that doc.
- `docs/architecture/widgets-as-built.md` (in flight) — theme + keyboard
  plumbing (`EddaCraftTheme`, `KeyHandler::map`) consumed by every loop variant
  here.
- [`activation-as-built.md`](./activation-as-built.md) — `anvil start` runs the
  activation orchestrator, not this runner; but the watch-fallback path it
  spawns reuses the `--tui=ratatui` watch surface, which mounts through
  `run_watch` here.
- `docs/architecture/tutorial-as-built.md` (in flight) — `anvil tutorial` mounts
  the tutorial surface through `run_tutorial` / `run_tutorial_in` here; the
  WELCOME-013 file-watch integration and the WELCOME-014 watch-demo transition
  are visible in `tutorial_loop`.
- [`kernel-as-built.md`](./kernel-as-built.md) — KERN-033 `EngineEvent` protocol
  consumed by `watch_loop` and `watch_demo_loop` via `WatchEventAdapter`.
