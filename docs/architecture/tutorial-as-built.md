# Tutorial Subsystem — As-Built

| Type     | Authority | Owner  | Status | Freshness                                                                                                                                                                                                                                                                                          |
| -------- | --------- | ------ | ------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| As-built | Derived   | LAUNCH | Live   | Last reviewed 2026-07-02 (targeted delta review: all-paths honesty pin closing G-05, Policy-path Rego refresh correcting G-01, editor-name de-hardcode); prior 2026-06-10 (showcase wiring, SCAN-004 provenance, pin sweep) against main `45dd1047a`; full review 2026-05-07 against `v0.6.0-beta` |

| Upstream                               | Downstream                                                                                |
| -------------------------------------- | ----------------------------------------------------------------------------------------- |
| `crates/anvil-tui`, `crates/anvil-cli` | anvil tutorial CLI command, welcome surface "Choose a learning path" entry point (LAUNCH) |

> **Status:** Live (beta) **Last reviewed:** 2026-06-10 (targeted delta review:
> showcase wiring, SCAN-004 provenance, pin sweep) against main `45dd1047a`;
> full review 2026-05-07 against `v0.6.0-beta` slate (HEAD `cf7ca040`) **Module
> / location:** `crates/anvil-tui/src/surfaces/tutorial/` **Module owner
> (APS):** LAUNCH-014 (ProtectionLoop default reframe — Complete) **Used by:**
> `anvil tutorial` CLI command (mounts the tutorial surface)

The UJ-012..015 docs-site tutorials (`docs/public/anvil/tutorials/*`) are a
separate artefact from this TUI engine and do not affect it — stated here
explicitly so the next reviewer doesn't re-chase that lead.

## Overview

The tutorial subsystem is the multi-file tutorial engine inside `anvil-tui` that
drives the LAUNCH-014 ProtectionLoop default path plus four legacy paths (Policy
/ Architecture / Drift / CI), with test-pinned copy invariants enforcing the
activation state vocabulary. The TUI as-built
([`tui-as-built.md`](./tui-as-built.md)) covers tutorial only at the surface
level. This doc dives into the multi-file engine — paths, discovery, executor,
fix, render, showcase, verify, watch_demo, copy invariants — that sits behind
`anvil tutorial` and the `welcome` "Choose a learning path" entry point.

The subsystem is the only non-watch TUI surface that consumes a live event
channel (the kernel watcher feeds `handle_file_change` for steps with a
`watch_path`), and it is the canonical home of the test-pinned copy that teaches
the activation-state vocabulary documented in
[`activation-as-built.md`](./activation-as-built.md). The honesty contract is
load-bearing: the tutorial does NOT claim pre-write protection — only
`anvil start --verify` is allowed to produce a literal `ProtectionState`.

## Architecture diagram

```text
┌──────────────────┐
│ anvil tutorial   │  CLI entry — crates/anvil-cli/src/commands/tutorial.rs
└─────────┬────────┘
          │   load progress (~/.anvil/tutorial-progress.json)
          │   start kernel watcher (or fall back to static mode)
          │
          ▼
┌──────────────────┐         ┌─────────────────┐
│ TutorialState    │◀────────│ paths.rs        │
│ (mod.rs)         │ steps   │ ProtectionLoop  │
│ phase: PathSelect│         │ Policy / Arch   │
│      → Running   │         │ Drift / CI      │
│      → Complete  │         └─────────────────┘
└────┬────────┬────┘
     │        │ on Select w/ command
     │        ▼
     │  ┌──────────────┐    CommandOutput   ┌──────────┐
     │  │ executor.rs  │──────────────────▶ │ verify.rs│
     │  │ sh -c / cmd  │                    │ FileExist│
     │  └──────────────┘                    │ ExitCode │
     │                                      │ OutputCo │
     │  on file event                       └────┬─────┘
     │  ┌──────────────┐                         │
     ├─▶│ kernel       │                         │ Pass/Fail
     │  │ watcher      │──── ChangeBatch ────────┘
     │  └──────────────┘
     │
     │  on watch_demo step
     │  ┌──────────────┐    EngineEvent     ┌──────────┐
     ├─▶│ watch_demo.rs│◀──── kernel ───────│ run_watch│
     │  │ overlay/hint │                    └──────────┘
     │  └──────────────┘
     │
     │  on render
     ▼
┌──────────────────┐
│ render.rs        │   path-select  /  running  /  complete
│   ↳ delegates to:│
│   discovery_render.rs   (ScanResults panel)
│   fix_render.rs         (per-step fix surface)
│   watch_demo_render.rs  (overlay-on-watch-grid)
│   showcase.rs           (curated example findings)
└──────────────────┘

  Final ProtectionLoop step ──▶  shells out: `anvil start --verify`
                                 (read-only verifier in
                                 crates/anvil-cli/src/activation/)
```

## Path inventory (`paths.rs`)

The `TutorialPath` enum (`mod.rs:36-44`) declares five paths. The first slot is
the LAUNCH-014 value-first default; the remaining four are the deeper-learning
track.

| Path           | Variant idx | Step count | Audience                                                                                        | Citation           |
| -------------- | ----------- | ---------- | ----------------------------------------------------------------------------------------------- | ------------------ |
| ProtectionLoop | 0 (default) | 5          | New user looking for the 60-second walk + the "do this next" pointer at `anvil start --verify`. | `paths.rs:107-136` |
| Policy         | 1 (legacy)  | 6          | User who wants to learn the policy/findings taxonomy by writing a `.anvil/policies/` rule.      | `paths.rs:138-183` |
| Architecture   | 2 (legacy)  | 6          | User who wants the boundary-violation walk through `anvil architecture validate`.               | `paths.rs:185-225` |
| Drift          | 3 (legacy)  | 6          | User who wants to learn `anvil drift snapshot/compare` plus the watch-demo handoff.             | `paths.rs:227-268` |
| CI             | 4 (legacy)  | 6          | User who wants the hooks + GitHub Actions integration walkthrough.                              | `paths.rs:270-306` |

`TutorialPath::label()`/`description()` are the canonical strings rendered in
the path selector (`mod.rs:46-83`). `from_label` round-trips legacy labels
("Policy", "Architecture", "Drift", "CI Integration") so progress files written
by older builds still resolve to the correct enum variant after the onboarding
rename (`mod.rs:57-70`); a round-trip test for ProtectionLoop pins the new label
(`mod.rs:958-965`).

The deep dive on the ProtectionLoop walk is in §6 below.

## State machine (`mod.rs`)

`TutorialState` (`mod.rs:131-162`) carries everything the surface needs:

- `phase: TutorialPhase` — `PathSelect | Running | Complete` (`mod.rs:87-92`)
- `paths: Vec<TutorialPath>` and `path_selected: usize` — what the picker shows
  (`mod.rs:172-178`; ProtectionLoop is index 0 by construction).
- `steps: Vec<TutorialStep>` and `current_step: usize` — the loaded path's walk
  (filled by `load_steps`, `mod.rs:262-274`).
- `static_mode: bool` + `static_notice: Option<String>` — set when the kernel
  watcher cannot start; disables command execution and turns every step into
  press-enter-to-continue (`mod.rs:148-152, 199-210`).
- `completed_paths: Vec<TutorialPath>` — persisted across sessions; the picker
  uses these to render the checkmark + `(redo)` suffix
  (`mod.rs:153-156, 214-216`; renderer at `render.rs:308-321`).
- `resuming_notice: Option<String>` — transient banner shown after a resumed
  session; cleared on first user advance (`mod.rs:155-156, 372`).
- `wants_watch_demo: bool` — handshake with the CLI command for the watch demo
  handoff (`mod.rs:159, 456-457`).
- `pending_fix: Option<FixRequest>` — the cross-surface fix envelope; when set,
  `should_quit()` returns true so the CLI command can apply the fix and re-enter
  (`mod.rs:160-161, 542`).

### Phase transitions

`PathSelect → Running` happens when the user presses Enter on the picker
(`handle_path_select`, `mod.rs:351-368`); `load_steps` materialises the path's
steps and computes `domain_findings` from the threaded `scan_results`
(`mod.rs:262-274`).

`Running → Complete` happens when `advance_step` walks past the final step
(`mod.rs:370-381`). `advance_step` also clears any `resuming_notice` on first
interaction.

`Complete → PathSelect` happens when the user presses Enter on the completion
screen (`mod.rs:498-506`); the steps and chosen path are cleared.

### Per-step lifecycle

For each step in `Running`, `handle_running` (`mod.rs:415-496`) dispatches:

- **Failed-step branch** (`current_step_failed()` returns true). Only `r` retry,
  `s` skip, `Esc`, and `q` are active. Retry clears `output`/`verify_result` and
  re-runs `executor::execute_command`. Skip marks the step complete and advances
  (`mod.rs:419-445`).
- **Watch-demo branch.** If the step's `watch_demo: true`, Enter sets
  `wants_watch_demo` and the TUI loop exits — the CLI command launches
  `WatchDemoState` and resumes the tutorial afterwards (`mod.rs:453-457`; CLI
  handoff at `crates/anvil-cli/src/commands/tutorial.rs:82-88, 97-138`).
- **Command branch.** If the step has a `command`, Enter runs it through the
  executor, stores the output, runs `verify` (if any), and advances on success
  (`mod.rs:458-472`).
- **Informational branch.** Steps without a command advance immediately on
  Enter; `Toggle` (space) also advances them but is **ignored** for command
  steps to prevent accidental shell invocation (`mod.rs:468-485`).
- **Fix branch.** Pressing `f` runs `next_fix_request()` against
  `domain_findings` (highest-severity first) and stores it in `pending_fix`
  (`mod.rs:246-260, 487-490`).

### `--reset` handling

`anvil tutorial --reset` deletes `~/.anvil/tutorial-progress.json` and returns;
it never enters the TUI (`tutorial.rs:37-39, 162-168`). On normal launch,
`load_progress` parses the file and `set_completed_paths` plus optional
`resume_path` rebuild the in-progress UI state (`tutorial.rs:46-64`).

## Discovery (`discovery.rs` + `discovery_render.rs`)

The discovery subsurface is the scan-results-to-tutorial bridge. Combined ~1714
lines: 933 in `discovery.rs` (state machine, finding types, filtering, ~30 unit
tests) and 781 in `discovery_render.rs` (three-phase render).

### Domain types (`discovery.rs:13-103`)

- `FindingSeverity` — `Info | Warning | Error`, ordered ascending so derived
  `Ord` puts `Error` highest (`discovery.rs:20-25`).
- `FindingSource` — `Architecture | AntiPattern | Secret`
  (`discovery.rs:28-33`).
- `Finding` — file/line/severity/source/title/message/suggestion, plus an
  optional `warning_id`. `fix_request()` maps known anti-pattern IDs (`AP-001`,
  `AP-003`, `AP-004`) to a `FixRequest::AntiPatternWarning` envelope; everything
  else returns `None` (`discovery.rs:56-85`).
- `ScanResults` — bag of findings + `files_scanned` + `duration_ms` +
  `truncated` flag + `files_skipped_by_ignore: usize` (`discovery.rs:89-103`).
  The fifth field is SCAN-004 gitignore provenance: the count of files that
  matched the scan criteria but were skipped because `.gitignore` excluded them.
  It is preserved across `filter_by_domain` (`discovery.rs:148`, pinned by the
  test at `discovery.rs:868`) and surfaced as a "skipped by .gitignore — set
  ANVIL_SCAN_ALL=1" advisory row on the continue screen
  (`discovery_render.rs:361-390`).

### Filtering (`discovery.rs:125-150`)

`ScanResults::filter_by_domain(path)` is the seam between scan output and the
tutorial path:

- `ProtectionLoop` — all findings (LAUNCH-014 v1 default; "downstream PRs may
  narrow this to a high-signal subset, but blanket inclusion is the honest v1
  default", doc-comment at `discovery.rs:118-120`).
- `Policy` — `AntiPattern + Secret` only.
- `Architecture` — `Architecture` only.
- `Drift` and `CI` — all findings (cross-cutting).

The ProtectionLoop / Drift / CI cases are now a single collapsed match arm
(`discovery.rs:139-141`) — semantics unchanged.

### `DiscoveryState` (a separate surface, `discovery.rs:157-359`)

`DiscoveryState` is its own `Surface` impl:
`Scanning { files_scanned, spinner_tick }` → `Results { selected }` (or →
`Continue` on no findings). Scanning is driven externally — the caller invokes
`update_progress`, `tick`, and `set_results`. The `s` key skips the scan from
the scanning phase; `set_results` is a no-op once results have been written (the
"results are never overwritten" invariant, `discovery.rs:213-227`).

### Rendering split (`discovery_render.rs`)

Three render paths, dispatched on `DiscoveryPhase`:

- `render_scanning` — bordered block + Braille spinner (`SPINNER_FRAMES`,
  `discovery_render.rs:11`) + file count + "Press 's' to skip"
  (`discovery_render.rs:42-85`).
- `render_results` — two-panel horizontal split (50/50): findings list on the
  left, finding-detail on the right with viewport scrolling
  (`discovery_render.rs:87-310`).
- `render_continue` — summary screen with severity counts
  (`discovery_render.rs:311-402`). When `files_skipped_by_ignore > 0`, an
  advisory row ("N files skipped by .gitignore — set ANVIL_SCAN_ALL=1 …") is
  appended below the summary line (`discovery_render.rs:361-390`).

`TutorialState` consumes the discovery output via `set_scan_results` and
`load_steps` (which calls `filter_by_domain`); the welcome / onboarding flows
wire the two together (`mod.rs:242-274`).

## The ProtectionLoop default path (deep dive)

LAUNCH-014's value-first walk lives in `paths::protection_loop_steps`
(`paths.rs:107-136`). Five steps:

1. **"Anvil's protection loop in 60 seconds"** — informational. Frames the scan
   → surface → react loop and tells the user the walk is on a fixture, not their
   repo (`paths.rs:109-113`).
2. **"What we'll check"** — informational. Describes the fixture: a tiny
   TypeScript file with `// @ts-ignore` and `: any`, both catalogued as
   escape-hatch findings (`paths.rs:114-118`).
3. **"Run the check (simulated)"** — informational. Renders a simulated
   catalogue result with `[AP-004]` and `[AP-003]` finding IDs. **No network
   call, no telemetry, no fixture deployed** — explicit honesty line
   (`paths.rs:119-123`).
4. **"What protection actually means here"** — informational. Lists all five
   user-actionable activation states with their meanings: `protecting`,
   `ready_restart_required`, `watching`, `needs_action`, `unsupported`. Calls
   out that the tutorial does NOT promote any state on its own and that
   activation does not imply the repo is clean of findings (LAUNCH-010 baseline
   framing, `paths.rs:124-128`).
5. **"Activate in this repo"** — command step. Runs `anvil start --verify`, the
   read-only activation diagnostic. The instruction copy notes that
   watch-fallback liveness probing is not yet wired (the verifier reports
   `watch: not requested` until a future PR introspects a running watcher) and
   that mutating activation is `anvil start` without `--verify`
   (`paths.rs:129-134`).

The path's source comments document the load-bearing copy invariants
(`paths.rs:87-106`):

- The headline never says "protected", "protecting", or "pre-write" without
  `anvil start --verify` evidence in the same step.
- The vocabulary lines reference each state literal by name so the user
  recognises them when the verifier prints one.
- "Future changes are checked" is the LAUNCH-010 baseline copy and lands
  honestly here regardless of whether the user has run `anvil start` yet.

The state vocabulary is the one defined in
[`activation-as-built.md`](./activation-as-built.md) (the `ProtectionState` enum
at `crates/anvil-cli/src/activation/state.rs`). Cross-link is load-bearing: a
rename in either place breaks the test pin in §13.

## Executor (`executor.rs`)

A single function: `execute_command(&str) -> CommandOutput`
(`executor.rs:21-54`). It hands the command to the platform shell (`sh -c` on
Unix, `cmd /C` on Windows), captures stdout/stderr/exit-code, and returns
synchronously. The function is `pub(crate)` so commands cannot be fed from
outside the tutorial module — the safety contract requires command strings to
come from the fixed allow-list in `paths.rs`, never from user input
(`executor.rs:10-14`).

Spawn errors yield a synthetic `CommandOutput` with `success: false`,
`exit_code: None`, and `stderr: "failed to spawn process: <err>"` so the caller
has a uniform shape (`executor.rs:46-52`).

The function is blocking — commands are expected to complete sub-second.
WELCOME-013 added the file-watcher seam so most interactive verification runs
through `handle_file_change` (re-verify on save) rather than blocking the loop
on a spawned process (`executor.rs:16-20`).

## Fix step (`fix.rs` + `fix_render.rs`)

`fix.rs` (~976 lines) defines `FixState`, a self-contained sub-surface for a
single finding. Four phases (`fix.rs:18-27`):

- `Watching` — waiting for an external editor to save the file.
- `Editing` — inline editor open (the EddaCraft `EditorState` widget).
- `Resolved` — the post-edit re-check passed.
- `TimedOut` — 600 ticks (~60 s) elapsed without a fix.

It is **not** the tutorial's per-step fix dispatcher; it is the surface that the
welcome-flow fix loop launches when the user presses `f` against a finding. The
tutorial itself emits `FixRequest` envelopes (via `TutorialState::pending_fix`)
which the CLI command applies through
`services::interactive_fix::apply_fix_request` — the same cross-surface seam
used by doctor and audit (see [`tui-as-built.md`](./tui-as-built.md)'s
`surfaces/fix_request.rs` section).

`FixState` is a pure state machine — file watching, I/O, and check execution are
driven externally via `set_context`, `notify_file_changed`, `set_check_result`,
and `tick` (`fix.rs:36-148`). The inline editor is gated on `editor_disabled`
(set by callers that cannot drive the editor save/check loop, e.g. the welcome
flow, `fix.rs:73-75`).

`fix_render.rs` (440 lines) draws the file-context panel, finding detail, and
phase-aware help footer.

## Verify step (`verify.rs`)

`verify.rs` (174 lines) is the verification primitive used by tutorial steps —
**not** the activation verifier. Three variants (`verify.rs:5-16`):

- `Verify::FileExists(String)` — uses `Path::exists()`.
- `Verify::ExitCode(i32)` — matches `output.exit_code`.
- `Verify::OutputContains(String)` — substring against stdout **or** stderr;
  `str::contains` is used deliberately to avoid pulling in the `regex` crate
  (`verify.rs:11-15`).

`Verify::check(&CommandOutput) -> VerifyResult` returns `Pass` or `Fail(String)`
with a contextual hint (`verify.rs:27-58`).

The cross-surface activation verify path is **separate**. Step 5 of the
ProtectionLoop walk is a `step_with_command` that shells `anvil start --verify`
via the executor (`paths.rs:129-134`). The verify diagnostic itself lives in
`crates/anvil-cli/src/activation/` (see
[`activation-as-built.md`](./activation-as-built.md)) and runs in a child
process — the tutorial does NOT call into the embedded activation path
in-process. This is deliberate: the tutorial captures whatever the real verifier
prints, including the literal `ProtectionState` line, and the user sees exactly
what they would see running the command outside the tutorial.

## Watch demo (`watch_demo.rs` + `watch_demo_render.rs`)

The watch demo is a separate surface launched from the Drift path's "Watch Mode
Demo" step (`paths.rs:254-261`, the only step with `watch_demo: true`) and from
any future step that flips the flag.

`WatchDemoState` (`watch_demo.rs:44-218`) wraps a real `WatchData` (the same
payload the live watch dashboard uses) and consumes real `EngineEvent`s — it is
**not** simulated. The CLI command starts a kernel watcher rooted at the
workspace, pipes events into the demo state, and shuts the watcher down when the
demo exits (`crates/anvil-cli/src/commands/tutorial.rs:97-138`).

A guided overlay
(`OverlayPhase::{Intro, Hint1, Hint2, Hint3, CycleComplete, Dismissed}`,
`watch_demo.rs:24-38`) auto-advances on a 10/20/30-second schedule until the
user witnesses a full cycle (≥2 snapshots — initial scan + one triggered by a
file change), at which point it flips to `CycleComplete` and offers
Enter-to-continue (`watch_demo.rs:88-130`). The overlay reveal is
`animate::Once`-driven for smooth fade-in/out (`watch_demo.rs:14-21, 169-181`).

Help text is overlay-aware (`watch_demo.rs:209-217`). `watch_demo_render.rs`
(135 lines) overlays the panel on top of the watch grid.

The watch-demo handoff back to the tutorial: the tutorial loop exits with
`wants_watch_demo = true`, the CLI command runs `run_watch_demo_for_tutorial`,
and on clean exit advances the tutorial step
(`crates/anvil-cli/src/commands/tutorial.rs:82-88`).

## Showcase (`showcase.rs`)

`showcase.rs` (145 lines) returns four curated example findings — one hard-coded
API key (`Secret`/`Error`), one TODO in production code
(`AntiPattern`/`Warning`), one cross-layer import (`Architecture`/`Warning`),
and one camelCase naming convention (`AntiPattern`/`Info`). Each title is
prefixed with `[Example]` so the renderer can distinguish them from real
findings (`showcase.rs:7-73`).

The showcase is wired through the welcome discovery flow: when `scan_project()`
returns zero findings, `crates/anvil-cli/src/commands/welcome.rs:340-353` swaps
in `showcase::showcase_findings()` so a clean repo still teaches what Anvil can
catch, and the same swap is the scan-error fallback (`welcome.rs:356-368`). The
clean-repo swap deliberately preserves the real scan's `files_skipped_by_ignore`
provenance (SCAN-004) rather than zeroing it — the skipped files were still
skipped. The `TODO(WELCOME-007)` that used to sit at the top of the file is
gone; the entry point is `showcase_findings()`
(`crates/anvil-tui/src/surfaces/tutorial/showcase.rs:11`). G-04 is resolved.

## Render path (`render.rs`)

`render.rs` (~1201 lines) is the main tutorial-frame renderer. Public entry is
`render(frame, area, state, theme)` at `render.rs:118-165`, which dispatches on
`state.phase` to:

- `render_path_select` (`render.rs:268-349`) — the bordered "Choose a Learning
  Path" picker. Computes `path_select_box_height` from word-wrapped path-line
  widths so paths are not silently clipped at IDE-side-panel widths
  (`render.rs:215-266`).
- `render_step_progress` (`render.rs:351-403`) — a two-line header: path label +
  "Step N of M", then a row of progress glyphs (filled / current / pending) per
  step.
- `render_step_content` (`render.rs:405-521`) — the main step body
  (description + instruction + watch-hint + captured stdout/stderr, truncated to
  `MAX_OUTPUT_LINES = 5`). Bails out below 4×3 area to avoid divide-by-zero in
  `Paragraph` wrap (`render.rs:415-417`). ANSI sequences in captured output are
  stripped (`strip_ansi`, `render.rs:71-104`).
- `render_complete` (`render.rs:551-651`) — the "Well Done" screen with per-path
  checkmark progress and a "Up next:" pointer to the next unfinished path.

### Affordances

- **Glyph fallback.** `progress_glyphs()` returns ASCII glyphs (`#`, `>`, `-`)
  when `ANVIL_ASCII=1` is set, otherwise Unicode geometric shapes
  (`render.rs:22-35`). Older Windows consoles and a few SSH multiplexers
  rendered the geometric shapes as double-wide or as replacement boxes; the env
  var is the escape hatch.
- **Title fitting.** `fit_block_title` truncates step titles with an ellipsis so
  they never punch through the border, computed against Unicode display width
  (`render.rs:41-64`).
- **Notice rows.** Static-mode and resuming notices reserve dynamic row counts
  via `notice_row_count` so a wrapped notice never silently clips on narrow
  widths (`render.rs:172-179`; the LAUNCH-009 fix for the watcher-unavailable
  notice clipping).
- **No zoom controls.** Unlike the watch dashboard, the tutorial does not expose
  a zoom key — there is no `z` handler in `handle_running` /
  `handle_path_select` / `handle_complete` (`mod.rs:343-511`). Zoom in the
  tutorial is currently a non-goal; the bordered-block layout is the only view.

## Copy invariants (the LAUNCH-014 tests)

Two test-pinned copy invariants live in `tutorial::tests` (`mod.rs`):

### `protection_loop_copy_uses_activation_state_vocabulary`

Location: `mod.rs:882-909`. Loads the ProtectionLoop steps, joins title +
description + instruction across all five into a single body, and asserts the
body contains every one of the five user-actionable activation-state literals:
`protecting`, `ready_restart_required`, `watching`, `needs_action`,
`unsupported`. Failure message names which literal is missing.

This protects the cross-surface vocabulary contract with
[`activation-as-built.md`](./activation-as-built.md): if the activation verifier
prints `state: needs_action`, the user must have seen that exact word in the
tutorial. Renaming a state in either place breaks the pin.

### `protection_loop_copy_does_not_claim_pre_write_protection`

Location: `mod.rs:912-955`. Lower-cases the joined body and asserts none of the
following forbidden phrases appear:

- `you are now protected`
- `you're now protected`
- `your repo is protected`
- `pre-write validation enabled`
- `pre-write validation active`
- `anvil is now intercepting`

It then asserts the body **does** contain `anvil start --verify` — the final
step must direct users at the only surface allowed to produce a literal
`ProtectionState`. The pin allows the bare state literal `protecting`
(referenced in step 4's vocabulary explainer) but rejects present-tense
protection claims about the user's repo.

### Other LAUNCH-014 pins

- `protection_loop_path_is_default_first_path` (`mod.rs:872-879`) — pins the
  load-bearing UX invariant that Enter on a fresh tutorial lands on
  ProtectionLoop, not Policy.
- `path_selection_advances_to_running` (`mod.rs:740-751`) — pins the same
  property at the Action level.
- `protection_loop_round_trips_through_label` (`mod.rs:958-965`) — pins the
  resumption seam for the new label.
- `no_path_claims_pre_write_protection` — extends the forbidden-phrase honesty
  guard across **all five** paths (not just ProtectionLoop), so copy drift on
  any legacy path that re-introduces a present-tense protection claim now fails
  CI. Closes G-05.

## Snapshot pinning (`snapshots/`)

Ten insta snapshot files, all named
`anvil_tui__surfaces__tutorial__render__tests__snapshot_<phase>[_<size>].snap`:

| Snapshot                                      | Pins                                                                                                           |
| --------------------------------------------- | -------------------------------------------------------------------------------------------------------------- |
| `snapshot_path_select.snap`                   | Default 80×24 path selector — five paths, ProtectionLoop pre-selected                                          |
| `snapshot_path_select_narrow_40x10.snap`      | Narrow IDE-side-panel layout (40×10) — wrapped path lines stay visible                                         |
| `snapshot_path_select_tiny_20x10.snap`        | Tiny terminal pin (20×10) — path selector survives extreme widths                                              |
| `snapshot_running_phase.snap`                 | Default 80×24 running-phase render — header + progress + step body                                             |
| `snapshot_running_phase_narrow_40x10.snap`    | Running phase at 40×10                                                                                         |
| `snapshot_running_phase_tiny_20x10.snap`      | Running phase at 20×10 — `render_step_content` early-bail kicks in just above this                             |
| `snapshot_running_static_mode.snap`           | Running phase with `static_mode = true` and the watcher-unavailable notice rendered as a wrapped warning strip |
| `snapshot_complete_phase.snap`                | "Well Done" screen for a single completed path                                                                 |
| `snapshot_complete_phase_multiple_paths.snap` | "Well Done" with two of five paths complete — checkmark + "Up next:" pointer                                   |
| `snapshot_complete_phase_all_paths.snap`      | "Well Done" when every path is complete — replaces "Up next:" with the all-done branch                         |

The pattern: per-phase pins at the default size, narrow (40×10), and tiny
(20×10) for the two phases where layout collapse matters most (`PathSelect` and
`Running`); plus the static-mode variant and three complete-phase variants that
exercise the `next_path` branch and the all-paths-done branch in
`render_complete`.

The snapshot tests are in `render.rs::tests` (`render.rs:772-1118`); each test
builds a `TutorialState`, renders into a `TestBackend`, and asserts via
`insta::assert_snapshot!`.

## Cross-cutting concerns

### Determinism

Same scan results → same `domain_findings` (filter is pure,
`discovery.rs:125-150`) → same step body. The fix-request priority is
deterministic: `next_fix_request` picks the highest-severity fixable finding
(`mod.rs:246-260`). The progress-glyph choice is stable for a given
`ANVIL_ASCII` value. Snapshot pins are the canonical determinism gate.

### Honesty contract

The tutorial does NOT claim pre-write protection. The two copy invariants in §13
are the test-enforced version of the honesty contract: state-vocabulary words
must be present so users recognise them later, and present-tense protection
claims about the user's repo must not appear. The final step must point at
`anvil start --verify` — the only surface that produces a literal
`ProtectionState`.

The fixture-based simulation step (`paths.rs:119-123`) explicitly says "No
network call, no telemetry, no fixture deployed to your repo" so the user knows
nothing has happened yet on their codebase.

### No external network / no cloud calls

The tutorial runs offline. The only commands the executor ever runs come from
the path definitions in `paths.rs` (allow-list documented at
`executor.rs:10-14`), and none of them require network. The watch-demo surface
uses the embedded kernel watcher only — same workspace root, no remote
connection.

### Notification source

`TutorialState` implements `NotificationSource` (`mod.rs:581-654`). It emits:

- `Warning/High` for the static-mode notice (`mod.rs:585-593`).
- `Info/Normal` for the resume notice (`mod.rs:595-603`).
- `Failure/High` for command failures and verify failures while a non-completed
  step is current (`mod.rs:609-650`).

Two security-relevant invariants are pinned by tests (`mod.rs:1751-1843`):

- `notifications_never_echo_stderr_contents` — failed-command notifications must
  not embed the step's stderr (CWE-209: stderr routinely contains absolute
  paths, credential-helper output, `$HOME`/username fragments). The notification
  message is the sanitised `"<title> failed with exit code N"`
  (`mod.rs:622-626`).
- `notifications_suppressed_after_verify_fail_skip_complete` — once the user has
  skipped a failed step and the phase has flipped to `Complete`, the stale
  `verify_result` must not re-surface as a live failure (adversarial F-002).

### Reset path

`anvil tutorial --reset` deletes `~/.anvil/tutorial-progress.json`
(`tutorial.rs:162-168`). The TUI is not entered. Subsequent launches start with
no completed paths and no in-progress session.

`Surface::reset` on `TutorialState` (`mod.rs:549-563`) is the orthogonal
in-process reset used by the welcome dispatcher when navigating between
surfaces; it preserves `static_mode`, `static_notice`, and `completed_paths`
deliberately because they represent environment / session state rather than
transient UI.

## Known gaps

### G-01: Legacy paths are stale relative to v0.6.0-beta concepts

The four legacy paths (Policy / Architecture / Drift / CI) were written before
the LAUNCH series reframed activation. Concrete drift visible in the source
today (`paths.rs:138-306`):

- **Policy** — since refreshed (2026-07-02 review; execution fix 2026-08-12):
  the intro frames policies as Rego (`.rego`) files, and the walk creates
  `.anvil/policies/no-todos.rego`. Step 4 ("Test the Policy") runs
  `anvil policy test`, which executes pack and free-form `*_test.rego` files
  in-process and exits non-zero on failure. The earlier "declarative YAML" /
  `anvil doctor` drift is gone.
- **Architecture** asks the user to create `.anvil/architecture.yaml` with layer
  definitions (`paths.rs:194-197`). The "Choose a Template" language ("layered,
  hexagonal, modular") describes a template catalog the v0.6.0-beta CLI does not
  ship.
- **Drift** uses `anvil drift snapshot --name baseline / current` and
  `anvil drift compare baseline current` (`paths.rs:236-252`). The drift
  subsystem is in scope but the tutorial walk is not validated against the
  current command surface in this release; commands run through the executor but
  no verifier checks output shape.
- **CI** mentions `--husky` and Git 2.54+ `--config` flags (`paths.rs:278-281`);
  structured exit codes are listed verbatim (`paths.rs:288-290`). These need a
  v0.6.0-beta sweep.

**Risk:** Medium for a new user choosing a legacy path. **Fix:** Targeted copy +
verifier updates against the v0.6.0-beta CLI surface, with new test pins
matching the LAUNCH-014 vocabulary pattern. Tracked through LAUNCH-014's "future
PRs may narrow this to a high-signal subset" lane in
`plans/archive/modules/launch-flow-readiness.aps.md`.

### G-02: Step 3 of ProtectionLoop is a literal simulation, not real execution

"Run the check (simulated)" prints the finding catalogue text inline
(`paths.rs:119-123`) rather than invoking a real check. This is by design (no
fixture is ever deployed) but means the user doesn't see actual `anvil` output
until step 5. **Risk:** Low. **Fix:** None planned for v0.6.0-beta — the
simulation is honest and the final step provides the real-output payoff.

### G-03: Watch-fallback liveness probe is not yet wired

ProtectionLoop step 5's instruction copy says "Watch-fallback liveness probing
is not yet wired; the verifier reports `watch: not requested` until a future PR
introspects a running watcher" (`paths.rs:131`). Pinned upstream against the
activation orchestrator, not this crate. The tutorial reader sees the gap
honestly. **Risk:** Low (UX honesty is preserved). **Fix:** Tracked in the
activation crate; tutorial copy auto-corrects when the verifier output changes.

### G-04: Showcase wiring into the discovery flow (resolved)

Resolved (welcome.rs wiring, 2026-06-10 review) — see §Showcase.

### G-05: Legacy-path copy honesty pin (resolved — forbidden-phrase half)

Previously the two LAUNCH-014 pins
(`protection_loop_copy_uses_activation_state_vocabulary`,
`protection_loop_copy_does_not_claim_pre_write_protection`) only covered the
ProtectionLoop body, so Policy / Architecture / Drift / CI could re-introduce a
"you are now protected"-style claim without CI noticing.

**Resolved (2026-07-02):** `no_path_claims_pre_write_protection` now runs the
forbidden-phrase guard across all five paths. The remaining, optional half —
requiring each legacy path's body to direct users at a per-path
verifier-equivalent — is deferred; it is a copy-quality nicety, not a honesty
risk, now that the false-claim guard covers every path.

### G-06: No `z` zoom in the tutorial surface

The watch dashboard supports `z` to zoom; the tutorial does not
(`mod.rs:343-511`). For the running-phase step body this is fine (content is a
single block), but the path-select picker can wrap awkwardly on narrow widths
even with the `path_select_box_height` fix. **Risk:** Low (the layout works,
just isn't fluid). **Fix:** Not planned; flagged for completeness.

## Source references

| File                                                          | Lines    | Role                                                                                                                                       |
| ------------------------------------------------------------- | -------- | ------------------------------------------------------------------------------------------------------------------------------------------ |
| `crates/anvil-tui/src/surfaces/tutorial/mod.rs`               | 1846     | Module surface — `TutorialState`, `TutorialPath`, `TutorialPhase`, `TutorialStep`, `STATIC_MODE_WATCHER_UNAVAILABLE`, copy-invariant tests |
| `crates/anvil-tui/src/surfaces/tutorial/paths.rs`             | 584      | Path definitions — `protection_loop_steps`, `policy_steps`, `architecture_steps`, `drift_steps`, `ci_steps`                                |
| `crates/anvil-tui/src/surfaces/tutorial/discovery.rs`         | 933      | Discovery state machine, `Finding`, `FindingSeverity`, `FindingSource`, `ScanResults`, domain filtering                                    |
| `crates/anvil-tui/src/surfaces/tutorial/discovery_render.rs`  | 781      | Discovery scanning / results / continue render paths                                                                                       |
| `crates/anvil-tui/src/surfaces/tutorial/executor.rs`          | 109      | `execute_command` shell wrapper (allow-listed by `pub(crate)`)                                                                             |
| `crates/anvil-tui/src/surfaces/tutorial/fix.rs`               | 976      | `FixState` finding-fix surface (Watching / Editing / Resolved / TimedOut)                                                                  |
| `crates/anvil-tui/src/surfaces/tutorial/fix_render.rs`        | 440      | Fix render — file context + finding detail + phase-aware footer                                                                            |
| `crates/anvil-tui/src/surfaces/tutorial/render.rs`            | 1201     | Main tutorial render — path-select, step-progress, step-content, complete + 10 snapshot tests                                              |
| `crates/anvil-tui/src/surfaces/tutorial/showcase.rs`          | 145      | Curated `[Example]` findings for the zero-findings case (wired via the welcome discovery flow — see §Showcase)                             |
| `crates/anvil-tui/src/surfaces/tutorial/verify.rs`            | 174      | `Verify::FileExists`, `Verify::ExitCode`, `Verify::OutputContains`, `VerifyResult`                                                         |
| `crates/anvil-tui/src/surfaces/tutorial/watch_demo.rs`        | 319      | `WatchDemoState` — overlay-on-watch-grid demo with auto-advancing hints                                                                    |
| `crates/anvil-tui/src/surfaces/tutorial/watch_demo_render.rs` | 135      | Watch demo render — overlay panel composited on the watch grid                                                                             |
| `crates/anvil-tui/src/surfaces/tutorial/snapshots/`           | 10 files | Insta snapshot pins (path-select / running / complete at default / narrow / tiny + static-mode)                                            |
| `crates/anvil-cli/src/commands/tutorial.rs`                   | ~450     | CLI entry — progress file IO, watcher startup, watch-demo handoff, `--reset`                                                               |

## Related docs

- [`tui-as-built.md`](./tui-as-built.md) — surface-level treatment of the
  tutorial inside the wider TUI; the "Tutorial surface (deep dive)" section is
  the bridge between this doc and the rest of the TUI.
- [`activation-as-built.md`](./activation-as-built.md) — the `ProtectionState`
  enum and the five user-actionable state literals the ProtectionLoop copy is
  pinned to.
- [`checks-as-built.md`](./checks-as-built.md) — the scan pipeline that produces
  the `ScanResults` the discovery surface consumes.
- [`plans/archive/modules/launch-flow-readiness.aps.md`](../../plans/archive/modules/launch-flow-readiness.aps.md)
  — LAUNCH-014 (tutorial reframe to ProtectionLoop default — Complete) and the
  test-pinned copy-invariant decisions.
- `RELEASE-PLAN.md` — v0.6.0-beta slate framing for LAUNCH-014.
