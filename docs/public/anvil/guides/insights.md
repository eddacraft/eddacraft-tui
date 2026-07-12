---
id: insights
title: Insights
description:
  Surface anvil's accumulated signal — weekly activity, suppression health, and
  drift trend — from the command line, with JSON output for tooling.
sidebar_position: 5
---

# Insights

| Type        | Authority     | Owner                                                                                                                                                    | Status | Freshness                                                                          |
| ----------- | ------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------- | ------ | ---------------------------------------------------------------------------------- |
| Public docs | Authoritative | INSIGHTS ([`plans/archive/modules/usage-insights.aps.md`](https://github.com/eddacraft/anvil-001/blob/main/plans/archive/modules/usage-insights.aps.md)) | Live   | Last reviewed 2026-06-10 against `main` for the v0.8.0-beta `anvil insights` views |

| Upstream                                                                         | Downstream                                                                    |
| -------------------------------------------------------------------------------- | ----------------------------------------------------------------------------- |
| `anvil insights`, the witness chain, and `anvil drift` snapshots under `.anvil/` | Operators tracking value signal; `--json` consumers (CI, dashboards, scripts) |

`anvil insights` summarises the signal anvil has already collected for your
project. It has four project-health views: a default weekly activity summary
(now followed by a cumulative value scoreboard), a suppression health view, a
drift trend, and a cumulative scoreboard view with a shareable scorecard. Every
view supports `--json` for scripting and dashboards. The current release window
also adds local command-usage views under `anvil kindling usage`.

## Weekly activity (default)

```bash
anvil insights
```

With no flag, `anvil insights` prints a rolling 7-day summary derived from the
witness chain and recorded activity:

- Witness events observed
- Saves observed
- Findings raised
- Suppressions applied and resolved
- Baseline edges added
- Daemon uptime percentage

Add `--json` to emit a schema-versioned `anvil.insights.v1` document for your
own tooling. The default `--json` document is unchanged by the cumulative
scoreboard below; the extended `anvil.insights.v2` document is opt-in via
`--cumulative --json`.

## Suppressing a finding

Not every finding needs fixing right now. Suppressions are appropriate for
legacy code that predates anvil, intentional decisions with a documented reason,
and temporary exceptions with planned work — they are an acknowledgement, not a
fix, so track them and reduce the count over time.

Suppress a finding inline with a comment on the offending line or the line above
it, naming one rule id and a reason after `--`:

```typescript
// @anvil-ignore AP-003 -- legacy parser uses any, migration planned
export function parse(input: any): Record<string, unknown> {
  // ...
}
```

Each directive suppresses exactly one rule. A directive without a `--` reason
still suppresses, but shows up as "No reason provided" in the health view below
— give every suppression a reason so the list stays auditable.

## Suppression health (`--suppressions`)

```bash
anvil insights --suppressions
```

Lists every active inline `@anvil-ignore` suppression in the project. **Stale**
suppressions — ones whose underlying violation no longer fires — are listed
first and marked `STALE`, so you can find and remove suppressions that are no
longer doing anything:

```text
Anvil suppression health
4 @anvil-ignore directive(s): 3 active, 1 stale (underlying violation gone)
[STALE] src/legacy.ts:42  AP-003  (2026-04-01)  temporary while refactoring
[ ok  ] src/api.ts:88     GS-001  (—)           false positive on generated code
...
Remove stale suppressions — their underlying violation no longer fires.
```

`--json` emits the full suppression health document, including the active and
stale counts. This is the same data the [Suppressions dashboard](dashboard.md)
renders interactively.

## Drift trend (`--drift`)

```bash
anvil insights --drift
```

Renders new cross-boundary edges per week over the last 8 weeks as a terminal
sparkline, derived from your `anvil drift` snapshots:

- A week with **no snapshot** reads as no-data, drawn as a gap (`·`) in the
  sparkline — it is _not_ a measured zero.
- The trend needs at least two weeks with snapshots to be meaningful. With less
  history, the command says so explicitly instead of drawing a misleading line.

`--json` emits a schema-versioned `anvil.drift_trend.v1` document with the
per-week buckets, the `weeks_with_data` count, and a `sufficient_data` flag.

The view flags — `--suppressions`, `--drift`, `--cumulative`, and `--share` —
are mutually exclusive: pick one view per invocation.

## Cumulative value scoreboard (`--cumulative`)

```bash
anvil insights --cumulative
```

Answers "what has anvil saved me?" from evidence already recorded locally:

- **Witness events** — since the first recorded event (the witness chain is
  append-only, so this is a genuine all-time count), plus 30- and 90-day windows
  anchored to the chain's own latest event.
- **Save-time protection** — checks observed, risky writes flagged, writes
  blocked, secret findings caught, and protective fences engaged, counted over
  the local observation store's **retained window** (it has bounded 7-day
  retention, so these are never presented as all-time claims). The window's
  bounds are named in every render.

Every number is a pure function of the recorded data — no wall clock is
consulted, and streams with no evidence say so honestly instead of showing
measured-looking zeros. The scoreboard also appears after the default weekly
summary.

`--cumulative --json` emits the extended `anvil.insights.v2` document: every
`anvil.insights.v1` field (still rolling/wall-clock-anchored) plus a
deterministic `cumulative` object.

## Shareable scorecard (`--share`)

```bash
anvil insights --share
anvil insights --share --output ~/reports/anvil-scorecard.html
```

Writes a self-contained single-file HTML scorecard of the headline numbers and
prints the plain-text summary. The card is safe to share by design:

- **Redacted by default** — counts, durations, and evidence-window dates only.
  No repository paths, repo or file names, branch names, secret values,
  hostnames, usernames, or emails ever appear in the card.
- **Self-contained and offline** — embedded styling, no scripts, no external
  assets, no network references.
- **Deterministic** — identical recorded data produces a byte-identical card;
  the dates shown are the evidence window's own bounds, not a generation
  timestamp.

Without `--output`, the card is created as `anvil-scorecard.html` in the current
directory and an existing file at that name is never overwritten — pass
`--output <path>` to choose (and overwrite) an explicit destination. When there
is no recorded evidence yet, `--share` says so and writes nothing.

## Local command usage (`anvil kindling usage`)

```bash
anvil kindling usage
```

`anvil kindling usage` reads local command-invocation observations from the
Kindling store. It is on-device only: command names and active feature-flag
names are recorded, but argument values are not, and no telemetry is sent. Use
it to answer questions like which anvil commands are being exercised and which
feature flags were active during those runs.

Operator controls:

- `ANVIL_USAGE_DISABLE=1` or `DO_NOT_TRACK=1` disables the CLI `command.invoked`
  producer.
- `ANVIL_INTERCEPT_DISABLE_OBSERVATION=1` disables both CLI and daemon usage
  producers.
- `ANVIL_USAGE_SIDECAR_NO_TRIM=1` disables the lazy 7-day / 64 MiB sidecar trim.

## The first-week nudge

For a newly adopted project, `anvil status` shows a single muted line pointing
you at `anvil insights` during roughly the first two weeks after adoption — a
low-noise reminder that the signal is accumulating and worth a look. It is
local-only, never blocks, and stops appearing once you have run `anvil insights`
(or once the window passes). It is a nudge toward the views above, not a
separate surface.

## Capturing the data

The drift trend is only as rich as your snapshot history. Capture snapshots
regularly (for example in CI or a scheduled job) with `anvil drift` so the
weekly buckets fill in. Suppression and weekly-activity views read live state
and need no setup.
