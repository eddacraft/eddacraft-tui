# Welcome Screen & Interactive Onboarding

Module: **WELCOME**
Status: In Progress
Owner: eddacraft

## Problem

The Ratatui port (PORT-010, PORT-040–044) shipped a welcome screen and tutorial
that lost the "wow moment" from the original Ink version. Three specific gaps:

1. **No project discovery.** The old Ink flow scanned the user's actual project,
   surfaced real findings, and guided them through a fix. The new version is a
   static menu leading to a generic tutorial with no project interaction. In
   development the scan only ever hits `__fixtures__/` test files (intentional
   anti-patterns), which isn't useful for demos or real users.

2. **No first-run onboarding.** There is no distinction between a user's first
   `anvil start` and their hundredth. The welcome menu assumes familiarity — it
   lists commands, not outcomes. A new user needs guided setup (init config,
   install hooks, see Anvil work on their code) before they're ready for the
   command menu.

3. **Passive tutorials.** The four feature tutorials (Policy, Architecture,
   Drift, CI) are "press enter to continue" text. Instructions say "Run:
   `anvil gate`" but the user must switch to a terminal, type the command, and
   mentally correlate the output. The fix step asks them to open their editor
   and find the file. Every context switch is friction that kills adoption.

## Design Decisions

### DD-1: Scan filtering

Exclude paths matching these patterns from tutorial scan results:
- `__fixtures__/`, `__mocks__/`, `__tests__/`, `test-data/`, `fixtures/`
- `*.test.ts`, `*.spec.ts`, `*.test.rs`, `*_test.rs`
- `node_modules/`, `target/`, `.git/`

The scan runs two passes: `run_embedded()` for architecture/policy violations,
and `anvil-checks` scanners (antipattern + secret) for code-level findings. Both
honour the filter. Results are merged into a unified `ScanResults` struct sorted
by severity.

If after filtering the scan finds zero real warnings, enter **showcase mode**:
show a curated set of example findings with clear "[Example]" labels so the user
still sees what Anvil catches, but knows these aren't from their project.

### DD-2: Fix experience — dual-mode editing

When the fix step presents a warning, offer two paths:

1. **External editor (default):** Show the full file path (clickable via OSC 8
   hyperlinks in supporting terminals) and start a single-file watcher on the
   target path. When the file changes on disk, re-run the relevant check. If the
   warning resolves, show success and advance. If it persists, show updated
   context. Timeout after 60 seconds with a "skip" option.

2. **Inline editor (opt-in, press 'e'):** Open the file in a minimal embedded
   editor panel within the TUI (read-only context + editable region around the
   warning line). Uses a new `EditorState` widget extending the existing
   `TextInputState` to multi-line editing. User can make the fix without leaving
   the TUI. On save (Ctrl-S), re-run the check.

The inline editor is valuable beyond the tutorial (future use in watch mode,
gate review, etc.), so it's built as a reusable `eddacraft-tui` widget.

### DD-3: Executable tutorial steps

Every tutorial instruction that contains an executable command (prefixed with
"Run:" or "Create:") gains an **Execute** action bound to Enter. Pressing Enter
runs the command via subprocess, captures output, displays it inline, and
triggers verification. Steps without executable instructions (informational
text) advance on Enter as before.

After execution, a verification check confirms the expected outcome (file
exists, exit code zero, specific output pattern). Results show a green tick or
red cross. On failure: retry, show hint, or skip.

For steps that ask the user to edit a file, a file watcher detects changes and
re-runs verification automatically — no need to press Enter.

This replaces the earlier "In Your Project" approach from DD-3 (v1). Instead of
adding one special scan step per tutorial path, every instruction step becomes
interactive. The user experiences Anvil's actual commands, not descriptions of
them.

### DD-4: First-run detection

Detect first run via absence of `.anvil/first-run` marker file. When
`anvil start` is invoked and no marker exists, show the onboarding flow
(WELCOME-001 → 003) instead of the standard welcome menu. After onboarding
completes (or is skipped), create the marker.

Respect `ANVIL_SKIP_WELCOME=1` env var to bypass entirely (CI environments,
scripted usage).

### DD-5: Graceful degradation

When the kernel watcher is unavailable (inotify limit reached, no project
directory, running in CI), fall back to the existing static tutorial content.
Detection is automatic — try to start the watcher, if it fails, switch to
static mode with a notice: "Interactive mode unavailable — showing guided
walkthrough."

The discovery scan (Phase 2) similarly degrades: if `run_embedded()` fails, skip
to showcase mode rather than erroring out.

## Tasks

#### Phase 1 — First-Run Onboarding

### WELCOME-001: First-run detection and auto-launch

- **Status:** Done
- **Intent:** Detect first run via absence of `.anvil/first-run` marker.
  When `anvil start` is invoked and no marker exists, show the onboarding
  flow instead of the standard welcome menu. After onboarding completes
  (or is skipped), create the marker. Respect `ANVIL_SKIP_WELCOME=1` env
  var to bypass entirely.
- **Validation:** Delete `.anvil/first-run` → `anvil start` shows
  onboarding; create marker → shows welcome menu; set env var → skips.
- **Files:** `crates/anvil-cli/src/commands/welcome.rs`,
  `crates/anvil-cli/src/services/first_run.rs`

### WELCOME-002: Onboarding welcome screen

- **Status:** Done
- **Intent:** Show a first-run specific welcome screen with: brand logo,
  value proposition ("Anvil catches architecture drift at save-time"),
  and three options: "Start guided setup", "Skip to tutorial", "Skip
  entirely". Distinct from the standard welcome menu — focused on
  getting the user productive, not listing all commands.
- **Validation:** Visual match against design system; three options
  navigable via j/k/Enter; "Skip entirely" creates marker and exits to
  standard welcome.
- **Files:** `crates/anvil-tui/src/surfaces/onboarding/welcome.rs`,
  `crates/anvil-tui/src/surfaces/onboarding/welcome_render.rs`
- **Dependencies:** WELCOME-001

### WELCOME-003: Guided init step

- **Status:** In Progress
- **Intent:** Wire the existing `InitSurface` (5-step wizard: mode,
  format, directory, checks, summary) into the onboarding flow. After
  init completes, transition to the discovery scan (WELCOME-007). If
  `.anvil.yaml` already exists, skip with a "you're already configured"
  message and proceed directly to discovery.
- **Validation:** Generated config is valid; `anvil doctor` passes after
  init; existing config detected and skipped.
- **Files:** `crates/anvil-tui/src/surfaces/onboarding/mod.rs`
- **Dependencies:** WELCOME-002
- **Note:** Config persistence was never implemented — `TODO` stub at
  `welcome.rs:156`. Reopened 2026-04-12.

#### Phase 2 — Discovery Scan Infrastructure

### WELCOME-004: Scan filter for test fixtures

- **Status:** Done
- **Intent:** Create `ScanFilter` in `anvil-checks` that excludes test
  fixture paths from results per DD-1 patterns. Accepts a list of glob
  patterns, returns a predicate function `fn(&Path) -> bool`. Add unit
  tests for each exclusion pattern and edge cases (nested fixtures,
  partial matches).
- **Files:** `crates/anvil-checks/src/filter.rs`

### WELCOME-005: Discovery surface with scan progress

- **Status:** Done
- **Intent:** Create `DiscoverySurface` in
  `crates/anvil-tui/src/surfaces/tutorial/discovery.rs`. Three phases:
  **Scanning** (spinner + file count from kernel `Progress` events) →
  **Results** (warning list with file:line, severity, message, suggestion
  — top 5 findings sorted by severity) → **Continue** (proceed to fix or
  tutorial). Runs `run_embedded_cancellable()` for architecture/policy
  violations and `anvil-checks` scanners for antipattern/secret findings.
  Both filtered via `ScanFilter`. Results merged into a `ScanResults`
  struct.
- **Validation:** Scan completes within 10s on a 500-file project;
  progress updates render during scan; results display matches warning
  format; 's' skips during scan.
- **Files:** `crates/anvil-tui/src/surfaces/tutorial/discovery.rs`,
  `crates/anvil-tui/src/surfaces/tutorial/discovery_render.rs`
- **Dependencies:** WELCOME-004

### WELCOME-006: Showcase mode fallback

- **Status:** Done
- **Intent:** When filtered scan returns zero warnings, display 3–4
  curated example findings covering different check types (secret
  detection, anti-pattern, architecture violation, policy violation).
  Each clearly labelled "[Example]" with muted styling. Examples derived
  from `__fixtures__/` at build time so they stay current with check
  capabilities.
- **Files:** `crates/anvil-tui/src/surfaces/tutorial/showcase.rs`
- **Dependencies:** WELCOME-005

### WELCOME-007: Wire discovery into welcome and onboarding flows

- **Status:** In Progress
- **Intent:** Two integration points: (a) After "Interactive Tutorial"
  from the standard welcome menu, run discovery scan before entering
  tutorial path selection. (b) After guided init in onboarding flow,
  run discovery scan before presenting tutorial. Pass `ScanResults`
  through to tutorial state in both cases.
- **Files:** `crates/anvil-tui/src/surfaces/welcome/mod.rs`,
  `crates/anvil-tui/src/surfaces/onboarding/mod.rs`,
  `crates/anvil-tui/src/surfaces/tutorial/mod.rs`
- **Dependencies:** WELCOME-003, WELCOME-005

### WELCOME-008: Scan results threading through phases

- **Status:** Done
- **Intent:** Extend `TutorialState` to carry `Option<ScanResults>`.
  Results from discovery flow into feature tutorials so executable
  steps can reference already-found issues rather than re-scanning.
  Tutorial paths receive findings relevant to their domain (policy
  tutorial gets policy violations, architecture tutorial gets boundary
  violations, etc.) via a `ScanResults::filter_by_domain()` method.
- **Files:** `crates/anvil-tui/src/surfaces/tutorial/mod.rs`,
  `crates/anvil-tui/src/surfaces/tutorial/discovery.rs`
- **Dependencies:** WELCOME-005

#### Phase 3 — Fix Experience

### WELCOME-009: Inline editor widget

- **Status:** Done
- **Intent:** Create `EditorState` / `EditorWidget` in
  `crates/eddacraft-tui/src/widgets/editor.rs`. Multi-line text editing
  with: line numbers, current-line highlighting, scroll, cursor movement
  (hjkl/arrows, Home/End, PgUp/PgDn), insert/delete/backspace. Load
  from file path, save back to file on Ctrl-S. Read-only context lines
  (above/below editable region) rendered in muted style.
- **Validation:** Load a file, navigate, edit, save — diff matches
  expected changes; read-only lines reject input; scroll handles files
  larger than viewport.
- **Files:** `crates/eddacraft-tui/src/widgets/editor.rs`,
  `crates/eddacraft-tui/src/widgets/mod.rs`

### WELCOME-010: Fix step with dual-mode editing

- **Status:** Done
- **Intent:** Present the top warning with 5 lines of context
  above/below. Default: show file path (OSC 8 hyperlink) + start
  single-file watcher. When file changes on disk, re-run the relevant
  check. If resolved, show success animation and advance. If persists,
  show updated context. Press 'e' to open inline editor (WELCOME-009)
  focused on the warning line. On save, re-run check. Timeout after 60s
  with skip option. Press 's' to skip immediately.
- **Validation:** External edit triggers re-check within 1s; fixed
  warning shows success; inline editor opens on 'e'; save triggers
  re-check; timeout and skip work.
- **Files:** `crates/anvil-tui/src/surfaces/tutorial/fix.rs`,
  `crates/anvil-tui/src/surfaces/tutorial/fix_render.rs`
- **Dependencies:** WELCOME-005, WELCOME-009

#### Phase 4 — Executable Tutorial Steps

### WELCOME-011: Executable instruction steps

- **Status:** Done
- **Intent:** Extend `TutorialStep` with an optional `command: String`
  field. When a step has a command, bind Enter to execute it via
  subprocess with output capture. Show command output inline below the
  step description. If the command succeeds (exit 0), mark the step
  complete and advance. If it fails, show the error with retry/skip
  options. Steps without commands (informational text) behave as before
  — Enter advances. Backfill commands for existing tutorial steps where
  the instruction already says "Run: ..." or "Create: ...".
- **Validation:** Each executable step runs its command; output captured
  and displayed; failure shows error with retry (r) / skip (s) options;
  non-executable steps still advance on Enter.
- **Files:** `crates/anvil-tui/src/surfaces/tutorial/mod.rs`,
  `crates/anvil-tui/src/surfaces/tutorial/paths.rs`,
  `crates/anvil-tui/src/surfaces/tutorial/executor.rs`

### WELCOME-012: Step verification and feedback

- **Status:** Done
- **Intent:** After executing a step's command, run a verification check
  to confirm the expected outcome. Verification types: file exists, exit
  code check, content match (regex against file or output). Each step
  declares its verification in a `verify: Option<Verify>` field. Show a
  green tick or red cross next to the step. On failure: offer retry,
  show contextual hint, or skip. Steps without verification auto-pass
  on successful execution.
- **Validation:** Correct action → green tick; incorrect → red cross with
  hint; skip → step marked skipped (distinct from completed).
- **Files:** `crates/anvil-tui/src/surfaces/tutorial/verify.rs`,
  `crates/anvil-tui/src/surfaces/tutorial/paths.rs`
- **Dependencies:** WELCOME-011

### WELCOME-013: Live file watching during tutorial steps

- **Status:** In Progress
- **Intent:** For steps that ask the user to edit a file (e.g. "Add a
  policy rule to no-todos.yaml"), start a file watcher on the target
  path. When the file changes, re-run verification automatically. This
  gives immediate feedback as the user edits — no need to press Enter
  to trigger verification. Step advances automatically when verification
  passes. Combine with WELCOME-012 for a smooth edit → verify → advance
  cycle. Requires kernel watcher — degrades to manual Enter when
  unavailable.
- **Validation:** File change detected within 1s; verification runs
  automatically; step advances on success; graceful fallback without
  watcher.
- **Files:** `crates/anvil-tui/src/surfaces/tutorial/executor.rs`
- **Dependencies:** WELCOME-012, KERN (watcher)

#### Phase 5 — Watch Demo & Hooks

### WELCOME-014: Watch mode demo with guided overlay

- **Status:** In Progress
- **Intent:** Start `anvil watch` in a tutorial context — render the
  real watch dashboard with a semi-transparent guided overlay explaining
  each panel (file watcher status, check results, warning list). The
  user can make file changes and see the dashboard update in real time.
  After 30 seconds or one full file-change → check → result cycle,
  offer to continue to next steps. Progressive hints at 10s/20s/30s.
  Press 's' to skip immediately.
- **Validation:** Watch dashboard renders with overlay annotations; file
  change triggers visible update; "continue" option works; skip works.
- **Files:** `crates/anvil-tui/src/surfaces/tutorial/watch_demo.rs`,
  `crates/anvil-tui/src/surfaces/tutorial/watch_demo_render.rs`
- **Dependencies:** WELCOME-010, KERN (watcher)

### WELCOME-015: Git hooks installation step

- **Status:** Done
- **Intent:** Offer to install git hooks (pre-commit, pre-push) that
  run Anvil checks. Show what each hook does before installing. Detect
  existing hook managers (Husky, lefthook, pre-commit framework) and
  adapt — add Anvil to the existing setup rather than overwriting.
  Confirmation required before any file modification. Decline skips
  without error.
- **Validation:** Hooks installed correctly; existing Husky setup
  detected and adapted; decline skips cleanly; hooks fire on next
  commit.
- **Files:** `crates/anvil-tui/src/surfaces/onboarding/hooks.rs`
- **Dependencies:** WELCOME-003

#### Phase 6 — Completion, Persistence & Resilience

### WELCOME-016: Tutorial progress persistence and resumption

- **Status:** Done
- **Intent:** Persist tutorial progress at `~/.anvil/tutorial-progress.json`
  so interrupted sessions can resume. Track: which steps completed, scan
  results (to avoid re-scanning), chosen configuration, onboarding
  completion state. On resume, skip completed steps and show "Resuming
  from step N". Completed tutorial offers "redo" option on next launch.
- **Validation:** Kill during step 3 → restart → jumps to step 3;
  completed tutorial → restart offers "redo"; scan results cached and
  reused on resume.
- **Files:** `crates/anvil-tui/src/surfaces/tutorial/mod.rs`,
  `crates/anvil-cli/src/commands/tutorial.rs`

### WELCOME-017: Next steps and completion summary

- **Status:** Done
- **Intent:** Show a summary of what was set up, what Anvil found, and
  what to do next. Content: findings count, config created (y/n), hooks
  installed (y/n). Suggestions: "Run `anvil watch` to monitor
  continuously", "Run `anvil gate` before pushing", "See `anvil --help`
  for all commands". Mark tutorial complete in progress file. Offer to
  return to the welcome menu.
- **Validation:** Summary shows correct counts; progress file updated;
  return-to-welcome works; suggestions render correctly.
- **Files:** `crates/anvil-tui/src/surfaces/onboarding/complete.rs`
- **Dependencies:** WELCOME-007

### WELCOME-018: Static tutorial fallback

- **Status:** Done
- **Intent:** When the kernel watcher is unavailable (e.g. inotify limit
  reached, no project directory, running in CI), fall back to the
  existing static tutorial content per DD-5. Detection is automatic —
  try to start watcher, if it fails, switch to static mode with a
  notice: "Interactive mode unavailable — showing guided walkthrough."
  Executable steps (WELCOME-011) also degrade: show the command text
  but disable auto-execution, reverting to "press enter to continue."
- **Validation:** Set inotify limit to 0 → tutorial launches in static
  mode; normal environment → interactive mode; notice displayed in
  static mode.
- **Files:** `crates/anvil-tui/src/surfaces/tutorial/mod.rs`
- **Dependencies:** WELCOME-011

## File Map

```text
# Phase 1 — First-Run Onboarding
crates/anvil-cli/src/commands/welcome.rs: WELCOME-001
crates/anvil-cli/src/services/first_run.rs: WELCOME-001
crates/anvil-tui/src/surfaces/onboarding/mod.rs: WELCOME-002, WELCOME-003, WELCOME-007
crates/anvil-tui/src/surfaces/onboarding/welcome.rs: WELCOME-002
crates/anvil-tui/src/surfaces/onboarding/welcome_render.rs: WELCOME-002

# Phase 2 — Discovery Scan Infrastructure
crates/anvil-checks/src/filter.rs: WELCOME-004
crates/anvil-tui/src/surfaces/tutorial/discovery.rs: WELCOME-005, WELCOME-006, WELCOME-008
crates/anvil-tui/src/surfaces/tutorial/discovery_render.rs: WELCOME-005, WELCOME-006
crates/anvil-tui/src/surfaces/tutorial/showcase.rs: WELCOME-006
crates/anvil-tui/src/surfaces/welcome/mod.rs: WELCOME-007
crates/anvil-tui/src/surfaces/tutorial/mod.rs: WELCOME-007, WELCOME-008, WELCOME-011, WELCOME-013, WELCOME-016, WELCOME-018

# Phase 3 — Fix Experience
crates/eddacraft-tui/src/widgets/editor.rs: WELCOME-009
crates/eddacraft-tui/src/widgets/mod.rs: WELCOME-009
crates/anvil-tui/src/surfaces/tutorial/fix.rs: WELCOME-010
crates/anvil-tui/src/surfaces/tutorial/fix_render.rs: WELCOME-010

# Phase 4 — Executable Tutorial Steps
crates/anvil-tui/src/surfaces/tutorial/paths.rs: WELCOME-011, WELCOME-012
crates/anvil-tui/src/surfaces/tutorial/executor.rs: WELCOME-011, WELCOME-013
crates/anvil-tui/src/surfaces/tutorial/verify.rs: WELCOME-012

# Phase 5 — Watch Demo & Hooks
crates/anvil-tui/src/surfaces/tutorial/watch_demo.rs: WELCOME-014
crates/anvil-tui/src/surfaces/tutorial/watch_demo_render.rs: WELCOME-014
crates/anvil-tui/src/surfaces/onboarding/hooks.rs: WELCOME-015

# Phase 6 — Completion, Persistence & Resilience
crates/anvil-cli/src/commands/tutorial.rs: WELCOME-016
crates/anvil-tui/src/surfaces/onboarding/complete.rs: WELCOME-017
```

## Dependencies

- `run_embedded()` / `run_embedded_cancellable()` from `anvil-kernel` for
  project scanning (returns `EmbeddedResult` with `Progress`/`Violation` events)
- `scan_file()` / `scan_content()` from `anvil-checks` for antipattern and
  secret detection
- `TextInputState` from `eddacraft-tui` as base for editor widget
- `WatchEventAdapter` from `anvil-tui` watch surface for watch demo integration
- `InitSurface` from `anvil-tui` init surface for guided init step
- Existing `EddaCraftTheme` for consistent styling
- KERN — kernel watcher for fix detection (WELCOME-010), file watching
  (WELCOME-013), and watch demo (WELCOME-014)

## Risks

| Risk | Likelihood | Impact | Mitigation |
| ---- | ---------- | ------ | ---------- |
| Scan finds zero warnings (clean project) | Medium | Medium | Showcase mode (WELCOME-006) with curated examples |
| Scan performance on large projects | Medium | Medium | `run_embedded_cancellable()` with 10s timeout; cap to 500 files; show progress; allow skip |
| File watcher conflicts with user's IDE | Low | Low | Non-recursive single-file watch for fix detection |
| Inline editor complexity | Medium | Low | Phase 3 ships minimal version (no syntax highlighting); iterate later. Editor is opt-in, not blocking |
| Kernel unavailable during tutorial | Low | Medium | Static fallback (WELCOME-018) degrades gracefully |
| Tutorial takes too long | Medium | Medium | Skip on every step; timeout on interactive steps; <5 min target for experienced devs |
| Showcase mode staleness | Low | Low | Derive examples from `__fixtures__/` at build time |
| Subprocess execution in TUI | Medium | Medium | Sandbox commands to read-only operations where possible; show command before executing; never run destructive commands without confirmation |
| Existing hook manager conflicts | Low | Medium | Detect Husky/lefthook/pre-commit and adapt rather than overwrite |

## Sequencing

Phases 1 and 4 are independent and can be developed in parallel:
- Phase 1 is visual/UI work (first-run detection, onboarding screens)
- Phase 4 is tutorial step enhancement (executor, verification)

Phase 2 depends on `anvil-kernel` and `anvil-checks` (both exist).
Phase 3 depends on Phase 2 results for the fix step.
Phase 5 depends on KERN watcher for watch demo and file detection.
Phase 6 wraps all previous phases.

Recommended parallel tracks:

```text
Track A: Phase 1 → Phase 5 (hooks) → Phase 6 (completion)
Track B: Phase 2 → Phase 3 → Phase 5 (watch demo) → Phase 6 (persistence)
Track C: Phase 4 (independent, merge into tutorial surface when ready)
```

## Supersedes

This module absorbs all work items from **TUTOR** (interactive-tutorial).
The TUTOR module is retired. Mapping:

| TUTOR Item | → WELCOME Item | Notes |
| ---------- | -------------- | ----- |
| TUTOR-001 | WELCOME-001 | First-run detection |
| TUTOR-002 | WELCOME-002 | Onboarding welcome |
| TUTOR-003 | WELCOME-003 | Guided init (reuses existing init surface) |
| TUTOR-004 | WELCOME-005 | Live scan (merged with discovery surface) |
| TUTOR-005 | WELCOME-010 | Fix detection (merged with dual-mode fix) |
| TUTOR-006 | WELCOME-014 | Watch demo (merged with guided overlay) |
| TUTOR-007 | WELCOME-015 | Hooks installation |
| TUTOR-008 | WELCOME-017 | Next steps / completion |
| TUTOR-009 | WELCOME-016 | Progress persistence |
| TUTOR-010 | WELCOME-018 | Static fallback |
| TUTOR-011 | WELCOME-011 | Executable steps |
| TUTOR-012 | WELCOME-012 | Step verification |
| TUTOR-013 | WELCOME-013 | File watching during steps |

## Stats

| Phase | Items | Status |
| ----- | ----- | ------ |
| 1 — First-Run Onboarding | 3 | Proposed |
| 2 — Discovery Scan Infrastructure | 5 | Proposed |
| 3 — Fix Experience | 2 | Proposed |
| 4 — Executable Tutorial Steps | 3 | Proposed |
| 5 — Watch Demo & Hooks | 2 | Proposed |
| 6 — Completion, Persistence & Resilience | 3 | Proposed |
| **Total** | **18** | — |
