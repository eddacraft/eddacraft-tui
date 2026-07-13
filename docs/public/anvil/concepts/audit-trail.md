---
id: audit-trail
title: Audit Trail
description:
  Verify Anvil witness records and portable review capsules with the current
  Rust CLI.
sidebar_position: 3
---

# Audit Trail

Anvil's current audit trail has two concrete layers:

1. a **witness chain** records governance events alongside the repository; and
2. a **review capsule** packages the evidence for one commit range so another
   machine can inspect and verify it.

The Rust CLI uses purpose-built witness and capsule commands rather than a
generic evidence command. The commands below are the public, shipped surfaces.

## Witness chain

Anvil-managed Git hooks append witness records under `anvil/witness/`. The
records bind governance activity to commits so an operator can detect commits
that bypassed the normal path.

Audit the branch ending at `HEAD`:

```bash
anvil audit-chain
```

Bound the walk when you only need a release or pull-request range:

```bash
anvil audit-chain --since v0.9.0-beta --branch HEAD
```

Use `--json` for automation. A nightly audit can also set `--threshold` and
`--max-runtime`; `--rescan` is the more expensive opt-in that re-evaluates the
walk with today's rules.

The witness audit answers a narrow question: _do the commits in this range have
the expected witness coverage?_ It does not claim that every historical result
would pass today's policy.

## Review capsules

A review capsule is a directory containing the commit-range manifest, policy and
rule identity, baseline and witness material, collected digests, diagnostics,
exceptions, and the recorded closed-state verdict.

Create one outside the repository by default:

```bash
anvil capsule create --range v0.9.0-beta..HEAD --out ../review-capsule
```

Then use the two read paths for different jobs:

```bash
anvil capsule explain ../review-capsule
anvil capsule verify ../review-capsule
```

- `explain` is descriptive. It reports what the capsule contains and always
  exits zero when the capsule can be read, regardless of its recorded verdict.
- `verify` adjudicates the capsule. It exits `0` for pass/warn, `1` for block,
  `2` for degraded, and `3` for an invalid or unreadable capsule.

For CI, request the canonical verification document and still gate on the exit
status:

```bash
anvil capsule verify --json ../review-capsule
```

Verification is closed-state and offline-capable. When the source repository is
available, Anvil also re-collects repository digests instead of trusting the
capsule's copies alone.

## What the trail can prove

| Question                                                            | Surface                       |
| ------------------------------------------------------------------- | ----------------------------- |
| Did reachable commits bypass the witness path?                      | `anvil audit-chain`           |
| What range and governance inputs does this package claim?           | `anvil capsule explain`       |
| Are the capsule structure, digests, and closed-state verdict valid? | `anvil capsule verify`        |
| What local value signal has accumulated?                            | `anvil insights --cumulative` |
| How has architecture changed between snapshots?                     | `anvil drift compare`         |

These are deterministic local checks. They do not upload repository content or
silently create a remote compliance store.

## Retention is explicit

Anvil never deletes review capsules automatically. If your team deliberately
stages capsules inside the repository, preview retention candidates first:

```bash
anvil capsule prune --keep-last 10
```

Apply only after reviewing the list:

```bash
anvil capsule prune --keep-last 10 --apply
```

`--apply` stages tracked deletions through Git; it does not commit them. A
capsule the repository cannot order is kept, and deleting every capsule remains
a manual decision.

For the capsule format and workflow in more detail, see
[Review Capsules](./review-capsules.md). For everyday protection state versus
daemon session state, see [Runs and Daemon Sessions](./sessions.md).
