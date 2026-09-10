# Release User Journeys — First-Run Advocacy and Daily Confidence

| ID | Type | Owner | Priority | Status | Progress |
| -- | ---- | ----- | -------- | ------ | -------- |
| JOURNEY | Conductor | Josh | high | In Progress | 13/16 |

**Last reviewed:** 2026-09-10 — JOURNEY-016 acceptance passed (claim #4613) on pinned `5489c6112`. JSIMP-001..006 Merged; JOURNEY-013 closed as fixed (no splash); JOURNEY-015 rehearsal Merged. No publication.
Stored Progress stays **13/16** until an exclusive ADR-053 APS reconcile bumps N/M (acceptance closeout does not invent the counter).
Historical delivery evidence remains unchanged.

## Purpose

Coordinate a continuous new- and returning-user journey. The current programme
repairs reliability, rehearses a pinned main build through the existing release
process, then simplifies the experience. It does not require an internal release
mechanism or authorise publication. The original release outcomes remain:

 `anvil welcome` gives a new user a
repository-specific first win worth sharing, and `anvil start` gives a time-poor
senior developer trustworthy protection with almost no learning or repeat-run
friction.

This is a **conductor** module. It owns the release-worthy sequence, product
acceptance gates, cross-platform rehearsal, and evidence. Implementation stays
in the coordinated vertical modules.

## Product Outcome

The release slice is complete when:

- `anvil welcome` reaches truthful repository-specific value within 60 seconds;
- a real finding leads to an inspect-before-write first-win path;
- clean repositories get an honest clean result or isolated sandbox option;
- interactive `anvil start` completes consent and installation end to end;
- healthy repeat start is terse and confidence-building;
- bare `anvil` is the daily on-switch (daemon + existing MCP ensure) without
  re-offering declined installs; `anvil start` remains activate/reconfigure
  (JOURNEY-011 / ADR-114);
- cumulative value can be shared without leaking repository or personal data;
- fresh-install, restart/reboot, repeat, degraded, and recovery journeys pass on
  Linux, macOS, and Windows;
- the release record carries reproducible evidence for every required journey.

## Scope

### Historical v0.9.0 Release Cut

- WOW-005 repository-specific first win.
- ACTTUI-009, ACTTUI-010, and ACTTUI-012 activation completion and default gate.
- CIB-183, CIB-184, and CIB-190 repeat-use and consent discipline.
- CIB-073 cumulative and shareable value receipt.
- Candidate-binary journey rehearsal on Linux, macOS, and Windows.
- Outcome evidence and final release decision.

### Existing Expansion

- WOW-006 sandbox autoplay.
- ACTTUI-005 celebration treatment and ACTTUI-006/-011 richer diagnostics.
- ACTMO-021 / CIB-075 optional always-on protection indicator.
- DASH browser continuity for later team-lead and operator journeys.

These items remain coordinated and visible but do not block the release unless
the operator explicitly promotes them into the cut.

### Current Reliability and Simplification Programme

[JREL](../archive/modules/journey-reliability.aps.md) owns twelve bounded reliability items.
[JSIMP](./journey-simplification.aps.md) owns six subsequent simplification items.
JOURNEY-014..016 own acceptance and handoff, not duplicate implementation.
JOURNEY-013 first-user navigation observation is closed as fixed; JSIMP-001 may consume the recorded residuals.

No next version or release claim is selected here. The active window remains
provisional in [RELEASE-PLAN](../../RELEASE-PLAN.md). A reliable-journey claim
requires JOURNEY-014 and JOURNEY-015 evidence; a broader simplification claim
also requires JOURNEY-016. Unrelated urgent hotfixes retain their existing process.

## Constraints

- Repository-specific claims come from real scan or witness evidence.
- Example and sandbox evidence is labelled and isolated from the user's repo.
- No repository write without explicit, named, unticked consent.
- Machine output and non-interactive paths stay deterministic.
- No telemetry is added by this conductor.
- Protection state wording follows existing honesty contracts.
- Each unsuccessful ending has one recovery owner.
- The conductor does not duplicate implementation work from owning modules.

## Coordinated Modules

| Module | Role | Release posture |
| ------ | ---- | --------------- |
| [journey-reliability](../archive/modules/journey-reliability.aps.md) | Daemon/MCP lifecycle, truthful readiness, setup recovery and executable regression coverage | Current reliability gate: JOURNEY-014 |
| [journey-simplification](./journey-simplification.aps.md) | One command contract, resumable setup, quiet daily use and useful first proof | Starts after JOURNEY-015; acceptance at JOURNEY-016 |
| [first-run-wow](../archive/modules/first-run-wow.aps.md) | Repository-specific first win and sandbox tutorial | WOW-005 required; WOW-006 expansion |
| [activation-tui](../archive/modules/activation-tui.aps.md) | Interactive activation, consent, contracts, celebration and diagnostics | ACTTUI-009/-010/-012 required; -005/-006/-011 expansion where not already required by their owner |
| [activation-mcp-optional](./activation-mcp-optional.aps.md) | Daemon, durable registration, MCP-optional protection, optional local control app | Existing spine required; ACTMO-021 expansion |
| [daemon-save-time-validation](../archive/modules/daemon-save-time-validation.aps.md) | Headless background save-time driver | Existing v0.9 usefulness cut-line |
| [continuous-improvement-backlog](./continuous-improvement-backlog.aps.md) | Share receipt, quiet repeat start, consent parity, local repeat value | CIB-073/-183/-184/-190 required; CIB-075 expansion |
| [bare-ensure](../archive/modules/bare-ensure.aps.md) | Bare `anvil` daily ensure vs `anvil start` reconfigure | JOURNEY-011; ADR-114 |
| [usage-insights](../archive/modules/usage-insights.aps.md) | Local-only value aggregates and evidence semantics | Producer consumed by CIB-073/-190 when trustworthy |
| [dashboard-foundation](../archive/modules/dashboard-foundation.aps.md) | Browser foundation for later continuity | Expansion; non-blocking |

## Work Items

### JOURNEY-001: Repository-specific first win

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-07-11 via PRs #3263/#3280 — ACTTUI-009 consent
  wiring + WOW-005 first-win reroute (deterministic top-of-discovery
  selection, diff-before-write with expected-before TOCTOU guard, unticked
  consent, decline→picker, honest clean interstitial). Validation on main:
  `cargo test -p eddacraft-anvil-tui tutorial` 346 passed, `welcome` 40
  passed, zero failures.
- **Intent:** Make the first minute of `anvil welcome` demonstrate value on the
  user's repository rather than requiring them to discover the strongest path.
- **Expected Outcome:** Discovery deterministically selects the highest-value
  actionable real finding, explains it in plain language, shows a proposed diff
  before any write, and requires explicit unticked consent to apply; a clean
  repository renders an honest clean result and may offer the isolated sandbox.
- **Dependencies:** ACTTUI-009
- **Coordinates with:** WOW-005, CIB-170, ACTTUI-009
- **Validation:** `cargo test -p eddacraft-anvil-tui tutorial`; `cargo test -p
  eddacraft-anvil-tui welcome`; candidate rehearsal records time to first
  repository-specific value.
- **Confidence:** medium

### JOURNEY-002: `anvil start` just-works activation gate

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-07-11 via PRs #3263/#3279/#3284 — consent reachable
  and applied end to end (ACTTUI-009), contract matrix pinned (ACTTUI-010),
  polish/dead-key/exit-story cleared (ACTTUI-012), plain MCP picker unticked
  (CIB-184). Validation on main: `cargo test -p eddacraft-anvil start` 168
  passed, `eddacraft-anvil-tui activation` 42 passed, activation e2e 8/8.
  The TTY-default flip itself remains a phase-C decision outside this gate.
- **Intent:** Ensure the flagship interactive activation path completes the same
  real work as the proven plain path without hidden skips or terminal hazards.
- **Expected Outcome:** Consent is reachable and applied, TTY/plain/verify/JSON
  contracts pass, raw mode restores, the default interactive path is consistent,
  and every terminal state has one next action.
- **Dependencies:** ACTTUI-009, ACTTUI-010, ACTTUI-012, CIB-184
- **Coordinates with:** ACTTUI-009, ACTTUI-010, ACTTUI-012, CIB-184
- **Validation:** `cargo test -p eddacraft-anvil start`; `cargo test -p
  eddacraft-anvil-tui activation`; `pnpm e2e -- --testPathPattern activation`.
- **Confidence:** high

### JOURNEY-003: Daily confidence loop

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-07-12 via PRs #3283/#3286 — healthy repeat output
  collapses to state/posture/one-next-step (evidence-gated, never on repair
  paths), with an optional trustworthy local value line (two-sided
  freshness, honest omission, 150 ms budget). Validation on main:
  `cargo test -p eddacraft-anvil start` 179 passed, `value_receipt` 11,
  `insights` 47; healthy/absent/stale/repair/redaction fixtures all covered.
  Rehearsed live: repeat start 0.026 s, byte-identical, collapsed to 6 lines.
- **Intent:** Make a healthy repeat `anvil start` feel like a fast confidence
  check rather than repeated onboarding.
- **Expected Outcome:** Healthy repeat output collapses to protection, daemon and
  worktree posture, at most one next action, and an optional trustworthy local
  value line; repair states retain actionable detail and scripted contracts stay
  deterministic.
- **Dependencies:** CIB-183, CIB-190
- **Coordinates with:** CIB-183, CIB-190, INSIGHTS-001, ACTTUI-010
- **Validation:** `cargo test -p eddacraft-anvil start`; `cargo test -p
  eddacraft-anvil insights`; healthy repeat, absent-evidence, stale-evidence,
  and repair fixtures are all covered.
- **Confidence:** medium

### JOURNEY-004: Advocacy-grade value receipt

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-07-11 via PR #3282 — cumulative + bounded-window
  aggregates in `anvil insights`, deterministic self-contained `--share`
  card (create-new default, symlink-refusing, 0o600) naming its evidence
  window; redaction proven structurally (only counts and re-serialised
  dates can reach the artefact) and by marker-seeded fixtures. Validation
  on main: `cargo test -p eddacraft-anvil -- insights` 47 passed.
- **Intent:** Give users a credible, privacy-safe artefact that communicates
  what Anvil has done without exposing their codebase.
- **Expected Outcome:** Cumulative and bounded-window value evidence is available
  through `anvil insights`, and a deterministic self-contained share format
  redacts paths, repository internals, secret values, and personal data by
  default while naming its evidence window.
- **Dependencies:** CIB-073
- **Coordinates with:** CIB-073, INSIGHTS-001..005
- **Validation:** `cargo test -p eddacraft-anvil -- insights`; redaction fixtures
  prove no repository path, secret value, or personal identifier is emitted.
- **Confidence:** medium

### JOURNEY-005: Three-platform release journey rehearsal

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-07-12 — Linux interactive rehearsal complete on
  candidate `d6d3aa39c`
  ([rehearsal record](../audits/2026-07-12-journey-005-linux-rehearsal.md)):
  fresh welcome, first/repeat start, `--verify`/`--json` byte contracts,
  no-MCP, daemon stop/restart with durable-registration reload, and a
  repair path all pass. macOS/Windows evidence per the operator's ESC-001
  **accept-CI** resolution (2026-07-12): the full `rust.yml` cross matrix is
  green on main (run 29171614019) after the candidate runs surfaced and the
  loop fixed the accumulated non-unix dead-code drift (PR #3290) and the
  macOS/APFS `base_store` claim races + Windows harnesses (CIB-194 via PR
  #3297). Manual macOS/Windows interactive legs were explicitly waived for
  this cut; the PR-CI cross-lint gap stays tracked as CIB-193.
- **Intent:** Prove the release candidate survives the real installation,
  restart, repeat-use, degraded, and recovery paths on every supported platform.
- **Expected Outcome:** One candidate SHA has recorded Linux, macOS, and Windows
  evidence for fresh `welcome`, authenticated `start`, no-MCP, restart-required,
  daemon restart/reboot, durable worktree registration, healthy repeat, and one
  repair path; required failures block the cut.
- **Dependencies:** JOURNEY-001, JOURNEY-002, JOURNEY-003
- **Coordinates with:** ACTMO-010, ACTMO-014..020, DSV, ACTTUI-010
- **Validation:** `pnpm validate:full`; release-readiness and platform smoke
  workflows pass on the same source SHA; rehearsal record is linked from the
  release record.
- **Confidence:** medium

### JOURNEY-006: Outcome-based release gate

- **Status:** Released/Shipped via v0.9.0-beta (6b0ed1d1 · 2026-07-12). Merged 2026-07-12 — the tag landed: `v0.9.0-beta` cut at source
  `6b0ed1d1d` and published on both repos (release run 29190475570, success on
  attempt 3 after the operator rotated the expired `ANVIL_RELEASES_TOKEN`;
  verification record and closeout on
  [#3305](https://github.com/eddacraft/anvil-001/issues/3305)). The operator
  approved the cut (ESC-002 **approve**, 2026-07-12) on the assembled evidence
  matrix (Linux metrics in the
  [rehearsal record](../audits/2026-07-12-journey-005-linux-rehearsal.md):
  first run 0.34 s, healthy repeat 0.026 s / 6 lines, one-next-action
  compliance on every observed terminal state, byte-stable machine contracts,
  redaction green; cross matrix green on main run 29171614019). Released/
  Shipped via v0.9.0-beta (2026-07-12); record:
  [`plans/releases/v0.9.0-beta.md`](../releases/v0.9.0-beta.md); execution
  journal: [`plans/execution/JOURNEY-006.actions.md`](../execution/JOURNEY-006.actions.md).
- **Intent:** Decide the cut from reproducible journey outcomes rather than the
  completion of disconnected feature lists.
- **Expected Outcome:** The candidate records time to first value, activation
  terminal state, healthy repeat duration/output size, share-receipt privacy,
  one-next-action compliance, and platform results; every required threshold is
  met or explicitly rejected by the operator before tagging.
- **Dependencies:** JOURNEY-004, JOURNEY-005
- **Coordinates with:** WOW, ACTTUI, ACTMO, DSV, CIB, INSIGHTS
- **Validation:** `pnpm release-plan:check`; `pnpm docs:check`; release record
  contains the conductor evidence matrix.
- **Confidence:** high

### JOURNEY-007: Sandboxed autoplay demonstration

- **Status:** Released/Shipped via v0.9.1-beta (6a971188 · 2026-08-02). Merged 2026-07-30 via PR #3441 — WOW-006 sandboxed autoplay landed
  on `main` (`feat/wow-006-sandbox-autoplay`). Design at
  [`plans/specs/2026-07-26-wow-006-autoplay-demo.md`](../specs/2026-07-26-wow-006-autoplay-demo.md):
  fresh-tempdir offline fixture with RAII cleanup, real in-sandbox execution
  behind a path-containment guard, `anvil tutorial --autoplay` + a picker
  discovery row, session-scoped autoplay that survives the watch-demo
  transition, and pacing that extends the WOW-002 reveal driver.
- **Intent:** Preserve a hands-free, isolated demonstration for clean repos and
  demos without making animation a substitute for real repository value.
- **Expected Outcome:** WOW-006 runs deterministically in a temporary fixture,
  never writes to the user's repository, cleans up safely, and hands control back
  on input.
- **Dependencies:** WOW-006, JOURNEY-001
- **Coordinates with:** WOW-006, ACTTUI-005
- **Validation:** Deterministic fixture yields an identical finding set offline;
  autoplay runs `ProtectionLoop` to completion unattended; a keypress on each
  surface converts to the interactive tutorial; the containment guard rejects an
  out-of-sandbox target; the tempdir is removed on exit (per the design doc).
- **Confidence:** medium

### JOURNEY-008: Celebration and richer diagnostics

- **Status:** Released/Shipped via v0.9.1-beta (6a971188 · 2026-08-02). Merged 2026-07-25 via PR #3408 — the "richer diagnostics on demand" half
  already shipped with ACTTUI-006 (the `LogPanel` `l`-toggle and in-surface
  `--why`). This slice adds the remaining celebration half: a
  once-per-local-environment `BigBanner` beat on the **first** protecting
  activation only — the run that actually writes the LAUNCH-010 baseline
  (`ActivationStep::BaselineSample` in the post-consent `project_applied` set,
  not merely the ticked box), silent on healthy repeat runs. The banner is a
  decorative flourish above the honesty-pinned `StatusBadge` headline — never a
  replacement, and suppressed on panes too short to hold both — and reuses the
  fixed `protecting` vocabulary so it cannot overclaim (reverses ACTTUI-012's
  removal of the earlier unused wiring).
- **Intent:** Add optional delight and operator depth after the core activation
  path is complete and trustworthy.
- **Expected Outcome:** First success may use the shared celebration treatment;
  typed evidence and diagnostic panes remain available on demand without adding
  noise to healthy repeat use.
- **Dependencies:** JOURNEY-002
- **Coordinates with:** ACTTUI-005, ACTTUI-006, ACTTUI-011
- **Files:** `crates/anvil-tui/src/surfaces/activation/verdict.rs`,
  `crates/anvil-tui/Cargo.toml` (`big-text` feature),
  `crates/anvil-cli/src/commands/start.rs`
- **Validation:** `cargo test -p eddacraft-anvil-tui activation` (47 passed;
  first-run celebrates, repeat/repair/watching do not, banner text stays
  honest, banner sits above the headline).
- **Confidence:** medium

### JOURNEY-009: Always-on confidence indicator

- **Status:** Proposed — **on hold 2026-07-26**: the operator has an alternate
  app-side plan in progress. Do **not** build ACTMO-021 as specced (the scoped
  local daemon-control app) without reconciling against that plan first. Left
  Proposed deliberately to avoid pre-empting or racing it.
- **Intent:** Explore a human-visible protection indicator without expanding it
  into a second findings or configuration product.
- **Expected Outcome:** If promoted, a scoped local control surface reports
  daemon state, registered worktrees, current protection posture, and recent
  value using existing IPC and local evidence only.
- **Dependencies:** ACTMO-021
- **Coordinates with:** ACTMO-021, CIB-075, CIB-073
- **Validation:** Promotion decision and platform spike evidence from the owning
  item.
- **Confidence:** low

### JOURNEY-010: Browser continuity

- **Status:** Proposed — **blocked 2026-07-26** on remaining DASH view waves:
  DASHCORE is 9/9 Merged (pending release evidence), but DASHARCH and DASHOPS
  have not yet built the browser views this item would carry continuity into,
  so a full design remains premature.
  Design intent is settled and recorded here: DASH must **consume existing typed
  contracts** (activation, protection, finding, value) and must not redefine
  that truth in the browser; terminal usefulness stays independent of the
  browser. Promote to Ready once at least one DASH view wave lands and defines
  its contract tests.
- **Intent:** Carry the same protection and value vocabulary into the browser
  after the terminal journeys meet the release bar.
- **Expected Outcome:** DASH consumes existing typed contracts and does not
  redefine activation, protection, finding, or value truth; terminal usefulness
  remains independent of the browser.
- **Dependencies:** DASH-001..011, JOURNEY-006
- **Coordinates with:** DASH, DASHCORE, DASHARCH, DASHOPS
- **Validation:** Dashboard contract tests and browser journey tests defined by
  the owning DASH modules.
- **Confidence:** medium

### JOURNEY-011: Bare `anvil` daily ensure vs `start` reconfigure

- **Status:** Released/Shipped via v0.9.1-beta (6a971188 · 2026-08-02). Merged 2026-08-01 via PR
  [#3474](https://github.com/eddacraft/anvil-001/pull/3474) (`0388a432a` on
  `main`). ADR-114 Accepted; ONSW-001..006 Merged. Evidence: `bare_invocation`
  + `ensure_existing_*` tests green; worktree smoke (`protecting` + registered);
  not-activated / non-worktree fail closed.
- **Intent:** Finish the daily-confidence story: bare `anvil` is the on-switch
  (daemon + existing MCP ensure, no re-offer of declined installs); `anvil start`
  stays activate/reconfigure/reinstall. Resolves the contradiction between
  JOURNEY-003 quiet repeats and consent re-offers on every `start`.
- **Expected Outcome:** Rehearsal evidence shows (1) first-run / never-activated
  bare is honest and does not install MCP/workflows; (2) healthy subsequent bare
  ensure is terse and turns the spine on without pickers; (3) after a deliberate
  decline on `start`, bare still does not install, while `start` can still offer
  NotPresent clients; (4) help/runbook state the split.
- **Dependencies:** ONSW-001..006, ADR-114
- **Coordinates with:** ONSW, ACTMO, ADR-044/082/092, CIB-177 (supersede bare
  exit-2-always)
- **Validation:** ONSW validation suite green; short interactive rehearsal
  record linked from this item or the release notes for the shipping window;
  `cargo test -p eddacraft-anvil ensure`; `cargo test -p eddacraft-anvil start`.
- **Confidence:** high

### JOURNEY-012: `anvil start` points a new user at the tutorial

- **Status:** Merged 2026-09-02 via PR #4317
- **Reconciliation (2026-09-05):** The tutorial pointer is implemented on the
  interactive activation path and suppressed after any tutorial path is completed.
  JOURNEY-013 first-user observation is now Ready in the reliability-first programme; no splash is authorised by this
  closeout.
- **Intent:** the tutorial is offered on the post-install banner
  (`install.sh:167-168`, CIB-288) and second of three in first-run onboarding
  (`crates/anvil-tui/src/surfaces/onboarding/welcome.rs:23-37`), but **`anvil start` never mentions
  `anvil welcome` or the tutorial at all** — the only matches in
  `crates/anvil-cli/src/commands/start.rs` are an unrelated code comment. A first-time user who
  installs anvil and runs `anvil start` is therefore never told the tutorial
  exists, because the offer lives exclusively on the `anvil welcome` path.
  Observed on a shadowed first-time install; the returning-user hub ordering
  (tutorial fifth of seven) is **not** the cause, since that surface is never
  reached on this path.
- **Expected Outcome:** on the interactive TTY path, `anvil start` ends by
  naming `anvil welcome` as the next step, shown while — and only while —
  `~/.anvil/tutorial-progress.json` records no completed path. The pointer
  reads existing user-scoped state
  (`crates/anvil-cli/src/commands/tutorial.rs:400-402`, `crates/anvil-tui/src/surfaces/tutorial/mod.rs:381`) and writes
  none, so it disappears permanently once any path is completed, with no config
  surface and no new persisted state.
- **Non-scope / do not:** do not reorder the welcome hub, change the installer
  banner (CIB-288 deliberately leads with the ungated command and has a guard
  test), or touch the onboarding menu. Do not add a dismissal prompt or read a
  keypress — `anvil start` must keep exiting deterministically. Do not emit the
  pointer on `--json`, `--verify`, piped or CI paths; those carry byte-stable
  single-document contracts (ADR-103). Do not create or migrate
  `tutorial-progress.json` from activation — unreadable or absent means "not
  completed".
- **Files:** `crates/anvil-cli/src/commands/start.rs`,
  `crates/anvil-cli/src/activation/render.rs`,
  `crates/anvil-cli/src/commands/tutorial.rs` (read-only reuse of
  `progress_file_path`)
- **Dependencies:** none
- **Coordinates with:** CIB-288 (installer banner, already correct), CIB-351
  (learning-path entry), ADR-103 (TTY-default and its degradation contracts),
  JOURNEY-013
- **Validation:** `cargo test -p eddacraft-anvil --no-fail-fast -- start`; a
  test proving the pointer is absent once a completed path is present and
  present when none is; a test proving `--json` / `--verify` output is
  byte-unchanged.
- **Design:** [`onboarding entry design`](../specs/2026-09-01-onboarding-entry-design.md)
- **Confidence:** high — the gap and the state file were both read in source.

### JOURNEY-013: Re-test first-run activation navigation before designing a splash

- **Status:** Merged — claim #4577. Evidence: [2026-09-10 first-run activation navigation](../audits/2026-09-10-journey-013-activation-navigation.md) on `62e1facd70abd8de0311946e3fc6d4c401e9db7b` (carries `e586b6e53`). Decision: **close as fixed** — no splash; no stepping-affordance change in this item.
- **Intent:** the operator observed a first-time user struggling to move between
  the `anvil start` consent screens and proposed a first-run splash teaching the
  key model. Investigation found the observed build cannot support that
  conclusion: at `v0.9.7-beta` `consent_help_text` **does not exist** — the
  consent picker had no help bar at all, and the only help string in that file
  belongs to the evidence pane and reads `j/k scroll …`. The contextual bar
  naming arrows landed in `e586b6e53` (2026-08-21), which `git tag --contains`
  places in no release tag. An unknown share of the observed difficulty is
  therefore already fixed and unreleased, and a first-run splash is expensive to
  walk back.
- **Expected Outcome:** one first-time user is observed against a build carrying
  the contextual help bar, and the residual difficulty (if any) is recorded
  concretely enough to design against — distinguishing "did not see the hint",
  "did not understand that arrows step between sections", and "did not
  understand the picker is a form that needs a deliberate toggle". The outcome
  is a decision to design a splash, to change the stepping affordance, or to
  close this as fixed.
- **Non-scope / do not:** do not design or build a splash, a key-hint overlay,
  or a progress indicator before this evidence exists. Do not treat the
  `h`/`j`/`k`/`l` aliases as missing — they have been live throughout, as
  deliberate silent aliases.
- **Dependencies:** JOURNEY-015; the observed pinned build must carry `e586b6e53`
- **Coordinates with:** ADR-103, ACTTUI (archived), JOURNEY-012
- **Validation:** a short observation record linked from this item, naming the
  build SHA the session ran.
- **Design:** [`onboarding entry design`](../specs/2026-09-01-onboarding-entry-design.md)
- **Confidence:** high on the diagnosis; the item exists precisely because
  confidence in the *remedy* is low until re-measured.


### JOURNEY-014: Reliability closure and regression evidence gate

- **Status:** Merged — claim #4570. Evidence: [2026-09-10 reliability gate](../audits/2026-09-10-journey-014-reliability-gate.md) + [identity](../audits/2026-09-10-journey-014-identity.json) on pinned `9684aa560`. APS lint clean; journey verify pass (6/6 required; upgrade optional/not-supplied). E2E clears host ANVIL_NO_SAVE_TIME_DRIVER unless a test opts in.
- **Intent:** Establish that the complete setup and daily-use spine works before changing the journey contract.
- **Expected Outcome:** Every JREL item has a linked reproduction and passing regression on its merged implementation, or evidence that the suspected defect was already fixed. No unresolved P0/P1 failure in the required journeys is labelled complete. Existing-owner work from CIB-405 / issue #4231 is reconciled by its owners before JREL-011 closes. Evidence identifies source SHA, platform, command, actual executed scenarios and remaining limitations.
- **Dependencies:** JREL-001, JREL-002, JREL-003, JREL-004, JREL-005, JREL-006, JREL-007, JREL-008, JREL-009, JREL-010, JREL-011, JREL-012
- **Coordinates with:** JREL, MCPLH, ACTMO, CIB
- **Validation:** `pnpm aps:active-lint`; execute the non-skipping command delivered by JREL-012 against the pinned build; check each linked regression result and ensure no missing/skipped required leg is reported as passed.
- **Confidence:** high — Linux verify identity recorded; cross-platform and upgrade legs remain JOURNEY-015.

### JOURNEY-015: Pinned-main journey rehearsal and existing release handoff

- **Status:** Merged — claim #4572. Evidence: [2026-09-10 pinned-main rehearsal](../audits/2026-09-10-journey-015-pinned-main-rehearsal.md) + [identity](../audits/2026-09-10-journey-015-identity.json) + [CI identity](../audits/2026-09-10-journey-015-ci-identity.json) on pinned `3f8890e15`. Local journey:verify pass (7/7, upgrade required); CI journey-verify pass (6/6 required). Operator platform waiver for macOS/Windows same-pin legs recorded by merge. Release disposition: claim freeze / publication not authorised; simplification permitted.
- **Intent:** Verify what a user will install without building an internal release system.
- **Expected Outcome:** Build from a recorded main SHA with the repository's supported build/CI process. Record binary version/hash and supported-platform results for fresh signed-out welcome, entitled start, optional/no MCP, one and multiple configured clients, healthy bare anvil, second repository/worktree, interrupted setup and resume, daemon death/restart, save-time child death, MCP restart/update, machine output and upgrade from the previous public build. Include real client calls and an observed save-time finding; config or PID presence is insufficient. Required failures block handoff. Record the release disposition and evidence in the existing release process; claim freeze, changelog, standing release gates and explicit publication authority remain required. Passing this gate permits simplification even if publication is scheduled later.
- **Dependencies:** JOURNEY-014
- **Coordinates with:** JREL-012, JOURNEY-013
- **Validation:** `pnpm validate:full`; `pnpm release-plan:check`; JREL-012's recorded end-to-end command; Linux/macOS/Windows matrix and interactive evidence on the same pinned source. Any platform waiver must be explicitly recorded by the operator, never inferred from a different platform's pass.
- **Confidence:** high — Linux conductor + CI identities recorded; macOS/Windows waived by operator merge; no publication claimed.

### JOURNEY-016: Simplified journey acceptance and documentation closeout

- **Status:** Merged — claim #4613. Evidence: [2026-09-10 simplified journey acceptance](../audits/2026-09-10-journey-016-simplified-acceptance.md) + [identity](../audits/2026-09-10-journey-016-identity.json) on pinned `5489c6112`. Decision: **pass**. No splash; no publication.
- **Intent:** Prove first-time and returning users experience one understandable journey after the reliability repairs.
- **Expected Outcome:** JSIMP's agreed command/state contract is implemented, documented and verified. Observe a first-time user from installation through first useful proof and a returning user on a later session and second repository. Both can identify current coverage and their next action without undocumented repair steps. Setup resumes, declined integrations stay declined, healthy daily invocation remains quiet, and changed machine contracts have explicit compatibility treatment. Record residual usability issues and resolve blocking ones before making a simplification release claim.
- **Dependencies:** JOURNEY-015, JSIMP-001, JSIMP-002, JSIMP-003, JSIMP-004, JSIMP-005, JSIMP-006
- **Coordinates with:** JOURNEY-013, JSIMP
- **Files:** `plans/audits/2026-09-10-journey-016-simplified-acceptance.md`, `plans/audits/2026-09-10-journey-016-identity.json`, `RELEASE-PLAN.md`
- **Validation:** `pnpm docs:check`; `pnpm aps:active-lint`; JREL-012's non-skipping journey command extended by JSIMP-006; linked first-time/returning-user observations against the same build.
- **Confidence:** high — Linux conductor + first-time/returning observation recorded; publication not claimed.

## Sequencing

| Phase | Owning work | Exit condition |
| ----- | ----------- | -------------- |
| Reliability | Start JREL-001/-002 and JREL-012 harness; execute remaining JREL items according to their Dependencies | JOURNEY-014 evidence gate passes |
| Rehearsal and release handoff | JOURNEY-015 on a pinned main build, using the existing public release process | Complete journey evidence and recorded release disposition; no internal channel needed |
| Simplification decision | JOURNEY-013 observation closed as fixed; JSIMP-001 contract/ADR decision | Agreed contract before downstream changes |
| Simplification delivery | JSIMP-002..006 in dependency order | JOURNEY-016 acceptance passes |

Dependencies on each item are authoritative. Historical JOURNEY-001..012
delivery remains recorded above; JOURNEY-009/-010 retain their hold/block and
are not pulled into this programme.

## Release Gate

JOURNEY-006 remains the historical v0.9.0 gate. For the current programme,
JOURNEY-014 and JOURNEY-015 gate any reliable-journey release claim;
JOURNEY-016 additionally gates a simplification claim. Passing a planning or
implementation gate does not itself publish a release.
