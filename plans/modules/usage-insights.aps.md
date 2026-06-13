# Usage Insights

<!-- Executable only if tasks exist and status is Ready or In Progress. -->

| ID       | Owner  | Status | Progress |
| -------- | ------ | ------ | -------- |
| INSIGHTS | @aneki | In Progress | 4/5 |

**Last reviewed:** 2026-06-10 (INSIGHTS-005 filed from the v0.8.0-beta
user-journey completeness review — extend the INSIGHTS-004 first-week nudge
to the `anvil welcome` closing output, the one command every new user runs.
Previously 2026-05-14: promoted **Proposed → Ready** alongside acceptance of
[`plans/specs/2026-05-14-release-plan-v0.7.0-sit-on.md`](../specs/2026-05-14-release-plan-v0.7.0-sit-on.md).
Module-level `In Progress` means Wave 4 work has started. INSIGHTS-001
is load-bearing; the others extend its data path. All data stays
local-only — no telemetry on this module.)

## Purpose

When Anvil is working correctly, most days nothing visible happens. The silent
middle is the intended state — warnings over blocks, exit 0 by default, no
noise without a real signal. But silent middle and "doing nothing" are
indistinguishable from outside. Users who do not see periodic evidence of
value question the value.

This module gives the user **periodic, local-only, low-noise** visibility into
what Anvil actually did. The goal is not analytics. The goal is the once-a-
week glance that answers: *"is this earning its keep?"*

The success criterion this module supports is `<10% of warnings are suppressed
without resolution` from the index Success Criteria. INSIGHTS makes that
number visible to the user, not just to a future post-release survey.

## In Scope

- `anvil insights` command: weekly summary of recent activity
- Local-only data: counts of saves watched, findings raised, suppressions
  applied, baseline edges added, daemon uptime
- Suppression health view: which suppressions exist, when they were added,
  whether the underlying violation is still present
- Drift trend: new cross-boundary edges per week with a simple sparkline
- All data lives in `.anvil/` (project-local); no network call, no telemetry,
  no aggregation across machines

## Out of Scope

- Cross-machine aggregation (a future cloud module)
- Anonymous opt-in telemetry to the Anvil project (deliberately deferred until
  the local-only surface is settled)
- Team-lead browser surface (Horizon 2)
- Real-time dashboards or streaming
- Replacement for `anvil status` (status is now; insights is over time)

## Interfaces

- **Depends on:**
  - `crates/anvil-witness/*` (witness lines as the canonical event source)
  - `crates/anvil-baseline/*` (baseline-add events and drift counters)
  - `crates/anvil-hook/src/suppression.rs` (current in-memory suppression surface)
  - `crates/anvil-cli/src/commands/`
- **Exposes:**
  - `anvil insights` top-level command with `--week`, `--suppressions`,
    `--drift` subviews
  - `.anvil/insights/` cache directory containing aggregated weekly rollups
    (regenerable from witness chain)

## Work Items

### INSIGHTS-001: `anvil insights` Weekly Summary

- **Intent:** Provide a single command that summarises Anvil activity over the
  last 7 days at a glance.
- **Expected Outcome:** `anvil insights` defaults to the last 7-day window and
  reports: total saves observed, findings raised, suppressions applied,
  suppressions resolved, baseline edges added, daemon uptime percentage. Output
  is plain text and fits a single screen. `--json` emits a schema-versioned
  document for editor/CI consumption. Data is derived from the witness chain
  and suppression log; no separate event store.
- **Files:**
  - `crates/anvil-cli/src/commands/insights.rs` (NEW)
  - `crates/anvil-cli/src/insights/aggregator.rs` (NEW)
  - `schemas/anvil-insights.v1.json` (NEW)
- **Validation:**
  - `cargo test -p eddacraft-anvil commands::insights::tests::weekly_summary_matches_fixture`
  - `cargo test -p eddacraft-anvil commands::insights::tests::derives_from_witness_chain`
- **Status:** Released/Shipped via v0.7.0-beta (d7873161 · 2026-05-21)
- **Done:** 2026-05-17 — `anvil insights` and `--json` emit the pinned
  `anvil.insights.v1` weekly summary from the witness chain. The v1 schema
  reports `witness_events_observed` separately so hook/commit evidence is not
  mislabelled as save-time observations. Counters without durable event sources
  in the current codebase are present and zero-filled for v1; INSIGHTS-002/-003/-004
  extend the data path.
- **changeType:** feature
- **releaseIntent:** candidate
- **releaseScope:** minor
- **releaseNote:**
  - audience: user
  - type: added
  - text: "`anvil insights` reports last-week activity at a glance."

### INSIGHTS-002: Suppression Health View

- **Intent:** Surface every active suppression with provenance so the user
  can review whether suppressions are still warranted.
- **Spec reconciliation (2026-05-26):** APS truth-validation before
  implementation found the data-source premise was wrong and the scope
  over-broad; corrected contract below:
  - There is **no suppression log** — `crates/anvil-hook/src/suppression_log.rs`
    does not exist and was never created. INSIGHTS-001 zero-fills its
    suppression counters for exactly this reason. The data is derived from a
    **live scan**, mirroring `services/sample_analyser.rs`: walk the workspace,
    run `anvil_checks::antipattern::run_antipattern_check`, and read
    `Warning.suppressed: Option<Suppression>`. (Same class of fix as
    DISTRIB-005 — the spec named a storage surface that doesn't exist.)
  - **Scope narrowed to inline `@anvil-ignore` directives** (the authoritative
    suppression form per ADR-029, parsed by
    `anvil_checks::antipattern::parse_suppression`). "Policy-level
    suppression" (config `RuleModes` rule-disabling) is a distinct concept —
    no per-site file/line, no date, no staleness — and does not fit the
    health/staleness model; tracked as a separate follow-up, not in -002.
  - **Provenance is partial:** `Suppression { reason, author: Option,
    timestamp: Option, scope }`. Inline `@anvil-ignore <ID> -- <reason>`
    directives populate `reason`/`scope` but leave `author`/`timestamp`
    `None`, so the "suppression date" renders as `—` when absent rather than
    being guaranteed.
  - **Staleness method:** a `suppressed: Some(..)` warning means the finding
    fired then got suppressed → underlying violation **present**. A directive
    found by sweeping source (`parse_suppression`) with **no** matching
    suppressed warning nearby → **stale** (violation gone). Sort stale-first.
- **Expected Outcome:** `anvil insights --suppressions` lists every active
  inline `@anvil-ignore` suppression with: the file/line, the rule it
  suppresses, the reason, the suppression date (when recorded), and a flag for
  whether the underlying violation is still present. Stale suppressions (the
  underlying violation is gone) sort first so the user can remove dead
  suppressions. `--json` emits a schema-versioned `anvil.suppressions.v1`
  document.
- **Files:**
  - `crates/anvil-cli/src/commands/insights.rs` (MODIFY — `--suppressions`
    flag + render)
  - `crates/anvil-cli/src/insights/suppressions.rs` (NEW — live-scan health)
  - `crates/anvil-cli/src/insights/mod.rs` (MODIFY — module decl)
  - ~~`crates/anvil-hook/src/suppression_log.rs`~~ — dropped; no log exists,
    data is derived from the live antipattern scan.
- **Validation:**
  - `cargo test -p eddacraft-anvil insights::suppressions::tests`
  - `cargo test -p eddacraft-anvil commands::insights::tests`
- **Status:** Released/Shipped via v0.7.3-beta (tag `8bfd48c4d` · 2026-05-31;
  Merged 2026-05-27 via PR
  [#1996](https://github.com/eddacraft/anvil-001/pull/1996)) — `anvil insights
  --suppressions` lists inline `@anvil-ignore` directives (stale-first) from a
  live antipattern scan; directive-primary `classify` (Council + Copilot
  review).
- **Dependencies:** INSIGHTS-001
- **changeType:** feature
- **releaseIntent:** candidate
- **releaseScope:** minor
- **releaseNote:**
  - audience: user
  - type: added
  - text: "`anvil insights --suppressions` lists active suppressions and
    flags ones whose underlying violation is gone."

### INSIGHTS-003: Drift Trend Sparkline

- **Intent:** Show the user the actual drift signal Anvil exists to slow —
  new cross-boundary edges per week — as a simple visible trend.
- **Spec reconciliation (2026-05-29):** APS truth-validation before
  implementation found the data-source premise was wrong (same class of fix
  as INSIGHTS-002 / DISTRIB-005 — the spec named a data path that does not
  exist); corrected contract below:
  - **"baseline diff entries" carry no per-week edge history.**
    `BaselineDiff.added` (`crates/anvil-baseline/src/store.rs`) is computed
    in-memory on demand and never persisted; `anvil/baseline.json` is a
    single snapshot atomically overwritten on every refresh, with no
    per-finding timestamps; the witness chain records a per-commit `ts` but
    carries no edge/finding payload, so it cannot be replayed into a weekly
    edge count. INSIGHTS-001 zero-fills `baseline_edges_added` for exactly
    this reason. An 8-week trend cannot be backfilled from any of these.
  - **The real durable source is the existing `anvil drift` snapshot store.**
    `anvil drift snapshot` writes timestamped
    `.anvil/snapshots/snapshot-*.json`
    (`DriftSnapshot { created_at, metrics.boundary_violations, violations[] }`),
    where each `SnapshotViolation { from_layer, to_layer, from_file, to_file,
    id }` is a cross-boundary edge. `commands::drift::compare_snapshots`
    already diffs violations by stable `id` to count added/removed; -003
    reuses that identity to count **new** edges week-over-week. No new
    on-disk format is introduced.
  - **Weekly bucketing + new-edge attribution:** snapshots are ordered
    oldest→newest; for each adjacent pair the added-edge count
    (`ids(curr) − ids(prev)`, by violation `id`, set-deduped on both sides)
    is attributed to the calendar week of the later snapshot and summed
    within each of the 8 trailing weekly buckets. The snapshot immediately
    preceding the window seeds the first in-window week's baseline when
    present. The metric is **edge introductions per week**, not a net
    end-of-week delta: an edge added then resolved within the same week
    still counts once (it was new drift that week), and an edge that
    reappears after being removed in an earlier week counts again. This
    keeps the signal honest about churn the user is trying to slow.
  - **Sporadic-snapshot reality:** snapshots are operator-triggered, so a
    week with no snapshot has *no data* (rendered distinctly from a measured
    zero). "Fewer than 2 weeks of data" is made concrete as **fewer than 2
    of the 8 trailing weeks containing at least one snapshot** → the command
    prints an explicit insufficient-data message instead of a misleading
    line.
- **Expected Outcome:** `anvil insights --drift` shows a per-week count of
  new cross-boundary edges over the last 8 weeks as a terminal sparkline,
  with the per-week numeric values listed below (weeks without a snapshot
  marked as no-data, not zero). Data is derived from the existing
  `.anvil/snapshots/` drift-snapshot store. When fewer than 2 of the 8
  trailing weeks contain a snapshot, the command reports that explicitly
  rather than rendering a misleading line. `--json` emits a schema-versioned
  `anvil.drift_trend.v1` document.
- **Files:**
  - `crates/anvil-cli/src/commands/insights.rs` (MODIFY — `--drift` flag +
    render)
  - `crates/anvil-cli/src/insights/drift_trend.rs` (NEW — snapshot-derived
    weekly trend)
  - `crates/anvil-cli/src/insights/mod.rs` (MODIFY — module decl)
- **Validation:**
  - `cargo test -p eddacraft-anvil commands::insights::tests::drift_trend_matches_fixture`
  - `cargo test -p eddacraft-anvil commands::insights::tests::insufficient_data_reports_clearly`
- **Status:** Released/Shipped via v0.7.3-beta (tag `8bfd48c4d` · 2026-05-31;
  Merged 2026-05-29 via PR [#2111](https://github.com/eddacraft/anvil-001/pull/2111))
  — `anvil insights --drift` renders the per-week new-cross-boundary-edge sparkline from the
  existing `.anvil/snapshots/` drift store (merge commit `4c3ea6b10`).
- **Dependencies:** INSIGHTS-001
- **changeType:** feature
- **releaseIntent:** candidate
- **releaseScope:** minor
- **releaseNote:**
  - audience: user
  - type: added
  - text: "`anvil insights --drift` shows new cross-boundary edges per
    week as a sparkline."

### INSIGHTS-004: First-Week Adoption Signal Hint

- **Intent:** During a user's first week with Anvil, surface a one-line
  insights hint after first-run plus a gentle reminder once at the end of
  the week, so the silent middle is not literally silent for the cohort
  most likely to bounce.
- **Spec reconciliation (2026-06-02):** APS truth-validation before
  implementation found the data-source premise was stale (same class of
  fix as INSIGHTS-002/003 and DISTRIB-005 — the spec named a file that
  does not exist in that form):
  - Detection uses the `created_at` field from `anvil/project-id` (plain
    text key: value file, not `.anvil/project-id.json`). The file lives
    at `anvil/project-id` relative to workspace root per
    `crates/anvil-cli/src/activation/identity.rs` (PROJECT_ID_PATH,
    ensure_project_id, new_fresh which sets created_at, parse). `.anvil/`
    is for cache/state (first-run marker, baseline.json, cache/,
    policies/); `anvil/` holds the tracked project identity (and witness).
  - (The Expected Outcome text below was written against the pre-MLP2-003
    assumption and will be honoured in implementation using the actual
    source.)
- **Expected Outcome:** `anvil status` and watch surface a one-line
  "Anvil watched N saves this week (run `anvil insights`)" hint exactly
  once per week, only during the first 14 days after install. Detection
  uses the install timestamp recorded in `.anvil/project-id.json` from
  MLP-001. Hint is suppressed if the user has already run `anvil insights`
  in the current week.
- **Files:**
  - `crates/anvil-cli/src/commands/status.rs`
  - `crates/anvil-tui/src/surfaces/watch/render.rs`
  - `crates/anvil-cli/src/insights/first_week_hint.rs` (NEW)
- **Validation:**
  - `cargo test -p eddacraft-anvil commands::status::tests::first_week_hint_shown_once`
  - `cargo test -p eddacraft-anvil commands::status::tests::hint_suppressed_after_use`
- **Status:** Released/Shipped via v0.8.0-beta (2026-06-11). Merged 2026-06-02
  via PR #2226 (merge commit `231a0e46f` confirmed in tag; status recorded
  retroactively 2026-06-12 — the post-merge flip was missed at land time).
  Earlier history: In Progress (promoted 2026-06-02 — both dependencies satisfied:
  INSIGHTS-001 Released/Shipped, MLP-001 Done; the prior Draft was a v0.7.0
  tag-time scope-cut, not a technical block. Per this module's Sequencing,
  -002/-003/-004 are parallel after -001, and -002/-003 already shipped.
  Daemon-independent 0.8.0 freight.) Implementation started 2026-06-02 via dev-workflow on feat/insights-004.
  **Implementation complete (pre-PR):** `anvil insights/first_week_hint.rs` + wires into status (plain + TUI), watch (data + render + footer), insights command (record viewed). Uses correct `anvil/project-id` `created_at`. Tests (internal + the two APS `commands::status::tests::first_week_hint_*`) green. `cargo test -p eddacraft-anvil --bin anvil first_week_hint*` + `hint_suppressed` pass. Full lint (clippy+fmt) + format green post-fix. Module progress 3/4.
  **PR #2226** created; per finishing-a-branch + addressing-pr-reviews loop to follow. Mark **Merged** on land (then cleanup agent to Released/Shipped).
- **Dependencies:** INSIGHTS-001, MLP-001 (install timestamp)
- **changeType:** feature
- **releaseIntent:** candidate
- **releaseScope:** minor
- **releaseNote:**
  - audience: user
  - type: added
  - text: "First-week users now see a single weekly nudge pointing at
    `anvil insights`."

### INSIGHTS-005: First-Week Nudge on the Welcome Surface

- **Status:** Ready
- **Intent:** The first-week nudge reaches the one command every new user
  definitely runs — `anvil welcome` — not only `status` and watch.
- **Expected Outcome:** `anvil welcome` closing output includes the
  first-week insights hint under the same contract as INSIGHTS-004 (14-day
  window from `anvil/project-id` `created_at`, once per week, suppressed
  after `anvil insights`, shared `.anvil/insights-hint.json` state — no new
  state file or rate-limit bucket).
- **Files:**
  - `crates/anvil-cli/src/commands/welcome.rs`
  - `crates/anvil-cli/src/insights/first_week_hint.rs`
- **Validation:**
  - `cargo test -p eddacraft-anvil` first-week-hint welcome surface tests
    (shown within window, suppressed after `anvil insights`, rate-limit
    shared with status/watch)
- **Dependencies:** INSIGHTS-004 (hint mechanism; PR #2226)
- **Identified From:** 2026-06-10 v0.8.0-beta user-journey completeness
  review; coordinates with [`UJ-001`](../archive/modules/user-journey.aps.md) (welcome
  closing-output threading).
- **Confidence:** high
- **changeType:** feature
- **releaseIntent:** candidate
- **releaseScope:** patch
- **releaseNote:**
  - audience: user
  - type: changed
  - text: "The first-week insights nudge now also appears after
    `anvil welcome`."

## Sequencing

1. **INSIGHTS-001** is the load-bearing item; everything else extends its
   data path.
2. **INSIGHTS-002**, **INSIGHTS-003**, **INSIGHTS-004** are parallel after
   -001.
3. **INSIGHTS-005** follows -004 (same hint mechanism, new surface).

## Release Notes

INSIGHTS items collectively justify a "Anvil now gives you a weekly view of
what it caught and what's drifting" line in `v0.7.0-beta`.

## Cross-References

- Coordinates with: [`MLP-002`](../archive/modules/multilayer-protection.aps.md) (witness chain
  as canonical event source), [`MLP-007`](../archive/modules/multilayer-protection.aps.md)
  (baseline diff for drift), [`MLP-003`](../archive/modules/multilayer-protection.aps.md)
  (suppression log for suppression view), [`ADTRUST-001`](../archive/modules/adoption-trust-surface.aps.md)
  (status surface where first-week hint renders).
- Blocks on: none at module level; INSIGHTS-002 / -003 inherit MLP data
  contracts already in place.
