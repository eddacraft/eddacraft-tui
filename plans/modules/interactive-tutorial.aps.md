<!--
APS Module: Interactive Tutorial
=========================
Replace static tutorial content with live project interaction.
Scan, fix, watch — the tutorial runs real commands against the user's
codebase and responds to their actions.

Scopes: TUTOR (main)
-->

# Interactive Tutorial

| ID    | Owner | Status |
| ----- | ----- | ------ |
| TUTOR | —     | Draft  |

## Purpose

Replace the static "press enter to continue" tutorial with an interactive
onboarding experience that scans the user's real project, shows real
warnings, watches for real fixes, and demonstrates Anvil's value against
their own codebase.

**Why:** The current Rust tutorial has 4 paths (Policy, Architecture, Drift,
CI) with pre-written text and no project interaction. The Ink CLI had live
scanning, file watching, and fix detection — the user saw Anvil work on
their actual code. This is the difference between reading documentation and
experiencing the product. First impressions set adoption trajectory.

**Ink CLI baseline:** `apps/anvil-cli/src/tui/commands/tutorial/steps/`
provided ScanStep (live project scan), FixStep (watch for file changes +
detect fix), WatchStep (live watch demo), NextStepsStep (guided follow-up).

## In Scope

- First-run detection and onboarding flow
- Live project scanning during tutorial
- Real warning display from scan results
- File-watching for fix detection (user fixes a warning, tutorial responds)
- Watch mode demo (live file change → gate result cycle)
- Guided setup steps (init config, install hooks)
- Tutorial progress persistence across sessions
- Graceful degradation when project has no warnings

## Out of Scope

- Tutorial content translation / i18n
- Video or animation within the TUI
- Cloud-connected tutorial analytics
- Tutorial customisation per project type

## Interfaces

**Depends on:**

- RCLI — Rust CLI foundation (TUI runner, surface trait)
- KERN — kernel watcher and event emission (file watching for fix detection)
- RATS — Ratatui TUI surfaces (rendering framework)

**Exposes:**

- Enhanced `anvil tutorial` command with interactive mode
- First-run hook that auto-launches tutorial on `anvil start` when
  `.anvil/first-run` marker is absent
- `crates/anvil-tui/src/surfaces/tutorial/` — enhanced tutorial surface

## Constraints

- Tutorial must complete in <5 minutes for an experienced developer
- Must work on a project with zero Anvil config (no `.anvil.yaml` required)
- Must not modify the user's files without explicit confirmation
- Must degrade gracefully if kernel/watcher is unavailable (fall back to
  static content)
- Scan results must be real — no simulated warnings on real projects
- Tutorial state persists at `~/.anvil/tutorial-progress.json`

## Ready Checklist

Change status to **Ready** when:

- [ ] RCLI Phase 7 complete (gate, watch, auth working)
- [ ] KERN watcher available from CLI (file change events accessible)
- [ ] Current static tutorial surface stable (RCLI-025 through RCLI-030)
- [ ] Design spec approved

---

## Phase 1 — First-Run Onboarding

### TUTOR-001: first-run detection and auto-launch

- **Status:** Draft
- **Intent:** Detect first run via absence of `.anvil/first-run` marker.
  When `anvil start` is invoked and no marker exists, show the onboarding
  flow instead of the standard welcome menu. After onboarding completes
  (or is skipped), create the marker. Respect `ANVIL_SKIP_WELCOME=1` env
  var to bypass entirely
- **Expected Outcome:** First `anvil start` on a new project launches
  onboarding; subsequent runs show the normal welcome menu
- **Validation:** Delete `.anvil/first-run` → `anvil start` shows
  onboarding; create marker → shows welcome menu; set env var → skips
- **Files:** `crates/anvil-cli/src/commands/welcome.rs`,
  `crates/anvil-cli/src/services/first_run.rs`
- **Confidence:** high
- **Priority:** High
- **Dependencies:** None

---

### TUTOR-002: onboarding welcome screen

- **Status:** Draft
- **Intent:** Show a first-run specific welcome screen with: brand logo,
  value proposition ("Anvil catches architecture drift at save-time"),
  and three options: "Start guided setup", "Skip to tutorial", "Skip
  entirely". Different from the standard welcome menu — focused on
  getting the user productive, not listing all commands
- **Expected Outcome:** First-run welcome shows focused onboarding
  options, not the full command menu
- **Validation:** Visual match against design system; three options
  navigable; "Skip entirely" creates marker and exits
- **Files:** `crates/anvil-tui/src/surfaces/onboarding/welcome.rs`
- **Confidence:** high
- **Priority:** High
- **Dependencies:** TUTOR-001

---

### TUTOR-003: guided init step

- **Status:** Draft
- **Intent:** Walk the user through `anvil init` interactively: detect
  project type (Node/Rust/mixed), suggest a configuration profile,
  create `.anvil.yaml` with sensible defaults. Show what each setting
  does as it's configured. If `.anvil.yaml` already exists, skip with
  a "you're already configured" message
- **Expected Outcome:** User ends up with a working `.anvil.yaml` tailored
  to their project
- **Validation:** Generated config is valid; `anvil doctor` passes after
  init; project type correctly detected
- **Files:** `crates/anvil-tui/src/surfaces/onboarding/init.rs`
- **Confidence:** medium (reuses init command logic)
- **Priority:** Medium
- **Dependencies:** TUTOR-002

---

## Phase 2 — Live Project Scanning

### TUTOR-004: live project scan step

- **Status:** Draft
- **Intent:** Run a real audit/scan of the user's project during the
  tutorial. Show a progress indicator while scanning, then display the
  top 3-5 warnings found. If no warnings found, congratulate the user
  and show what Anvil would catch in typical projects. Uses the same
  scan logic as `anvil audit` but presents results in the tutorial
  context with explanations
- **Expected Outcome:** Tutorial shows real warnings from the user's
  project with line numbers and explanations
- **Validation:** Warnings match `anvil audit` output; progress indicator
  shows during scan; zero-warning case handled gracefully
- **Files:** `crates/anvil-tui/src/surfaces/onboarding/scan.rs`
- **Confidence:** medium (needs to reuse audit/gate infrastructure)
- **Priority:** High
- **Dependencies:** TUTOR-003

---

### TUTOR-005: fix detection step

- **Status:** Draft
- **Intent:** After showing a warning, prompt the user to fix it in their
  editor. Start a file watcher on the warning's file path. When the file
  changes, re-run the relevant check. If the warning is resolved, show a
  success animation and advance. If the warning persists, show updated
  context. Timeout after 60 seconds with a "skip this step" option.
  Must not block if the user doesn't want to fix anything
- **Expected Outcome:** User edits a file → tutorial detects the change
  → re-checks → shows "Fixed!" or "Still present"
- **Validation:** File change triggers re-check within 1 second; fixed
  warning shows success; unfixed shows updated context; timeout works
- **Files:** `crates/anvil-tui/src/surfaces/onboarding/fix.rs`
- **Confidence:** medium (needs kernel watcher integration in tutorial
  surface — currently tutorials are passive)
- **Priority:** High
- **Dependencies:** TUTOR-004, KERN (watcher)

---

## Phase 3 — Watch Mode Demo

### TUTOR-006: watch mode demo step

- **Status:** Draft
- **Intent:** Start `anvil watch` in a tutorial context — show the watch
  dashboard with a guided overlay explaining each panel. The user can
  make file changes and see the dashboard update in real time. After
  30 seconds or one file change cycle, offer to continue to next steps
- **Expected Outcome:** User sees the watch dashboard responding to their
  real file changes with tutorial annotations
- **Validation:** Watch dashboard renders with overlay; file change
  triggers visible update; "continue" option works
- **Files:** `crates/anvil-tui/src/surfaces/onboarding/watch_demo.rs`
- **Confidence:** low (needs overlay rendering on top of existing watch
  surface — may need a composite surface pattern)
- **Priority:** Medium
- **Dependencies:** TUTOR-005, KERN (watcher)

---

### TUTOR-007: hooks installation step

- **Status:** Draft
- **Intent:** Offer to install git hooks (pre-commit, pre-push) that
  run Anvil checks. Show what each hook does before installing. Respect
  existing hooks (Husky, lefthook) — detect and adapt. Confirmation
  required before any file modification
- **Expected Outcome:** "Install hooks?" → user confirms → hooks
  installed → verification shown
- **Validation:** Hooks installed correctly; existing Husky setup
  detected and adapted; decline skips without error
- **Files:** `crates/anvil-tui/src/surfaces/onboarding/hooks.rs`
- **Confidence:** high (reuses hooks command logic)
- **Priority:** Medium
- **Dependencies:** TUTOR-003

---

## Phase 4 — Completion and Persistence

### TUTOR-008: next steps and completion

- **Status:** Draft
- **Intent:** Show a summary of what was set up, what Anvil found, and
  what to do next. Suggest: "Run `anvil watch` to monitor continuously",
  "Run `anvil gate` before pushing", "See `anvil --help` for all
  commands". Mark tutorial as complete in progress file. Offer to
  return to the welcome menu
- **Expected Outcome:** Tutorial ends with actionable next steps and
  a sense of accomplishment
- **Validation:** Summary shows correct counts; progress file updated;
  return-to-welcome works
- **Files:** `crates/anvil-tui/src/surfaces/onboarding/complete.rs`
- **Confidence:** high
- **Priority:** Medium
- **Dependencies:** TUTOR-004

---

### TUTOR-009: tutorial progress and resumption

- **Status:** Draft
- **Intent:** Persist tutorial progress so interrupted sessions can
  resume. Track: which steps completed, scan results (so re-scan isn't
  needed), chosen configuration. Store at `~/.anvil/tutorial-progress.json`.
  On resume, skip completed steps and show "Resuming from step N"
- **Expected Outcome:** Close terminal mid-tutorial → re-run → resumes
  where you left off
- **Validation:** Kill during step 3 → restart → jumps to step 3;
  completed tutorial → restart offers "redo" option
- **Files:** `crates/anvil-cli/src/commands/tutorial.rs` (progress
  loading/saving already exists — extend for interactive steps)
- **Confidence:** high
- **Priority:** Medium
- **Dependencies:** TUTOR-001

---

### TUTOR-010: static tutorial fallback

- **Status:** Draft
- **Intent:** When the kernel watcher is unavailable (e.g. inotify limit
  reached, no project directory, running in CI), fall back to the
  existing static tutorial content. Detection should be automatic —
  try to start watcher, if it fails, switch to static mode with a
  notice: "Interactive mode unavailable — showing guided walkthrough"
- **Expected Outcome:** Tutorial works everywhere, just with reduced
  interactivity on constrained environments
- **Validation:** Set inotify limit to 0 → tutorial launches in static
  mode; normal environment → interactive mode
- **Files:** `crates/anvil-tui/src/surfaces/tutorial/mod.rs`
- **Confidence:** high
- **Priority:** Low
- **Dependencies:** TUTOR-004

---

## Phase 5 — Executable Tutorial Steps

Upgrade the existing 4-path tutorial (Policy, Architecture, Drift, CI)
so instruction steps are actionable — press Enter to execute the suggested
command, and the tutorial verifies the result before advancing.

### TUTOR-011: executable instruction steps

- **Status:** Draft
- **Intent:** When a tutorial step has an instruction like "Run: mkdir -p
  .anvil/policies" or "Create .anvil/policies/no-todos.yaml", add an
  "Execute" action (Enter key) that runs the command or creates the file.
  Show the command output inline. If the command succeeds, mark the step
  complete and advance. If it fails, show the error and let the user
  retry or skip. Steps without executable instructions (informational
  text) behave as before — Enter advances
- **Expected Outcome:** Tutorial step says "Run: anvil gate" → user
  presses Enter → command runs → output shown → step advances on success
- **Validation:** Each executable step runs its command; output is
  captured and displayed; failure shows error with retry/skip options
- **Files:** `crates/anvil-tui/src/surfaces/tutorial/mod.rs`,
  `crates/anvil-tui/src/surfaces/tutorial/paths.rs`,
  `crates/anvil-tui/src/surfaces/tutorial/executor.rs`
- **Confidence:** medium (needs TUI ↔ subprocess integration with
  output capture)
- **Priority:** High
- **Dependencies:** None (works against existing tutorial surface)

---

### TUTOR-012: step verification after execution

- **Status:** Draft
- **Intent:** After executing a step's command, verify the expected
  outcome. For example: after "Create .anvil/policies/no-todos.yaml",
  check the file exists. After "Run: anvil gate", check exit code.
  Show a green tick or red cross next to the step based on verification.
  If verification fails, offer: retry, show hint, or skip
- **Expected Outcome:** Each executable step has a verification check
  that confirms the action was performed correctly
- **Validation:** Create correct file → green tick; create wrong file →
  red cross with hint; skip → step marked skipped
- **Files:** `crates/anvil-tui/src/surfaces/tutorial/verify.rs`
- **Confidence:** medium
- **Priority:** Medium
- **Dependencies:** TUTOR-011

---

### TUTOR-013: live file watching during tutorial steps

- **Status:** Draft
- **Intent:** For steps that ask the user to edit a file (e.g. "Add a
  policy rule to no-todos.yaml"), start a file watcher on the target
  path. When the file changes, re-run verification. This gives immediate
  feedback as the user edits — no need to press Enter to trigger
  verification. Combine with TUTOR-012 for a smooth edit → verify →
  advance cycle
- **Expected Outcome:** User edits the target file in their editor →
  tutorial detects the change → re-verifies → advances automatically
  when verification passes
- **Validation:** File change detected within 1 second; verification
  runs automatically; step advances on success
- **Files:** `crates/anvil-tui/src/surfaces/tutorial/executor.rs`
- **Confidence:** low (needs watcher integration in tutorial surface)
- **Priority:** Low
- **Dependencies:** TUTOR-012, KERN (watcher)

---

## Risks

| Risk | Likelihood | Impact | Mitigation |
| ---- | ---------- | ------ | ---------- |
| Scan finds zero warnings (clean project) | Medium | Medium | Graceful zero-state with example of what Anvil catches |
| File watcher conflicts with user's IDE | Low | Low | Non-recursive single-file watch for fix detection |
| Tutorial takes too long | Medium | Medium | Skip option on every step; timeout on interactive steps |
| Kernel unavailable during tutorial | Low | Medium | TUTOR-010 static fallback |
| User's project too large to scan quickly | Medium | Medium | Timeout with partial results; sample subset |

## Sequencing Note

This module is independent of RCLI Phase 7 completion — it can be
developed in parallel. However, the fix detection (TUTOR-005) and watch
demo (TUTOR-006) require the kernel watcher, which must handle the
inotify limit fix (RCLI-014a).

Recommended order:

1. Phase 1 (first-run + onboarding screens) — visual work, no kernel
2. Phase 2 (scan + fix) — needs audit infrastructure + kernel watcher
3. Phase 3 (watch demo) — needs working watch mode
4. Phase 4 (completion + persistence) — can follow any phase

## Stats

| Phase | Items | Status |
| ----- | ----- | ------ |
| 1 — First-Run Onboarding | 3 | Draft |
| 2 — Live Project Scanning | 2 | Draft |
| 3 — Watch Mode Demo | 2 | Draft |
| 4 — Completion & Persistence | 3 | Draft |
| 5 — Executable Tutorial Steps | 3 | Draft |
| **Total** | **13** | — |
