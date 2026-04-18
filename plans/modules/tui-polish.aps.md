<!--
APS Module: TUI Polish
=========================
Fix rough edges in the existing Ratatui TUI surfaces:
welcome screen, tutorial flow, progress indicators,
completion states, and text rendering.

Scopes: POLISH (main)
-->

# TUI Polish

| ID     | Owner | Status |
| ------ | ----- | ------ |
| POLISH | —     | In Progress |

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
- **Status:** Ready

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
- **Status:** Ready

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
- **Status:** Ready

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
- **Status:** Ready

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
- **Status:** Ready

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
- **Status:** Ready

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



- All POLISH tasks complete or explicitly deferred with rationale
- Full welcome → tutorial → completion flow navigable without issues
- Screenshot set captured for use in docs and early access onboarding
- No regressions in existing TUI surfaces

## Notes

Identified during April 2026 TUI demo review (Morgan Brighthand).
Demo screenshots saved at `workspace/tui-screenshots.html`.
