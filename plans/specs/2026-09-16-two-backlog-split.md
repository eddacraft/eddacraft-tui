# Two-Backlog Audience Split

**Module:** CIB (Continuous Improvement Backlog) → CIB + PROD
**Status:** Draft
**Date:** 2026-09-16

## Overview

Split the single continuous-improvement backlog into two queues partitioned by
**audience**: work a user of anvil feels, and work only we feel. Both queues
shed completed entries to an archive, and a deterministic gate warns when a
`Ready` entry's premise has gone stale.

This spec records the design approved on 2026-09-16. The governing decision is
[ADR-148](../decisions/148-two-backlog-audience-split.md).

## Problem

`plans/modules/continuous-improvement-backlog.aps.md` holds **425 entries in
14,380 lines**, of which 337 are Released/Merged/Done and 58 are open. Three
distinct failures follow from that single file:

1. **Category collision.** Roughly half the 58 open entries are product-shaped
   (a scanner reporting the wrong column, a secret rule blind to `camelCase`
   bindings, a remediation that provably changes nothing) and half are
   workshop-shaped (pre-commit globs, CI path classifiers, worktree disk
   pressure). They compete for the same attention under the same "what is
   Ready" heuristic. On 2026-09-16 a three-item wave spent three repair rounds
   on pre-commit glob coverage while product defects sat `Proposed`.

2. **Hot-file contention.** The file is a shared multi-writer surface that
   feature PRs are forbidden to touch precisely because of merge collisions.
   CIB-404 exists *because* a sibling PR conflicted on this file and
   `pr-required-status` then misreported the conflict as "not finished".

3. **No currency.** CIB-401 sat `Ready` for thirteen days after commit
   `dfbf5aefd` deleted the code it described; a worker was dispatched against a
   premise that no longer existed. Separately, CIB-403's embedded file count
   drifted 77 → 87 → 88 within a single PR.

## Design

### Partition: audience

The discriminator is **does a user of anvil experience this?** — not who raised
it, and not whether it fits a module.

| Test | Queue |
| --- | --- |
| A user of anvil feels it | PRODUCT |
| Only someone building anvil feels it | CIB |

Origin is explicitly **not** the axis. A product defect we find ourselves is
product work.

### Topology: inbox + thin working set

Both queues share one shape:

```text
INBOX        GitHub issues (labelled, unbounded, zero merge conflicts)
                |
                |  promote when Ready
                v
WORKING SET  a thin .aps.md holding ONLY Ready + In Progress
                |
                |  on merge + reconcile
                v
ARCHIVE      plans/archive/ (history preserved, out of the hot path)
```

- **PRODUCT** — inbox is GitHub issues; working set is
  `plans/modules/product-backlog.aps.md`.
- **CIB** — intake stays the `ci-log` harvest → triage flow, unchanged; working
  set is the existing module, thinned.

### Identity

An inbox item is identified by its **GitHub issue number**. It earns an APS ID
(`PROD-NNN`) only on promotion into the working set. Minting IDs for work that
may never start is part of how CIB reached 425 entries.

### Shedding rule

A completed entry **leaves the working file**. This applies to both queues and
is the only rule that keeps the working set thin enough to stay out of merge
contention.

### Currency gate

A deterministic advisory check over `Ready` entries in both working sets:

```text
pnpm aps:currency

  WARN CIB-401 (Ready): 3 of 4 Files paths missing
    packages/edda-stack/src/ember/proposal-store.ts
    packages/edda-stack/src/ember/*.test.ts
  -> premise may be superseded; re-read before work

exit 0 (advisory, new-edges-only baseline)
```

Warns, does not block. Baselines existing state and reports new violations only
— anvil's own principles applied to anvil's own plans.

### Reuse, not invention

The existing label taxonomy is used as-is: `priority:P0..P4`, `kind:bug|ci|
security|docs`, `readiness:ready|needs-design`, `tracked:needs-aps|
promoted-to-aps`, `area:*`, `tech-debt`. No new labels are required.

The shared multi-writer convention extends unchanged to the product module:
feature PRs never edit either working file; intake, promotion, status
reconcile and count updates happen only on single-writer bookkeeping branches.

## Migration

Sequenced, not one change:

| Step | Action | Result |
| --- | --- | --- |
| 1 | Archive the 337 done CIB entries | CIB ~14,380 → ~2,000 lines |
| 2 | Classify the 58 open entries by audience | ~26 product / ~32 workshop |
| 3 | Product-shaped → GitHub issues; Ready subset → `product-backlog.aps.md` | PRODUCT queue live |
| 4 | Workshop-shaped stay in CIB | CIB keeps its stated purpose |
| 5 | Add the currency gate | Staleness becomes visible |

Step 1 is a large mechanical change to the repo's hottest file and **must land
on a quiet bookkeeping branch with no siblings in flight**.

## Risks

- **Boundary erosion (primary).** "Does a user feel it?" is testable but not
  self-enforcing. A few miscategorised items and this becomes one queue with
  extra steps. No gate is proposed; this is a review-time judgement and is
  named here rather than papered over.
- **Archive-split collision.** Step 1 touches the file most likely to conflict.
- **The gate catches stale premises, not stale judgement.** An entry whose code
  still exists but is no longer worth doing stays invisible.

## Non-goals

- No change to the `ci-log` harvest or triage flow.
- No public customer-facing tracker; `eddacraft/anvil-001` stays private.
- No new tooling beyond the currency gate.
- No change to how modules themselves are planned or archived.

## References

- ADR-148 (governing decision)
- `plans/project-context.md#work-item-claim-issues`
- `plans/project-context.md#keeping-plans-current`
- `docs/guides/continuous-improvement-log.md`
- CIB-401 (stale premise), CIB-403 (drifting derived facts), CIB-404 (hot-file
  conflict misreported)
