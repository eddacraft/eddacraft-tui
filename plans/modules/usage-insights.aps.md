# Usage Insights

<!-- Executable only if tasks exist and status is Ready or In Progress. -->

| ID       | Owner  | Status | Progress |
| -------- | ------ | ------ | -------- |
| INSIGHTS | @aneki | In Progress | 1/4 |

**Last reviewed:** 2026-05-14 (promoted **Proposed → Ready** alongside
acceptance of
[`plans/specs/2026-05-14-release-plan-v0.7.0-sit-on.md`](../specs/2026-05-14-release-plan-v0.7.0-sit-on.md).
Module-level `In Progress` means Wave 4 work has started. INSIGHTS-001
is load-bearing; the other three extend its data path. All data stays
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

## Tasks

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
- **Expected Outcome:** `anvil insights --suppressions` lists every active
  `@anvil-ignore` and policy-level suppression with: the file/line, the
  suppression date, the rule it suppresses, and a flag indicating whether
  the underlying violation is still present. The list is sorted with stale
  suppressions (where the underlying violation is gone) first, so the user
  can remove dead suppressions.
- **Files:**
  - `crates/anvil-cli/src/commands/insights.rs`
  - `crates/anvil-cli/src/insights/suppressions.rs` (NEW)
  - `crates/anvil-hook/src/suppression_log.rs` (extended if needed)
- **Validation:**
  - `cargo test -p eddacraft-anvil commands::insights::tests::suppression_view_flags_stale`
- **Status:** Draft
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
- **Expected Outcome:** `anvil insights --drift` shows a per-week count of
  new cross-boundary edges over the last 8 weeks as a terminal sparkline,
  with the per-week numeric values listed below. Data is derived from
  baseline diff entries. If fewer than 2 weeks of data exist, the command
  reports that explicitly rather than rendering a misleading line.
- **Files:**
  - `crates/anvil-cli/src/commands/insights.rs`
  - `crates/anvil-cli/src/insights/drift_trend.rs` (NEW)
- **Validation:**
  - `cargo test -p eddacraft-anvil commands::insights::tests::drift_trend_matches_fixture`
  - `cargo test -p eddacraft-anvil commands::insights::tests::insufficient_data_reports_clearly`
- **Status:** Draft
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
- **Status:** Draft
- **Dependencies:** INSIGHTS-001, MLP-001 (install timestamp)
- **changeType:** feature
- **releaseIntent:** candidate
- **releaseScope:** minor
- **releaseNote:**
  - audience: user
  - type: added
  - text: "First-week users now see a single weekly nudge pointing at
    `anvil insights`."

## Sequencing

1. **INSIGHTS-001** is the load-bearing item; everything else extends its
   data path.
2. **INSIGHTS-002**, **INSIGHTS-003**, **INSIGHTS-004** are parallel after
   -001.

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
