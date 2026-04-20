<!--
APS Module: TUI Polish
=========================
Fix rough edges in the existing Ratatui TUI surfaces:
welcome screen, tutorial flow, progress indicators,
completion states, and text rendering.

Scopes: POLISH (main)
-->

# TUI Polish

| ID     | Owner | Status      | Progress |
| ------ | ----- | ----------- | -------- |
| POLISH | —     | In Progress | 7/8      |

## Purpose

Address UX rough edges identified during the April 2026 TUI review.
The core TUI surfaces (welcome, tutorial) are functional but have
polish issues that affect first impressions. These are small, targeted
fixes — not feature work.

**Why now:** The TUI is being used for demo screenshots and early access
onboarding. First impressions matter. These issues are quick to fix and
high-signal for users seeing Anvil for the first time.

## In Scope

- Welcome → tutorial navigation flow
- Tutorial progress indicator legibility
- Tutorial box sizing and text wrapping
- Tutorial completion / celebration screen
- Empty space in tutorial selection menu
- Welcome screen: verify all 6 options navigate correctly

## Out of Scope

- Interactive tutorial rewrite or new content (that shipped under WELCOME)
- TUI dashboard features (that's TUIDASH)
- Architecture or engine changes

## Interfaces

**Depends on:**
- RCLI — Rust CLI (surface trait, TUI runner)
- RATS — Ratatui surfaces (rendering framework, already complete)

**Touches:**
- `crates/anvil-tui/src/surfaces/tutorial/`
- `crates/anvil-tui/src/surfaces/welcome/`

## Tasks

### POLISH-001: Verify welcome → tutorial navigation end-to-end

- **Intent:** Confirm that selecting "Interactive tutorial" from the welcome
  screen correctly launches `anvil tutorial` and returns cleanly on exit
- **Expected Outcome:** Welcome → tutorial → completion → back to welcome
  works without errors or blank screens
- **Validation:** Manual walkthrough of the full flow; no panics, no blank
  states, clean exit on `q`
- **Files:** `crates/anvil-tui/src/surfaces/welcome/`
- **Confidence:** high
- **Priority:** High
- **Status:** Complete

---

### POLISH-002: Tutorial selection menu — fill empty space

- **Intent:** The tutorial selection menu shows 4 options then a large empty
  box. Either add descriptive copy below the list or reduce the box height to
  match content.
- **Expected Outcome:** No large blank region below the 4 tutorial options;
  space is either used meaningfully or removed
- **Validation:** Screenshot comparison — no visible dead space in the menu
- **Files:** `crates/anvil-tui/src/surfaces/tutorial/`
- **Confidence:** high
- **Priority:** Medium
- **Status:** Complete

---

### POLISH-003: Progress indicator — improve legibility

- **Intent:** Current progress indicator (`> - o - o - o - o - o`) is hard
  to read at a glance. Replace with filled/empty dot pattern or numbered
  steps that clearly communicate position within the tutorial.
- **Expected Outcome:** Progress position is immediately clear without
  needing to parse the indicator carefully. E.g. `● ○ ○ ○ ○ ○` or
  `Step 1 of 6`
- **Validation:** Screenshot shows progress at step 1, 3, and 6 — each
  clearly distinguishable
- **Files:** `crates/anvil-tui/src/surfaces/tutorial/render.rs`
- **Confidence:** high
- **Priority:** Medium
- **Status:** Complete

---

### POLISH-004: Tutorial text wrapping

- **Intent:** Tutorial content text wraps mid-word in some steps due to
  fixed box width. Fix wrapping logic to break at word boundaries.
- **Expected Outcome:** No mid-word breaks in any tutorial step content at
  standard terminal widths (80, 120, 200 cols)
- **Validation:** Run through all steps in Policy path at 80-col terminal —
  no mid-word wraps visible
- **Files:** `crates/anvil-tui/src/surfaces/tutorial/render.rs`
- **Confidence:** high
- **Priority:** Medium
- **Status:** Complete

---

### POLISH-005: Tutorial completion screen — improve celebration

- **Intent:** The "Well Done" completion screen is minimal — plain text in
  a box. For a new user completing their first tutorial, this should feel
  like a moment. Add visual emphasis, next-step prompts, or a brief summary
  of what they learned.
- **Expected Outcome:** Completion screen has clear visual emphasis (e.g.
  checkmark, colour accent) and at least one actionable next step beyond
  "choose another tutorial"
- **Validation:** Screenshot of completion screen looks meaningfully
  different from a mid-tutorial step
- **Files:** `crates/anvil-tui/src/surfaces/tutorial/`
- **Confidence:** medium
- **Priority:** Low
- **Status:** Complete

---

### POLISH-006: Welcome screen — verify all 6 options

- **Intent:** Confirm each option on the welcome screen navigates to the
  correct destination or launches the correct command. Particularly: "View
  documentation" should open docs.eddacraft.ai in the browser.
- **Expected Outcome:** All 6 welcome options work correctly with no dead
  ends or unimplemented stubs
- **Validation:** Manual test of each option; document any stubs for
  follow-up
- **Files:** `crates/anvil-tui/src/surfaces/welcome/`
- **Confidence:** medium
- **Priority:** High
- **Status:** Complete

### POLISH-007: Tutorial commands out of sync with actual CLI — CRITICAL

- **Intent:** Audit every command referenced in tutorial step instructions against
  the actual CLI. Replace stubs and auth-walled commands with commands that
  actually work for an unauthenticated early access user, or remove them.
- **Expected Outcome:** Every command shown in the tutorial either (a) runs and
  produces real output, or (b) is clearly framed as "you'll do this after setup"
  with no expectation of immediate execution. Zero "Authentication required"
  surprises during tutorial.
- **Validation:** Walk all 4 tutorial paths and run every referenced command.
  None should fail with auth errors or command-not-found.
- **Findings from April 2026 audit:**

  | Command in tutorial | Actual behaviour |
  |---|---|
  | `anvil gate` | ❌ Auth wall — `Authentication required` |
  | `anvil check` | ❌ Auth wall |
  | `anvil drift capture` | ❌ Auth wall (and wrong subcommand — it's `snapshot`) |
  | `anvil drift compare` | ❌ Auth wall |
  | `anvil architecture compile` | ❌ Command does not exist (it's `validate`) |
  | `anvil architecture validate` | ❌ Auth wall |
  | `anvil doctor` | ✅ Works unauthenticated |
  | `anvil tutorial` | ✅ Works unauthenticated |
  | `anvil welcome` | ✅ Works unauthenticated |
  | `anvil status` | Unverified |

  Additionally: tutorial copy contains internal stub language that should
  never be user-facing:
  - "once shipped" (Policy path, step 5)
  - "will be the hook command once shipped" (CI path, step 2)
  - "Gate step will be added once shipped" (CI path, step 3)

- **Files:**
  - `crates/anvil-tui/src/surfaces/tutorial/paths.rs` — fix command references and stub copy
  - `crates/anvil-cli/src/` — verify which commands need auth and which don't
- **Confidence:** high
- **Priority:** Critical — this is the highest priority issue in the module.
  A user who runs the tutorial and types the commands will hit auth errors
  on nearly every instruction. This is a broken first experience.
- **Status:** Complete

---

### POLISH-008: `anvil start` onboarding on existing codebases — config detection and missing landing screen

- **Intent:** Fix two related issues surfaced while walking `anvil start` on a
  codebase that has already been initialised, and surface some breathing room
  between guided init and the tutorial so the user knows what just happened.
- **Findings from April 2026 walkthrough:**

  1. **Config detection mismatch (fixed).** `onboarding::config_exists_in`
     checked for `.anvil.yaml | .anvil.json | .anvil.toml`, but
     `commands::init::generate_config` always writes `.anvilrc` regardless of
     selected format. Effect: guided init could not detect an existing Anvil
     config in an already-onboarded repo. Fix: include `.anvilrc` in the
     filename set (`crates/anvil-tui/src/surfaces/onboarding/mod.rs`) with a
     regression test covering `.anvilrc`.
  2. **No landing screen between init and tutorial (fixed).** `CompletionState`
     / `OnboardingSummary` existed in
     `crates/anvil-tui/src/surfaces/onboarding/complete.rs` but were only
     exercised by their own unit tests — `run_onboarding` / `run_guided_init`
     in `crates/anvil-cli/src/commands/welcome.rs` dropped straight from init →
     `run_discovery` → `run_tutorial_with_fix` with no explanation of what was
     written to disk or what the tutorial is about. Fix: added focused
     `InitCompleteState` / `InitCompleteSummary` in
     `crates/anvil-tui/src/surfaces/onboarding/init_complete.rs`, wired into
     `run_guided_init` after `generate_config` succeeds. Copy follows the
     user-approved minimal receipt-style pattern (what was written, what
     happens next, takes ~5 min).
  3. **Onboarding surfaces don't share the tutorial's new outer padding
     (fixed).** The `inset_content` helper added in POLISH-001 applied to
     tutorial surfaces only; discovery, init, and the onboarding welcome
     surface rendered flush against the shell chrome. Fix: hoisted
     `inset_content` and its margin constants into `crates/anvil-tui/src/shell.rs`
     as public helpers, and threaded them through `init/render.rs`,
     `tutorial/discovery_render.rs`, `onboarding/welcome_render.rs`, and the
     new `init_complete.rs`. Tutorial `render.rs` now re-uses the shared
     helper. Init snapshots were re-captured to reflect the shift.

- **Expected Outcome:**
  - Running `anvil start` in a repo that already has `.anvilrc` skips the
    init wizard (config detection works for the real filename).
  - After guided init succeeds, the user sees a landing / "what just happened"
    screen summarising what was written (`.anvilrc`, `plans/`, `.gitignore`
    entry) and what's next, before the tutorial launches. Wire up
    `CompletionState` or equivalent.
  - Discovery, init, and onboarding welcome inherit the same horizontal /
    top padding as the tutorial surfaces, either by hoisting `inset_content`
    into a shared helper or by applying it per-surface.

- **Validation:**
  - New unit test `config_exists_detects_anvilrc` (added alongside the fix).
  - Manual run of `anvil start` on a repo with `.anvilrc` already present —
    init wizard should be skipped.
  - Manual run of `anvil start` on a fresh repo — completion screen appears
    between init and tutorial.
  - Screenshot comparison of discovery and init surfaces before/after
    padding change.

- **Files:**
  - `crates/anvil-tui/src/surfaces/onboarding/mod.rs` — config detection (done)
  - `crates/anvil-cli/src/commands/welcome.rs` — wire `CompletionState` into
    `run_guided_init` → discovery transition
  - `crates/anvil-tui/src/surfaces/onboarding/complete.rs` — review copy, ensure
    `OnboardingSummary` carries the real fields (files written, next step)
  - `crates/anvil-tui/src/surfaces/tutorial/discovery_render.rs`,
    `crates/anvil-tui/src/surfaces/init/`, `crates/anvil-tui/src/surfaces/onboarding/welcome_render.rs` —
    apply shared outer padding

- **Deferred from April 2026 council review (session council-d4d5df8b) —
  roll into this work item:**
  - `render_complete` multi-completed-paths snapshot fixture missing
    (`crates/anvil-tui/src/surfaces/tutorial/render.rs:248`).
  - `Wrap { trim: false }` does not break long unbreakable tokens; `step.title`
    is unwrapped and can overflow border line
    (`crates/anvil-tui/src/surfaces/tutorial/render.rs:398`).
  - Unicode geometric shapes (`● ◉ ○`) may render double-wide on some
    Windows/SSH terminals — consider ASCII fallback via env var or a
    `unicode-width` assertion test (`render.rs:259`).
  - Tutorial snapshots are not width-parameterised (no coverage below
    40 columns); add snapshots at (20, 10) and (40, 10)
    (`render.rs:541`).
  - TOCTOU race between `config_exists` and `generate_config` — use
    `OpenOptions::create_new(true)` when writing
    (`crates/anvil-cli/src/commands/welcome.rs:206`).
  - Copy owner for landing-screen text not yet assigned; block item 2
    on the copy decision before starting.
- **Confidence:** high for items 1 and 3; medium for item 2 (needs design for
  summary copy — what exactly should the landing page say?).
- **Priority:** Medium — items 1, 2, and 3 shipped; only the council-deferred
  follow-ups below remain.
- **Status:** In Progress — config detection, landing screen, and shared
  outer padding all landed; council-deferred follow-ups remain.

---

- All POLISH tasks complete or explicitly deferred with rationale
- Full welcome → tutorial → completion flow navigable without issues
- Screenshot set captured for use in docs and early access onboarding
- No regressions in existing TUI surfaces

## Notes

Identified during April 2026 TUI demo review (Morgan Brighthand).
Demo screenshots saved at `workspace/tui-screenshots.html`.
