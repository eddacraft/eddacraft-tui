# Release User Journeys — First-Run Advocacy and Daily Confidence

| ID | Type | Owner | Priority | Status | Progress |
| -- | ---- | ----- | -------- | ------ | -------- |
| JOURNEY | Conductor | Josh | high | In Progress | 6/10 |

**Last reviewed:** 2026-07-12 — **`v0.9.0-beta` shipped** under JOURNEY-006:
tag at source `6b0ed1d1d`, published on both repos 2026-07-12T17:06Z
(release run 29190475570 attempt 3; verification and closeout on
[#3305](https://github.com/eddacraft/anvil-001/issues/3305); record:
[`plans/releases/v0.9.0-beta.md`](../releases/v0.9.0-beta.md)). The release
cut chain (JOURNEY-001..006) is complete — all six coordinated code items
Merged (CIB-184 #3279, WOW-005 #3280, CIB-073 #3282, CIB-183 #3283,
ACTTUI-012 #3284, CIB-190 #3286), outcomes verified on the candidate, Linux
rehearsal recorded
([audit](../audits/2026-07-12-journey-005-linux-rehearsal.md)), escalation
queue cleared (ESC-001 accept-CI, ESC-002 approve). JOURNEY-007..010 remain
Proposed post-cut expansion. Created 2026-07-11 from the operator's release
goal and the accepted
[`release user journeys conductor design`](../specs/2026-07-11-release-user-journeys-conductor.md).

## Purpose

Coordinate the release around two outcomes: `anvil welcome` gives a new user a
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
- cumulative value can be shared without leaking repository or personal data;
- fresh-install, restart/reboot, repeat, degraded, and recovery journeys pass on
  Linux, macOS, and Windows;
- the release record carries reproducible evidence for every required journey.

## Scope

### Release Cut

- WOW-005 repository-specific first win.
- ACTTUI-009, ACTTUI-010, and ACTTUI-012 activation completion and default gate.
- CIB-183, CIB-184, and CIB-190 repeat-use and consent discipline.
- CIB-073 cumulative and shareable value receipt.
- Candidate-binary journey rehearsal on Linux, macOS, and Windows.
- Outcome evidence and final release decision.

### Post-Cut Expansion

- WOW-006 sandbox autoplay.
- ACTTUI-005 celebration treatment and ACTTUI-006/-011 richer diagnostics.
- ACTMO-021 / CIB-075 optional always-on protection indicator.
- DASH browser continuity for later team-lead and operator journeys.

These items remain coordinated and visible but do not block the release unless
the operator explicitly promotes them into the cut.

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
| [first-run-wow](./first-run-wow.aps.md) | Repository-specific first win and sandbox tutorial | WOW-005 required; WOW-006 expansion |
| [activation-tui](./activation-tui.aps.md) | Interactive activation, consent, contracts, celebration and diagnostics | ACTTUI-009/-010/-012 required; -005/-006/-011 expansion where not already required by their owner |
| [activation-mcp-optional](./activation-mcp-optional.aps.md) | Daemon, durable registration, MCP-optional protection, optional local control app | Existing spine required; ACTMO-021 expansion |
| [daemon-save-time-validation](./daemon-save-time-validation.aps.md) | Headless background save-time driver | Existing v0.9 usefulness cut-line |
| [continuous-improvement-backlog](./continuous-improvement-backlog.aps.md) | Share receipt, quiet repeat start, consent parity, local repeat value | CIB-073/-183/-184/-190 required; CIB-075 expansion |
| [usage-insights](./usage-insights.aps.md) | Local-only value aggregates and evidence semantics | Producer consumed by CIB-073/-190 when trustworthy |
| [dashboard-foundation](./dashboard-foundation.aps.md) | Browser foundation for later continuity | Expansion; non-blocking |

## Work Items

### JOURNEY-001: Repository-specific first win

- **Status:** Merged 2026-07-11 via PRs #3263/#3280 — ACTTUI-009 consent
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

- **Status:** Merged 2026-07-11 via PRs #3263/#3279/#3284 — consent reachable
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

- **Status:** Merged 2026-07-12 via PRs #3283/#3286 — healthy repeat output
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

- **Status:** Merged 2026-07-11 via PR #3282 — cumulative + bounded-window
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

- **Status:** Merged 2026-07-12 — Linux interactive rehearsal complete on
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

- **Status:** Merged 2026-07-12 — the tag landed: `v0.9.0-beta` cut at source
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

- **Status:** Proposed
- **Intent:** Preserve a hands-free, isolated demonstration for clean repos and
  demos without making animation a substitute for real repository value.
- **Expected Outcome:** WOW-006 runs deterministically in a temporary fixture,
  never writes to the user's repository, cleans up safely, and hands control back
  on input.
- **Dependencies:** WOW-006, JOURNEY-001
- **Coordinates with:** WOW-006, ACTTUI-005
- **Validation:** Defined when WOW-006's design gate closes.
- **Confidence:** low

### JOURNEY-008: Celebration and richer diagnostics

- **Status:** Proposed
- **Intent:** Add optional delight and operator depth after the core activation
  path is complete and trustworthy.
- **Expected Outcome:** First success may use the shared celebration treatment;
  typed evidence and diagnostic panes remain available on demand without adding
  noise to healthy repeat use.
- **Dependencies:** JOURNEY-002
- **Coordinates with:** ACTTUI-005, ACTTUI-006, ACTTUI-011
- **Validation:** `cargo test -p eddacraft-anvil-tui activation`.
- **Confidence:** medium

### JOURNEY-009: Always-on confidence indicator

- **Status:** Proposed
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

- **Status:** Proposed
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

## Sequencing

```text
Release cut:
  ACTTUI-009 -> JOURNEY-001 / WOW-005
  ACTTUI-009 -> ACTTUI-010 -> ACTTUI-012 -> JOURNEY-002
  CIB-183 + CIB-190 -> JOURNEY-003
  CIB-073 -> JOURNEY-004
  JOURNEY-001..004 + DSV cut-line -> JOURNEY-005 -> JOURNEY-006

Post-cut expansion:
  JOURNEY-007 || JOURNEY-008 || JOURNEY-009 || JOURNEY-010
```

## Release Gate

JOURNEY-006 is the final conductor gate. Completion of a coordinated module is
necessary evidence, not sufficient evidence: the candidate must pass the two
journeys on the same source SHA that is tagged.
