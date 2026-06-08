---
id: dashboard
title: Dashboards
description:
  Read-only TUI dashboards over your project's persisted anvil state —
  architecture health, drift snapshots, suppressions, and the gate summary.
sidebar_position: 4
---

# Dashboards

| Type        | Authority     | Owner                                                                                                                                               | Status | Freshness                                                                          |
| ----------- | ------------- | --------------------------------------------------------------------------------------------------------------------------------------------------- | ------ | ---------------------------------------------------------------------------------- |
| Public docs | Authoritative | TDASH ([`plans/modules/native-tui-dashboards.aps.md`](https://github.com/eddacraft/anvil-001/blob/main/plans/modules/native-tui-dashboards.aps.md)) | Live   | Last reviewed 2026-06-08 against `main` for the v0.8.0-beta gate-summary dashboard |

| Upstream                                                                                                 | Downstream                                                             |
| -------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------- |
| `anvil dashboard`, persisted `.anvil/` state (architecture, drift snapshots, suppressions, `gates.json`) | Operators and beta testers inspecting project health from the terminal |

`anvil dashboard` opens native, read-only terminal dashboards over the state
anvil has already persisted under `.anvil/`. Nothing is recomputed and nothing
is written — the dashboards render what scanning, drift snapshots, and
suppressions have already recorded, so they are safe to open at any time.

## Opening a dashboard

```bash
# Interactive picker — choose a dashboard from the list
anvil dashboard

# Open a specific dashboard directly
anvil dashboard architecture
anvil dashboard drift
anvil dashboard suppressions
```

Run with no argument to get the picker. Pass a name to jump straight to one
surface.

## The built-in surfaces

### Architecture Health

Layer boundaries, boundary violations, and rule compliance for the project's
architecture definition (`.anvil/architecture.yaml`). Use it to see, at a
glance, which layers currently have crossing edges and how the project tracks
against its declared boundaries.

### Drift Snapshots

Snapshot history and new-edge deltas versus the baseline. Each `anvil drift`
snapshot you capture appears here, so you can read how cross-boundary edges have
moved over time rather than at a single point. See the
[drift trend](insights.md) view in `anvil insights` for the same data as a
sparkline.

### Suppressions

Every active suppression with its scope, file, reason, and expiry. This mirrors
the suppression health view in [`anvil insights --suppressions`](insights.md),
in an interactive surface.

## Gate Summary

Every `anvil gate` run records a snapshot of its result to `.anvil/gates.json` —
pass rate, per-check status, and the checks that need attention. The **Gate
Summary** dashboard renders that snapshot, so you can read the last gate run at
a glance without re-running it.

Unlike the three surfaces above, Gate Summary is a **saved dashboard spec**
rather than a built-in surface: `anvil init` seeds it at
`.anvil/dashboards/gate-summary.dashboard.json`, and anvil's spec-driven
dashboard engine renders it (data binding, charts, and responsive layout from a
declarative JSON spec). Open it from the picker — run `anvil dashboard` and
choose **Gate Summary** from the list, below the built-in dashboards.

Because it is an ordinary saved spec, you can edit its JSON, or drop your own
`*.dashboard.json` files into `.anvil/dashboards/`, to tailor or add dashboards;
they show up in the same picker. The seeded file is preserved across
`anvil init` runs (overwritten only with `anvil init --force`).

## What a dashboard reads

Dashboards are projections of on-disk state, not a fresh scan:

- **Architecture** reads the architecture definition and the most recent
  analysis results.
- **Drift** reads the `anvil drift` snapshot store under `.anvil/`.
- **Suppressions** reads the inline `@anvil-ignore` directives discovered in
  your tree.
- **Gate Summary** reads the `.anvil/gates.json` snapshot written by the last
  `anvil gate` run.

If a dashboard looks empty, it usually means the underlying state has not been
produced yet — run a scan or capture a drift snapshot first. Dashboards require
an interactive terminal; for non-interactive consumers, use the `--json`
surfaces on `anvil insights` and `anvil drift` instead.
