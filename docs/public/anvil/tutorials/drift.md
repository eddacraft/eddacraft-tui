---
id: drift
title: Drift Detection
sidebar_position: 4
---

# Drift Detection

Architecture drift is the gradual divergence between your intended design and
what the code actually does. anvil tracks drift by comparing dependency
snapshots over time.

## Prerequisites

- anvil initialised with architecture boundaries configured
- Architecture boundaries configured (`.anvil/architecture.yaml`) and at least
  one successful `anvil gate --only-checks import-boundaries` run

The drift commands are identical on macOS, Linux, and Windows. On Windows,
PowerShell displays snapshot paths with backslashes on disk, but the snapshot
names and `.anvilrc` values stay the same.

## 1. Capture a Baseline Snapshot

macOS / Linux:

```bash
anvil drift snapshot --name baseline
```

Windows PowerShell:

```powershell
anvil drift snapshot --name baseline
```

```
Capturing snapshot: baseline
  Modules:  4
  Edges:    12
  Files:    28

Snapshot saved to .anvil/snapshots/snapshot-baseline.json
```

A snapshot records every module and every dependency edge at a point in time.

## 2. List Available Snapshots

macOS / Linux:

```bash
anvil drift list
```

Windows PowerShell:

```powershell
anvil drift list
```

```
Snapshots:
  baseline    2026-04-01  4 modules, 12 edges
```

## 3. Make Changes

Work on your codebase as normal. Over days or weeks, new imports get added,
modules grow, and the dependency graph shifts.

For this tutorial, suppose you add a utility that the API layer imports directly
from the data layer -- a boundary violation that was suppressed because of a
deadline.

## 4. Capture a New Snapshot

macOS / Linux:

```bash
anvil drift snapshot --name after-changes
```

Windows PowerShell:

```powershell
anvil drift snapshot --name after-changes
```

```
Capturing snapshot: after-changes
  Modules:  4
  Edges:    14
  Files:    31

Snapshot saved to .anvil/snapshots/snapshot-after-changes.json
```

## 5. Compare Snapshots

macOS / Linux:

```bash
anvil drift compare baseline after-changes
```

Windows PowerShell:

```powershell
anvil drift compare baseline after-changes
```

```
Drift Report: baseline -> after-changes

New edges:
  + presentation -> data      (2 imports)   [VIOLATION]
  + data -> business          (1 import)    [VIOLATION]

Removed edges:
  - business -> shared        (1 import removed, 2 remaining)

Summary:
  New edges:      +3
  Removed edges:  -1
  Net drift:      +2
  Violations:     2
```

The report highlights exactly which new dependency edges appeared and whether
they violate your architecture rules.

## 6. Set a Drift Budget

Add a drift budget to `.anvilrc` to fail CI when drift exceeds a threshold:

```json
{
  "drift": {
    "budget": {
      "maxNewEdges": 5,
      "maxViolations": 0
    },
    "baselineSnapshot": "baseline"
  }
}
```

| Option             | Description                                          |
| ------------------ | ---------------------------------------------------- |
| `maxNewEdges`      | Maximum allowed new dependency edges before failing  |
| `maxViolations`    | Maximum allowed new boundary violations (0 = strict) |
| `baselineSnapshot` | Name of the snapshot to compare against              |

With `maxViolations: 0`, any new boundary-crossing edge fails the check:

macOS / Linux:

```bash
anvil check --all
```

Windows PowerShell:

```powershell
anvil check --all
```

```
Checking drift...
  2 new violations since baseline (budget: 0)

Drift budget exceeded.
```

:::info Update the baseline periodically as your architecture evolves. Named
snapshots cannot be silently overwritten — choose a new name or remove the old
`snapshot-<name>.json` file under `.anvil/snapshots/` before recapturing. :::

## Workflow Tips

- **Capture snapshots at release boundaries** -- tag them with version numbers
- **Review drift reports in PRs** -- the CI integration can post drift summaries
  as PR comments
- **Use drift budgets as guardrails** -- allow some flexibility for active
  development, tighten for stable modules

## See Drift on the Dashboards

Drift findings feed the same surfaces as every other check family:

- the [dashboard guide](../guides/dashboard.md) shows gate and check results --
  including drift -- in the terminal dashboard picker
- the [insights guide](../guides/insights.md) tracks how findings move over
  time, which is drift's question asked weekly

---

**Next:** [Custom Policies](policies.md)
