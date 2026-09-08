---
name: aps-landing
description: >-
  Crash-safe merge boundary for dev-loop drain mode: fenced LAND sequence,
  race-free status reconciliation, and wave locks for parallel execution.
  Invoked by the loop when landing items or resuming an interrupted merge.
---

# APS Landing

The LAND checkpoint is the one **irreversible** boundary in an APS-aware
`dev-loop` drain run:
entering shared history (checkpoint (3)). Everything before it lives in a
throwaway branch or worktree and is reversible; merging is not. This skill makes
that boundary crash-safe, and makes parallel reconciliation race-free, so an
unattended loop can never produce a duplicate merge or a lost status update.

**Layering:** `land-branch` owns per-item landing (PR, merge authority,
integration-ancestor gate, APS reconcile). This skill supplies what `land-branch`
executes under drain and parallel runs: the crash-safe fenced sequence for
autonomous merges, and the single-writer queue plus wave locks that keep many
workers from corrupting the plan.

Two distinct problems, two mechanisms:

1. **The merge boundary** — a single landing must survive a crash at any point
   between three separate writes (git, APS, journal) without re-merging.
2. **Parallel reconciliation** — many workers landing in waves must never
   corrupt the plan files; APS is a **single-writer** resource.

## The fenced LANDING sequence (checkpoint (3))

Landing involves three independent durable writes — the **git** merge, the
**APS** status, and the **journal** (`plans/execution/loop-journal.md`)
record — and a crash can fall between any of
them. The fence makes the order recoverable. Run these steps **in order**; never
reorder, never skip the journal writes.

```
1. journal: write LANDING(sha)            # fencing token — BEFORE the merge
2. git:     merge                          # the irreversible act
3. journal: write MERGED(merge_sha, merged_at)
4. APS:     set work item status = Merged  # interim — NOT Complete (ADR-0013)
5. cleanup: remove the worktree / branch   # ONLY after the status write
```

Each clause is load-bearing:

- **(1) before (2).** The `LANDING(sha)` token is written to the journal
  _before_ the merge so that, on resume, we always know a landing was attempted
  for this `sha` even if the process died mid-merge.
- **(4) is `Merged`, not `Complete`.** Merging a PR does not finish a work item.
  Per ADR-0013, `Complete` is **tag-gated**: a separate cleanup agent advances
  `Merged -> Released/Shipped -> Complete` only on release/ship evidence. The
  loop sets `Merged` and stops. Do not self-declare done.
- **(5) only after (4).** Worktree/branch cleanup happens _after_ the status
  write, never before. Cleaning up first would destroy the evidence needed to
  retry a landing that had not yet recorded its status.

**Who runs the fence.** In a **solo drain** (one executor, no wave), the
executor runs fenced steps 1–5 directly — it is the only writer, so the
single-writer rule is trivially satisfied. In a **parallel wave**, only the
**conductor** runs the fenced sequence, serially at the wave gate. Workers
arrive at the gate with VERIFY green and append status events to the queue
only — fenced step 4 becomes a queue append, applied by the conductor as the
sole writer of index/module files.

### Record the merged SHA in two places

Write the merge SHA and timestamp to **both** the journal (`MERGED(merge_sha,
merged_at)`) **and** the work item itself (as merge evidence on the status
line). A merged item carries its own proof, so the loop can recognise an
already-merged item and **never re-merge it** — no duplicate PRs, no duplicate
merges.

## RESUME disambiguation (the three-write split-brain window)

Because git, APS, and the journal are three separate writes, a crash leaves the
journal possibly ahead of (or behind) reality. On resume, when the journal shows
`LANDING` for an item but the work-item status is **not yet `Merged`**, you are
inside the split-brain window. Do **not** blindly retry — that risks a second
merge. Instead, **ask git, the ground truth**:

```
journal == LANDING  AND  status != Merged  ->  check target-branch history for merge_sha
   merge_sha present in target branch  ->  the merge LANDED: skip to post-merge
                                           (write MERGED if missing, set Merged, cleanup)
   merge_sha absent from target branch ->  the merge did NOT land: safe to retry from step 1
```

The target branch's commit history is authoritative for whether the merge
actually happened. This resolves every interleaving of the three writes without
ever merging twice.

## Single-writer reconciliation (parallel waves)

APS index and module `.aps.md` files are a **single-writer resource**. In a
parallel wave, worker subagents **NEVER** write index/module files directly.
Concurrent writers to the same markdown file lose updates.

Instead:

- Each worker enqueues one complete status event through a provider that offers
  an explicit cross-platform lock and atomic replacement (or an equivalent
  transaction). Do not treat POSIX append behaviour as a portable concurrency
  guarantee.
- A **single conductor** drains the queue **serially** at the wave gate and is
  the _only_ process that edits the index and module files.

The current bundled append-only implementation is a POSIX fallback. On native
Windows without a conforming landing-queue adapter, disable parallel landing
waves and reconcile items serially; do not silently invoke Git Bash.

```
worker A ─┐
worker B ─┼─enqueue validated event─▶ [locked landing queue]
worker C ─┘                                  │
                                          conductor drains serially at the gate
                                          (sole writer of index/module .aps.md)
```

This turns N concurrent writers into one serial writer, so no status update is
ever lost and the plan files never tear.

## Wave file-locks (actual, not declared)

A wave is a set of items run concurrently. Membership requires the items' file
sets to be **actually pairwise-disjoint** — two items in the same wave must not
touch the same file.

- Compute file sets from `git diff --name-only` against the baseline **AFTER
  BUILD** — the _actual_ changed files — **not** the declared best-effort
  `Files:` field, which is advisory and routinely incomplete.
- The "small slack outside declared scope" (files an item touched beyond what it
  declared) **counts toward lock eligibility retroactively**. Real edits hold
  locks, declarations do not.
- If two completed items' actual file sets **intersect**, they cannot share a
  wave: the **later-completing** item is held back to the **next** wave. The
  earlier one lands; the later one re-evaluates against the new baseline.

Because eligibility is recomputed from real diffs after build, an item cannot
sneak a conflicting change into a wave by under-declaring its `Files:`.

## The wave gate is a barrier

A wave gate is a **barrier**: every member must be green (verified) before any
member of the **next** wave starts. At the gate:

- The conductor drains the status queue and reconciles index + modules (single
  writer).
- Members that landed advance to `Merged` via the fenced sequence above.
- A **failed** member **parks itself** (records the failure, escalates) and
  **blocks only its dependents** — independent items in later waves proceed. A
  single failure never stalls the whole plan, only the subtree that needs it.

## Cross-references

- `plans/designs/2026-06-18-canonical-aps-loop.design.md` — historical source
  (design changes #4 and #5 / gap G6; LAND fencing, single-writer
  reconciliation, actual-file wave locks) (catalogue-repo history — not
  present in consuming projects; do not read at runtime)
- `plans/decisions/0013-aps-loop-state-model.md` — ADR-0013, the lifecycle
  decision: `Merged` is interim, `Complete` is tag-gated and advanced later by
  the cleanup agent on release/ship evidence (catalogue-repo history — not
  present in consuming projects; do not read at runtime; the behaviour is
  stated inline above)
- `dev-loop` — the canonical loop whose LAND checkpoint (3) this skill realises
