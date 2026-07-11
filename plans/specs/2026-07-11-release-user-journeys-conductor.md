# Release User Journeys Conductor Design

| Type | Authority | Owner | Status | Freshness |
| ---- | --------- | ----- | ------ | --------- |
| Design | Authoritative | JOURNEY | Accepted | Approved by the operator 2026-07-11 after review of the current `anvil welcome` and `anvil start` journeys |

| Upstream | Downstream |
| -------- | ---------- |
| [`first-run-wow`](../modules/first-run-wow.aps.md), [`activation-tui`](../modules/activation-tui.aps.md), [`activation-mcp-optional`](../modules/activation-mcp-optional.aps.md), [`continuous-improvement-backlog`](../modules/continuous-improvement-backlog.aps.md), [`usage-insights`](../modules/usage-insights.aps.md), [`dashboard-foundation`](../modules/dashboard-foundation.aps.md) | [`release-user-journeys`](../modules/release-user-journeys.aps.md), [`RELEASE-PLAN`](../../RELEASE-PLAN.md) |

## Problem

The current release has strong individual surfaces but no single owner for the
two product outcomes that determine whether it is genuinely useful:

1. A new user runs `anvil welcome`, sees Anvil understand their repository,
   experiences a concrete first win, and has something worth telling another
   developer about.
2. A time-poor senior developer runs `anvil start`, reaches honest protection
   without learning Anvil's internals, and trusts the fast repeat experience
   enough to keep Anvil running every day.

WOW owns the tutorial narrative, ACTTUI owns activation presentation, ACTMO and
DSV own the background protection spine, CIB owns bounded friction fixes,
INSIGHTS owns local value evidence, and DASH owns the browser surface. None of
those verticals should become a shadow release plan. A conductor is needed to
define the outcome, order the work, and hold the release gate.

## Product Outcome

The release is journey-ready when:

- a supported repository produces a truthful, repository-specific value moment
  within 60 seconds of `anvil welcome`;
- the first-win path leads with the highest-value real finding, explains why it
  matters, and shows a proposed change before any explicitly consented write;
- a clean repository gets an honestly labelled sandbox demonstration or a clear
  clean result, never fabricated findings presented as local truth;
- `anvil start` completes activation without documentation, hidden consent, or
  a dead-end interactive phase;
- a healthy repeat `anvil start` is terse, fast, and answers both "am I
  protected?" and, when reliable local evidence exists, "what has Anvil done?";
- a user can produce a leak-safe, self-contained value receipt without exposing
  repository internals, secret values, or personal data;
- Linux, macOS, and Windows release candidates pass fresh-install, activation,
  restart/reboot, repeat-run, degraded-mode, and recovery rehearsals;
- failures offer one clear recovery action and never over-claim protection.

## Decisions

### D1 — One conductor, existing vertical owners

Create `JOURNEY` as a conductor. It owns sequencing, cut criteria, acceptance
evidence, and release reconciliation. Implementation remains in WOW, ACTTUI,
ACTMO/DSV, CIB, INSIGHTS, and DASH.

No new vertical module is needed now:

- WOW-005 already owns the repository-specific first win.
- WOW-006 already owns the sandbox autoplay demonstration.
- CIB-073 already owns cumulative and shareable value evidence.
- CIB-183/-184 already own quiet repeat output and explicit MCP consent.
- ACTTUI-005/-006/-011 already own celebration and richer diagnostics.
- ACTMO-021 and CIB-075 already own optional always-on visibility.
- DASH already owns the browser expansion.

The one uncovered behaviour — an optional local value line on a healthy repeat
`anvil start` — is small and cross-surface, so it is added to CIB rather than
given a new module.

### D2 — Release cut and expansion are explicit

The conductor has two lanes:

- **Release cut:** WOW-005; ACTTUI-009/-010/-012; CIB-183/-184; the repeat-start
  value receipt; CIB-073's leak-safe share contract; cross-platform journey
  rehearsal; final outcome evidence.
- **Post-cut expansion:** WOW-006; ACTTUI celebration and richer diagnostics;
  optional always-on indicator; broader DASH continuity.

Expansion stays visible in the same product narrative but does not silently
block the release. Any promotion into the cut requires an explicit operator
decision and a conductor update.

### D3 — Repository truth creates the first wow

The first-win path uses discovery results already produced by `anvil welcome`.
It selects the highest-severity actionable real finding with deterministic
tie-breaking, explains the consequence in plain language, and offers inspection
before action. A proposed diff is shown before a write; applying it requires an
explicit unticked consent action and uses ACTTUI's shared consent posture.

If no suitable real finding exists, the journey states that clearly. The
sandbox demonstration may be offered as an honestly separate example and must
not write to the user's repository.

### D4 — Repeat start is confidence, not onboarding

A healthy repeat `anvil start` collapses to the protection state, daemon and
worktree posture, and at most one next action. If a bounded local aggregate can
be read without delaying activation or weakening truthfulness, one value line
may report recent saves checked or findings caught. Missing, stale, ambiguous,
or zero-filled evidence is omitted rather than converted into a claim.

Repair states retain the detail needed to recover. Machine-readable and
non-interactive contracts remain deterministic.

### D5 — Shareable evidence is privacy-first

CIB-073 remains the owning work item for cumulative and shareable value. The
first shareable format must be deterministic and self-contained, redact paths
and repository internals by default, never contain secret values, and state the
time window and evidence source. Advocacy is an outcome of credible evidence,
not a reason to weaken the trust boundary.

### D6 — Journey rehearsal is a release gate

Automated suites prove contracts, but the release also needs candidate-binary
journey evidence. The conductor owns a release rehearsal across Linux, macOS,
and Windows covering:

- fresh supported repository and clean repository;
- signed-out `welcome` followed by authenticated `start`;
- TTY, compact, `--verify`, and `--json` paths;
- daemon start/reuse and durable worktree registration after restart/reboot;
- MCP-present, MCP-absent, explicit no-MCP, and restart-required states;
- healthy repeat start and one representative repair path;
- terminal teardown, bounded runtime, and one-next-action output.

Platform-specific release infrastructure may supply the evidence; JOURNEY owns
the acceptance record and refuses the cut when required evidence is absent.

## Success Measures

Release evidence records:

- time from command start to the first repository-specific value statement;
- whether the user reaches a real finding, clean result, or labelled sandbox;
- whether activation reaches an honest terminal state without documentation;
- repeat-start duration and output size on the healthy path;
- whether every failure ending has exactly one recovery owner;
- whether shareable output passes privacy/redaction fixtures;
- pass/fail by platform and journey variant.

These are release-test measurements, not permission to add telemetry.

## Non-Goals

- Making the browser dashboard a prerequisite for terminal usefulness.
- Adding network telemetry or cross-machine aggregation.
- Treating animation, banners, or autoplay as substitutes for repository-specific
  evidence.
- Claiming a clean repository is risk-free.
- Moving implementation authority into the conductor.

## Approval

The operator approved the conductor-plus-existing-owners approach, the
`JOURNEY` / `release-user-journeys` naming, and the supporting APS and release
plan edits on 2026-07-11.
