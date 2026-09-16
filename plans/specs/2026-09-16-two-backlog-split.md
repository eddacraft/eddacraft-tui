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

### Measurement base

All corpus figures below are pinned to **`f2a8f1522`** and are reproduced by
`scripts/aps/cib-corpus-stats.mjs` (added by migration step 0). They are stated
as an as-of measurement, not a standing fact — this spec's own thesis is that
counts embedded in prose drift, and these will.

At `f2a8f1522`, `plans/modules/continuous-improvement-backlog.aps.md` holds
**426 entries in 14,480 lines**: **342 terminal** (198 Released, 104 Merged,
38 Done, 2 Superseded) and **82 open** (40 Draft, 30 Proposed, 12 Ready), plus
one entry with no parseable `Status:`.

Three distinct failures follow from that single file:

1. **Category collision.** A substantial share of the 82 open entries are
   product-shaped
   (a scanner reporting the wrong column, a secret rule blind to `camelCase`
   bindings, a remediation that provably changes nothing) and half are
   workshop-shaped (pre-commit globs, CI path classifiers, worktree disk
   pressure). They compete for the same attention under the same "what is
   Ready" heuristic. **The exact product/workshop ratio is deliberately not
   asserted here** — establishing it per entry is migration step 2's job, and
   an earlier draft of this spec quoted "~26/~32 of 58 open" from a status
   normalisation that silently dropped variant spellings such as
   `Draft — filed from ci-log triage`. That error is retained in this note
   because it is the same defect class the currency gate exists to catch. On 2026-09-16 a three-item wave spent three repair rounds
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
- **CIB** — intake stays the `ci-log` harvest → triage flow; working set is the
  existing module, thinned.

**Routing happens at promotion, before an ID is minted.** The `ci-log` flow is
audience-blind by construction: a feature session appending a note does not
know, and should not have to decide, which queue the finding belongs to. Triage
does. The promotion step therefore gains the audience question as a gate:

| Triage decision | Destination |
| --- | --- |
| promote + user-facing | GitHub issue, `kind:*` + `area:*` labels; `PROD-NNN` only on promotion to Ready |
| promote + workshop-only | `CIB-NNN` in the CIB working set, as today |
| absorb / leave | unchanged |

This is the single point where the audience test is applied. Without it the two
queues are nominal: `docs/guides/continuous-improvement-log.md` step 3 and
`plans/modules/continuous-improvement-backlog.aps.md` both currently instruct
triage to file `CIB-NNN` unconditionally, so every internally discovered
product defect would keep minting into CIB regardless of this design.

**Downstream amendments required** (part of migration, not optional):

- `docs/guides/continuous-improvement-log.md` — triage step 3 and the promotion
  bar gain the audience question.
- `plans/modules/continuous-improvement-backlog.aps.md` — the intake paragraph
  stops implying `promote: CIB` is the only destination.
- `pnpm ci-log:append --follow-up` — `promote: CIB` becomes one of
  `promote: CIB` | `promote: PRODUCT` | `promote: <queue-undecided>`, the last
  being the honest default for a session that cannot tell.

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

**Matching semantics (required for the gate to be deterministic).** `Files`
bullets are not literal paths: at `f2a8f1522`, 42 of 918 backticked path tokens
are glob or brace patterns (`crates/anvil-cli/src/mcp/tools/*.rs`,
`crates/anvil-graph-cache/src/{snapshot.rs,snapshot_io.rs}`), and CIB-401's own
bullet contains `packages/edda-stack/src/ember/*.test.ts`. A literal `exists`
check would warn on all 42. The gate therefore classifies each token first:

| Token shape | Test | Zero match |
| --- | --- | --- |
| Literal path | path exists | warn |
| Directory (trailing `/`) | directory exists and is non-empty | warn |
| Glob / brace pattern | expand against the tracked file set | warn — "pattern matched nothing" |
| Unparseable / prose | skipped | never warns |

Expansion is over `git ls-files`, not the working tree, so untracked build
output cannot mask a deleted source path. A pattern that matches nothing is a
warning, not silence: "the code this entry describes is gone" is precisely the
CIB-401 signal, and a glob is the shape most likely to carry it. Tokens the
parser cannot classify are skipped rather than guessed — a currency gate that
invents warnings is worse than one with gaps.

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
| 0 | Add `cib-corpus-stats.mjs`; re-measure against the merge base | Figures derivable, not quoted |
| 1 | Archive the 342 terminal CIB entries | CIB ~14,480 → ~2,500 lines |
| 2 | Classify the 82 open entries by audience | Ratio established (not assumed) |
| 3 | Product-shaped → GitHub issues; Ready subset → `product-backlog.aps.md` | PRODUCT queue live |
| 4 | Workshop-shaped stay in CIB | CIB keeps its stated purpose |
| 5 | Amend the triage contract and `ci-log` follow-up vocabulary | Routing applied at promotion |
| 6 | Add the currency gate | Staleness becomes visible |

Step 1 is a large mechanical change to the repo's hottest file and **must land
on a quiet bookkeeping branch with no siblings in flight**. Step 5 is
load-bearing, not cleanup: until the triage contract routes by audience, the
split exists only on paper.

## Risks

- **Boundary erosion (primary).** "Does a user feel it?" is testable but not
  self-enforcing. A few miscategorised items and this becomes one queue with
  extra steps. No gate is proposed; this is a review-time judgement and is
  named here rather than papered over.
- **Archive-split collision.** Step 1 touches the file most likely to conflict.
- **The gate catches stale premises, not stale judgement.** An entry whose code
  still exists but is no longer worth doing stays invisible.
- **Corpus figures in this document will themselves go stale.** They are pinned
  to `f2a8f1522` and reproducible by command for exactly that reason; an
  earlier draft already drifted between writing and review.

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
