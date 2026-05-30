---
id: dashboard
title: Dashboards
description:
  Read-only TUI dashboards over your project's persisted anvil state —
  architecture health, drift snapshots, and suppressions.
sidebar_position: 4
---

# Dashboards

| Type        | Authority     | Owner                                                                                                                                               | Status | Freshness                                                                             |
| ----------- | ------------- | --------------------------------------------------------------------------------------------------------------------------------------------------- | ------ | ------------------------------------------------------------------------------------- |
| Public docs | Authoritative | TDASH ([`plans/modules/native-tui-dashboards.aps.md`](https://github.com/eddacraft/anvil-001/blob/main/plans/modules/native-tui-dashboards.aps.md)) | Live   | Last reviewed 2026-05-31 against `main` for the v0.7.3-beta `anvil dashboard` surface |

| Upstream                                                                                   | Downstream                                                             |
| ------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------- |
| `anvil dashboard`, persisted `.anvil/` state (architecture, drift snapshots, suppressions) | Operators and beta testers inspecting project health from the terminal |

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

## The three surfaces

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

## What a dashboard reads

Dashboards are projections of on-disk state, not a fresh scan:

- **Architecture** reads the architecture definition and the most recent
  analysis results.
- **Drift** reads the `anvil drift` snapshot store under `.anvil/`.
- **Suppressions** reads the inline `@anvil-ignore` directives discovered in
  your tree.

If a dashboard looks empty, it usually means the underlying state has not been
produced yet — run a scan or capture a drift snapshot first. Dashboards require
an interactive terminal; for non-interactive consumers, use the `--json`
surfaces on `anvil insights` and `anvil drift` instead.
