# First-Run Wow — Tutorial and Welcome Surface Uplift

| ID  | Owner | Status      | Progress |
| --- | ----- | ----------- | -------- |
| WOW | Josh  | In Progress | 5/6      |

**Last reviewed:** 2026-07-27 — the operator-approved
[`JOURNEY` conductor](./release-user-journeys.aps.md) promotes WOW-005 into the
release cut and retains WOW-006 as a coordinated post-cut enhancement. The
module was created 2026-07-08 via planning-workflow from the
operator's first-run-experience review (interactive session): the tutorial's
Enter-executes-a-real-command behaviour is not evident before the keypress, and
the first-run journey underuses the discovery scan's real findings. WOW-001..004
are Merged. WOW-005's first-win direction and consent boundary are accepted by
the JOURNEY design, while its execution depends on ACTTUI-009's production
consent wiring. WOW-006 is In Progress: implementation and Council repair are
complete, with draft PR
[#3441](https://github.com/eddacraft/anvil-001/pull/3441) open for review.

## Purpose

Make the first five minutes of anvil demonstrably about the user's own repo,
and make the tutorial's command execution honest-by-affordance: a user should
know **before** pressing Enter whether the tutorial will run a real command,
and should see anvil's value on their own findings, not generic copy.

Complements — does not overlap — the 2026-07-04 welcome/start user-journey
repair items (CIB-162..179,
[audit](../audits/2026-07-04-anvil-start-welcome-user-journey.md)): those fix
honesty and navigation defects in the existing flow; this module adds
evidence affordances and personalization on top of the repaired baseline.

## In Scope

- Tutorial command-step evidence: visible command bar, run/read-only badges,
  step-aware footer help (`crates/anvil-tui/src/surfaces/tutorial/`).
- Typed-command execution presentation (deterministic, skippable animation).
- Path-picker personalization from existing per-domain scan findings.
- Completion-screen findings delta via re-scan.
- (Design-gated) First-win reroute after discovery; autoplay/demo mode.

## Out of Scope

- CIB-162..179 repair items (owned by CIB; coordinate, don't duplicate —
  especially CIB-170 showcase labelling and CIB-171 welcome TUI navigation).
- Web tutorials (UJ-012..015, shipped v0.8.0-beta).
- Any change to activation honesty contracts (LAUNCH-014 copy pins stay);
  no new claims of protection anywhere in tutorial copy.
- New scanner/enforcement capability — this is presentation and flow only.

## Constraints

- **Deterministic**: animations are fixed-timing and snapshot-testable; any
  re-scan reuses the discovery scanner so identical input yields identical
  findings. No wall-clock or randomness in rendered content.
- **Honesty pins**: the `no_path_claims_pre_write_protection` and
  LAUNCH-014 copy invariants in `tutorial::tests` must keep passing.
- **Consent**: nothing writes to the user's repo without an explicit
  per-action keypress on a surface that names the write (CIB-165 lesson:
  no pre-selected writes). Space-advances-without-running is preserved.
- **Static mode**: every feature degrades to the existing static
  (watcher-unavailable) walkthrough without new failure modes.

## Work Items

### WOW-001: Command-step evidence affordance

- **Status:** Merged 2026-07-08 via PR #3226
- **Intent:** A user can tell before pressing Enter whether the current
  tutorial step executes a real command, and whether it mutates their repo.
- **Expected Outcome:** Command steps render the command distinctly (prompt
  styled, visually separate from prose) with a badge distinguishing
  runs-in-your-repo from read-only; footer help differs between command and
  informational steps; Space's skip-without-running behaviour is discoverable.
- **Scope:** `TutorialStep` gains a declared mutating/read-only marker set in
  `paths.rs`; render + help-text changes; snapshot coverage for both variants.
- **Non-scope:** No change to execution semantics — commands still run only
  on Enter. (Reconciled 2026-07-08 during implementation: the original "no
  key-handling change" wording contradicted this module's own constraint
  "Space-advances-without-running is preserved" — before this item, space
  was a silent no-op on command steps, leaving no way to decline a command.
  Resolution: space now advances command steps without executing, pinned by
  `toggle_skips_command_step_without_executing`.)
- **Files:** `crates/anvil-tui/src/surfaces/tutorial/{mod,render,paths}.rs`
- **Validation:** `cargo test -p anvil-tui` (new snapshots: command step
  before execution, read-only vs mutating badge; help-text drift tests)
- **Confidence:** high

### WOW-002: Typed-command execution presentation

- **Status:** Merged 2026-07-08 via PR #3226
- **Dependencies:** WOW-001
- **Intent:** Executing a command reads as anvil visibly driving the
  terminal, making the run-for-real behaviour unmistakable at the moment it
  happens.
- **Expected Outcome:** On Enter, the command is revealed into the step's
  prompt line with deterministic fixed-interval pacing before output appears;
  any keypress completes the reveal instantly; static mode and failed-step
  retry/skip behaviour are unchanged; the TUI loop drives pacing via its
  existing tick, no threads.
- **Files:** `crates/anvil-tui/src/surfaces/tutorial/{mod,render}.rs`,
  `crates/anvil-cli/src/tui.rs`
- **Validation:** `cargo test -p anvil-tui -p anvil-cli` (state-machine tests
  for reveal states incl. skip-to-complete; snapshot mid-reveal at fixed tick)
- **Confidence:** high

### WOW-003: Personalized path picker

- **Status:** Merged 2026-07-08 via PR #3226
- **Intent:** The tutorial path picker shows each path's relevance to the
  user's repo using the discovery scan already threaded into the tutorial.
- **Expected Outcome:** When scan results are present, each path row shows its
  per-domain finding count (via the existing domain filter); zero-finding and
  no-scan cases fall back to current copy; showcase-derived counts are never
  presented as real findings (coordinate with CIB-170).
- **Files:** `crates/anvil-tui/src/surfaces/tutorial/{mod,render,discovery}.rs`
- **Validation:** `cargo test -p anvil-tui` (snapshots: picker with counts,
  clean repo, no scan)
- **Confidence:** high

### WOW-004: Completion findings delta

- **Status:** Merged 2026-07-08 via PR #3226
- **Intent:** Tutorial completion shows the user what changed in their repo
  during the walk instead of only offering the next path.
- **Expected Outcome:** The complete phase can present a before/after findings
  count for the chosen domain from a re-scan against the session's opening
  scan; re-scan is read-only and reuses the discovery scanner; absent scan
  results render the current completion screen unchanged.
- **Files:** `crates/anvil-tui/src/surfaces/tutorial/{mod,render}.rs`,
  `crates/anvil-cli/src/commands/welcome.rs`
- **Validation:** `cargo test -p anvil-tui -p anvil-cli` (delta rendering for
  improved / unchanged / regressed / no-scan cases)
- **Confidence:** medium

### WOW-005: First-win reroute after discovery

- **Status:** Merged 2026-07-11 via PR #3280 — first-win surface lands after
  discovery: deterministic highest-value actionable-finding selection,
  plain-language explanation, diff-before-write, fix apply behind ACTTUI's
  unticked consent chrome (gated under `project_writes_gated`), honest
  clean-repo result, decline lands on the path picker unchanged
- **Intent:** A first-run user lands on their repo's highest-severity real
  finding with a guided fix opportunity before seeing the generic path picker,
  so the first minute delivers a concrete win on their own code.
- **Design closeout:** Consent posture for a fix write during onboarding (per the
  Constraints section and the CIB-165 precedent: explicit, named, per-action
  consent; show the diff before apply) reuses ACTTUI's unticked consent model.
  Discovery deterministically selects the highest-value actionable real finding;
  declining returns to the path picker. A clean repository states the clean
  result and may offer the isolated WOW-006 sandbox, never showcase findings as
  local truth. CIB-127 coordination remains required so a tutorial-time fix does
  not confuse the activation baseline.
- **Depends on ACTTUI (added 2026-07-08):** the onboarding fix write must
  reuse ACTTUI's interactive consent chrome (`Confirm` / `Select` /
  `OverlayStack`) and its repo-scoped-write posture (CIB-165 unticked default;
  suppressed under `project_writes_gated`) rather than hand-roll a second,
  divergent consent surface — and the discovery → fix → picker flow must slot
  into the activation surface's phase model. ACTTUI-000/-004 established the
  model; ACTTUI-009 must make it reachable and apply selections in production.
  See [`activation-tui.aps.md`](./activation-tui.aps.md).
- **Expected Outcome:** First-run flow offers the highest-value actionable real
  finding as the default landing after discovery, explains why it matters, and
  shows the proposed diff before an explicit write; declining lands on the path
  picker exactly as today.
- **Validation:** `cargo test -p eddacraft-anvil-tui tutorial`; `cargo test -p
  eddacraft-anvil-tui welcome`; JOURNEY-005 candidate rehearsal records time to
  first repository-specific value and proves Enter-without-consent writes
  nothing.
- **Confidence:** medium

### WOW-006: Autoplay demo mode on a sandboxed fixture

- **Status:** In Progress 2026-07-27 — design gate **closed** (operator approved;
  design at
  [`plans/specs/2026-07-26-wow-006-autoplay-demo.md`](../specs/2026-07-26-wow-006-autoplay-demo.md)).
  Prerequisites are in place (ACTTUI-000 Done, ACTTUI-001 scaffold landed,
  WOW-002 reveal driver Merged, shared ACTTUI widgets available). The local
  implementation passed Council and independent verification; draft PR
  [#3441](https://github.com/eddacraft/anvil-001/pull/3441) is open against
  `main`. Coordinated by JOURNEY-007.
- **Intent:** A "watch anvil work" mode plays the tutorial hands-free —
  commands, inline-editor ghost-typing, verification — against a scaffolded
  temp fixture repo, so the demo executes for real without touching the
  user's repo, and any keypress hands control back.
- **Design decisions (gate closed):** (1) fresh OS tempdir, pinned offline
  fixture, RAII cleanup — never touches the user repo or `ANVIL_HOME`;
  (2) steps execute for real in-sandbox behind a path-containment guard that
  hard-aborts on any out-of-sandbox target; (3) `anvil tutorial --autoplay`
  flag + a tutorial-picker discovery row, never auto-firing; (4) session-scoped
  autoplay that survives the watch-demo transition, any keypress converts to
  interactive; (5) pacing extends the WOW-002 `reveal_tick` driver (reveal →
  dwell → auto-advance), no bespoke chrome. Full rationale in the design doc.
- **Depends on ACTTUI (added 2026-07-08):** the demo surface reuses ACTTUI's
  widget vocabulary (`ParallelProgress`, `OverlayStack`, `Toast`, `BigBanner`,
  shared `HelpBar`) and the WOW-002 reveal driver instead of a bespoke autoplay
  chrome, so the demo and the activation surface read as one product.
- **Expected Outcome:** A demo entry runs the ProtectionLoop path end-to-end
  unattended in a fixture repo with deterministic findings; interrupting at any
  point converts to the normal interactive tutorial.
- **Validation:** Deterministic fixture yields an identical offline finding set;
  each run owns a fresh tempdir; autoplay runs `ProtectionLoop` to completion
  unattended behind bounded command and watch deadlines; a keypress cancels
  latent automation and converts each surface to interactive; discovery and
  completion context is restored on teardown; containment rejects an
  out-of-sandbox target; child, watcher, terminal, and tempdir cleanup is
  exercised on success and error paths.
- **Files:** `crates/anvil-cli/src/commands/{tutorial.rs,tutorial/autoplay.rs,welcome.rs}`,
  `crates/anvil-cli/src/tui.rs`,
  `crates/anvil-tui/src/surfaces/tutorial/{executor,mod,paths,render,verify,watch_demo}.rs`,
  and the three updated tutorial picker snapshots.
- **Evidence (2026-07-28):** Council `council-4da7ba79` converged with eight
  findings fixed and none open; independent verify-loop passed; CLI tutorial
  30/30, TUI tutorial 370/370, and welcome 40/40 passed; strict Clippy,
  `pnpm validate:changed`, docs 9/9, APS lint, and APS index checks passed.
- **Confidence:** medium

## Sequencing

**Release cohort (operator decision updated 2026-07-11):** WOW-001..005 ship in
the same release as ACTTUI activation TUI — see
[`activation-tui.aps.md`](./activation-tui.aps.md) phase B (parallel tracks).

Wave 1: WOW-001 → WOW-002 (same surface, 002 builds on 001's render split);
WOW-003 and WOW-004 are independent of both and of each other — may run in
parallel with ACTTUI-003..005 after ACTTUI-002 lands. WOW-005 is now a JOURNEY
release-cut gate after ACTTUI-009; WOW-006 remains a coordinated post-cut
enhancement unless the operator explicitly promotes it.

## Coordination

- **CIB-170 / CIB-171** (showcase labelling, welcome TUI navigation): same
  files; land repair before or with WOW-003 to avoid conflicting picker edits.
- **UJ-011 open question** (in-terminal vs web tutorial narrative
  convergence): WOW changes presentation, not narrative order; no conflict.
- **ACTTUI (activation-tui)**: owns the interactive activation surface — the
  shared consent chrome (`Select` / `Confirm` / `OverlayStack`, CIB-165
  unticked posture, `project_writes_gated` gating) and the `eddacraft-tui`
  widget vocabulary. WOW-001..004 shipped in ACTTUI's phase-B cohort.
  WOW-005 execution is gated on ACTTUI-009's working production consent path;
  WOW-006 consumes the shared widget foundation when its post-cut design closes.
- **JOURNEY (release-user-journeys):** conductor for the release outcome.
  JOURNEY-001 requires WOW-005; JOURNEY-007 retains WOW-006 as visible post-cut
  expansion without making it a release blocker.
- **LAUNCH-014 / ADR-080**: tutorial honesty pins and the ungated welcome
  demo posture are load-bearing and unchanged.
