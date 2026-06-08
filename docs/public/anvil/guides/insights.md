---
id: insights
title: Insights
description:
  Surface anvil's accumulated signal — weekly activity, suppression health, and
  drift trend — from the command line, with JSON output for tooling.
sidebar_position: 5
---

# Insights

| Type        | Authority     | Owner                                                                                                                                    | Status | Freshness                                                                          |
| ----------- | ------------- | ---------------------------------------------------------------------------------------------------------------------------------------- | ------ | ---------------------------------------------------------------------------------- |
| Public docs | Authoritative | INSIGHTS ([`plans/modules/usage-insights.aps.md`](https://github.com/eddacraft/anvil-001/blob/main/plans/modules/usage-insights.aps.md)) | Live   | Last reviewed 2026-06-08 against `main` for the v0.8.0-beta `anvil insights` views |

| Upstream                                                                         | Downstream                                                                    |
| -------------------------------------------------------------------------------- | ----------------------------------------------------------------------------- |
| `anvil insights`, the witness chain, and `anvil drift` snapshots under `.anvil/` | Operators tracking value signal; `--json` consumers (CI, dashboards, scripts) |

`anvil insights` summarises the signal anvil has already collected for your
project. It has three views: a default weekly activity summary, a suppression
health view, and a drift trend. Every view supports `--json` for scripting and
dashboards.

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
own tooling.

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

`--suppressions` and `--drift` are mutually exclusive — pick one view per
invocation.

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
