---
id: drift
title: Drift tutorial
description: Capture two architecture snapshots and compare them.
owner: DOCSYNC
upstream:
  - crates/anvil-cli/src/commands/drift.rs
verified_against: 0.9.0-beta
---

# Drift tutorial

**For:** projects with validated architecture boundaries

**Time:** 15 minutes plus the time between snapshots

**Outcome:** compare current dependency evidence with an earlier snapshot

## 1. Capture the starting point

```text
anvil drift snapshot
```

Success means anvil records a snapshot and identifies it.

## 2. Make normal changes

Continue development. A useful comparison needs a real later state; do not
manufacture drift in a protected branch.

## 3. Capture again

```text
anvil drift snapshot
anvil drift list
```

## 4. Compare

Use the identifiers shown by `anvil drift list`:

```text
anvil drift compare --help
```

Then run the exact comparison syntax for your installed version. For the latest
two snapshots, try:

```text
anvil drift report
```

## Interpret the result

A changed edge is not automatically a defect. Decide whether it reflects an
intentional architecture change, an incorrect definition, or accidental drift.

## Next step

Review architecture and drift in the terminal with
`anvil dashboard architecture` and `anvil dashboard drift`, or the
[dashboard guide](../guides/dashboard.md).
