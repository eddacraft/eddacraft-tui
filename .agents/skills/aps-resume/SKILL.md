---
name: aps-resume
description: >-
  Trust-window resume for dev-loop drain mode: rehydrate a crashed or
  compacted run from the committed journal without silently replaying
  non-idempotent work. Invoked by the loop on cold restart.
---

# APS Resume

A drain-mode support leaf for canonical `dev-loop`. When a fresh agent wakes against an
interrupted run, it must answer two questions before doing any work: **where was
the loop**, and **what is safe to assume already happened**. This skill encodes
both — cheaply when state is provably fresh, paranoidly when it is not.

**Layering:** a single `dev-loop` run resumes from its run checkpoint
(`.dev-loop/checkpoints/<runId>.json`) via `resume <TARGET>`. This skill governs
the layers around that file: the committed run-journal for drain-level resume
across cycles, the trust window for skipping re-verification, and
non-idempotent-action safety. On wake, locate the run checkpoint first; apply
the trust window to decide whether its claims need re-verification.

If `plans/index.aps.md` does not exist, this skill does not apply.

## Principle

The previous agent is gone; its memory died with it. Reconstruct from durable,
committed truth, not from a transcript. Re-derive by re-running validation
**unless** the state is provably fresh — re-running a twenty-minute suite on
every wake burns the budget and may never make progress. "Trust nothing" is the
exception for genuinely stale state, not a tax on every resume.

## Durable truth

Three artifacts, in priority order, survive an agent's death:

1. **APS work-item status + committed action checkpoints** — the ground truth.
   What is committed happened; what is not committed did not.
2. **The run-journal** (`plans/execution/loop-journal.md`) — the durable,
   **committed** resume record. It is not a throwaway scratchpad: it is
   committed as standalone bookkeeping so a fresh agent can read it. It records
   the baseline SHA, the last-verify result and timestamp, the recorded phase,
   the worktree/branch name, and the last action checkpoint. Journal commits
   for a work item ride that item's feature branch/worktree; the drain-level
   journal `plans/execution/loop-journal.md` is committed on the branch where
   the loop bookkeeping runs and is reconciled to integration at LAND (see
   `aps-landing`).
3. **Lessons** (`plans/execution/lessons/`) — read at orient, fold forward.

The journal is a **hint**; command output plus APS status is **truth**. When
they disagree, reality wins and the journal is corrected.

## Trust window

On cold resume, re-derive by re-running validation — **unless all three hold**:

1. **Recency** — `last-verify` is within window **T** (configurable; default
   ~30 min). Stale time means the world may have moved.
2. **Baseline match** — the baseline SHA recorded in the journal equals the
   current branch HEAD. A different HEAD means commits landed since verify.
3. **Scope unchanged** — `git diff` shows no files in the item's declared scope
   changed since `last-verify`. A changed scope file invalidates the result.

When all three hold, the previous verify result may be **trusted** and
re-verification **skipped**. If any fails, re-run the full validation. Record in
the journal which branch was taken (`trusted last-verify` vs `re-verified`) so
the audit trail is identical regardless.

T is a knob, not a constant: lengthen it for slow suites on quiet branches,
shorten it toward zero for shared or fast-moving branches.

## Non-idempotent actions

Each action in the plan declares its replay safety (e.g. `Idempotent: false`).
Non-idempotent actions append to a migration, send a request, mutate external
state — re-running them is **not** a no-op.

- **Non-idempotent actions are NEVER auto-replayed on resume.** Either escalate
  for a human decision, or reset-to-baseline and replan from that action
  forward. Never re-run on a guess.
- **Idempotent actions** may be safely re-run, or skipped when their output
  already exists. Check for the output, then decide.

If an action's replay safety is undeclared, treat it as non-idempotent.

## Heartbeat

The executing agent writes a **monotonically increasing sequence** to a
heartbeat after **each tool call**, separate from the action log. The heartbeat
lives at **`.dev-loop/heartbeat.<runId>`**, alongside the run checkpoints. Its
format is a single line — the monotonic action sequence number plus an ISO 8601
timestamp — **overwritten** after each tool call, never appended. The action
log's sequence numbers live in the committed run-journal entries: each journal
action checkpoint records the sequence at which that action completed.

On resume, compare:

- `heartbeat.sequence == last-recorded-action sequence` → the agent exited
  **cleanly between actions** (e.g. token limit, clean stop). The last action
  completed; resume after it.
- `heartbeat.sequence > last-recorded-action sequence` → the agent was killed
  **mid-action**. The partial action is **suspect**: it may have half-applied.
  Treat it under the non-idempotent / worktree-triage rules — do not assume it
  completed and do not blindly redo it.

The heartbeat exists only to distinguish a clean exit mid-action from a
completed action; it is never used as evidence that work succeeded.

## Cold-rehydration protocol

A fresh agent runs this verbatim — no guessing, no shortcuts:

1. **Read the index "What's Next".** Find the `In Progress` work item(s).
2. **Open the journal / resume token** for that item: baseline SHA, last-verify,
   recorded phase, worktree/branch, last action checkpoint, heartbeat sequence.
3. **Re-attach the named worktree / branch.** Do not start fresh on the default
   branch.
4. **Worktree triage FIRST.** If the tree is dirty / has uncommitted changes it
   is **suspect** — a half-applied tree is neither baseline nor done. Before any
   re-validation: stash the changes, re-establish the baseline, and decide
   replay vs discard (using the heartbeat and the action's idempotency). Only
   then proceed. Do not re-validate on top of an unexplained dirty tree.
5. **Re-run baseline + last verify, subject to the trust window.** If the trust
   window holds, trust the recorded last-verify and skip; otherwise re-run.
6. **Reconcile the journal against reality** — `git log`, the working tree, APS
   status. The journal is a hint; command output and APS status are truth.
   Correct the journal where they disagree.
7. **Resume at the recorded phase.** Hand back to the loop at the phase the
   journal recorded, with the reconciled state.

## Checkpoint cadence

The executing loop must journal frequently enough that a token-limit death loses
at most one action:

- after **every action checkpoint**;
- after **every verify** (record result, timestamp, baseline SHA);
- **immediately before any stop** (planned or forced, where possible).

This cadence is what makes the resume protocol above cheap and lossless. A
journal written only once per cycle cannot bound the loss to one action.

## Cross-references

- `plans/designs/2026-06-18-canonical-aps-loop.design.md` — historical source
  (design change #6 / gap G8: trust-window resume + non-idempotent
  handling + heartbeat) (catalogue-repo history — not present in consuming
  projects; do not read at runtime)
- `dev-loop` — the canonical loop this leaf serves (Orient/resume step)
