---
name: f5-aps-loop
description: >-
  Autonomous APS execution loop for Claude Fable 5. Implements the current
  APS plan item by item and evolves the plan as it learns — reconciling
  state, folding in lessons, and generating the next plan iteration after
  each cycle. Use for unattended or long-horizon runs against a project with
  plans/index.aps.md: "run the APS loop", "work through the plan", "implement
  the plan and keep it current". Wraps f5-dev-workflow and
  f5-planning-workflow; supersedes autonomous-execution on Fable 5.
---

# Fable 5 APS Loop

Run an APS plan end to end: pick the next item, implement it, verify it,
reconcile the plan, evolve the plan, repeat. The plan is both the input and
an output of every cycle — each iteration leaves behind a plan that is more
true than the one it started from.

If `plans/index.aps.md` does not exist, this skill does not apply; offer
`f5-planning-workflow` to bootstrap one instead.

## Operating stance

You are operating autonomously. The user is not watching in real time and
cannot answer questions mid-task. For reversible actions that follow from the
plan, proceed without asking. Before ending a turn, check your last
paragraph: if it is a plan, a question, or a promise about work not yet done,
do that work now with tool calls. End the turn only when the loop has reached
a stop condition.

Before reporting progress, audit each claim against a tool result from this
session. Only report work you can point to evidence for; if something is not
yet verified, say so explicitly.

Do not stop, summarize, or suggest a new session on account of context
limits; if the harness compacts context, the journal and plan files carry the
state you need to continue.

## The loop

```
Orient -> Select -> Validate -> Implement -> Verify -> Reconcile -> Evolve -> (repeat)
```

1. **Orient.** Load APS context via `aps-planning` (modules, items, statuses,
   file map). Read `plans/execution/lessons/` and the tail of
   `plans/execution/loop-journal.md`. If a previous iteration was
   interrupted, resume it from the journal rather than starting fresh.
2. **Select.** Highest-priority `Ready` item with no unmet dependencies. When
   an action plan defines waves, dispatch independent actions to parallel
   subagents and keep working while they run. If nothing is Ready, skip
   straight to **Evolve** — replanning is the work.
3. **Validate.** Run the `aps-planning` truth gate. Drift (already-done work,
   moved files, stale assumptions, invalid commands) is not a blocker; it is
   replanning input. Correct the item via `f5-planning-workflow`, then
   continue with the corrected item.
4. **Implement.** Route through `f5-dev-workflow` — its gates (worktree,
   tests, local CI-equivalent green, review, single-purpose PRs) all apply
   inside the loop.
5. **Verify.** Run the item's validation command. Then have a fresh-context
   subagent check the work against the item's Expected Outcome — give it the
   item text and the diff, not your conclusions. Failures route back to
   implement (or to `systematic-debugging`); three consecutive failed
   attempts on one item mark it `Blocked` with the evidence, and the loop
   moves on.
6. **Reconcile.** Update item status with the validation evidence, add new
   files to `Files:` fields, append the journal entry, and commit plan
   changes as a standalone bookkeeping commit (the `APS:` trailer applies).
7. **Evolve.** Generate the next iteration of the plan — see below.
8. **Continue or stop.** Next cycle, unless a stop condition holds.

## Evolving the plan

Each cycle ends by making the plan correct for the next cycle. Drawing on
what implementation just taught you — drift found, scope discovered,
assumptions confirmed or broken, lessons recorded — update the plan files:

- advance or correct work item statuses with their evidence;
- draft new `Proposed` items for discovered work, following
  `plans/aps-rules.md` (intent, outcome, validation — never implementation
  steps);
- split items that proved too large; retire items the work made moot, with a
  note saying why;
- unblock items whose dependencies just closed;
- refresh module status and the index Modules table when a module's items
  all close;
- when a whole milestone closes or remaining items run thin, draft the next
  module or milestone as `Proposed`, grounded in the index's Problem and
  Success Criteria — the plan should always contain the next coherent slice
  of work, or an explicit statement that none remains.

### Authority

| Change | Loop may apply autonomously |
| --- | --- |
| Item statuses, `Files:` fields, drift corrections, action plans | yes |
| New `Proposed` items and modules within the index's stated scope | yes |
| Splitting, merging, retiring items (rationale recorded) | yes |
| Index Problem, Success Criteria, Constraints; accepted ADRs | no — checkpoint |
| Deleting modules or abandoning a milestone | no — checkpoint |

A checkpoint means: record the proposal in the journal, surface it in your
report (or via the harness's send-to-user mechanism if one exists), and
continue with other work if any is available — only end the turn when
nothing else can proceed.

## Memory

Keep lessons in `plans/execution/lessons/`, one lesson per file with a
one-line summary at the top. Record corrections and confirmed approaches
alike, including why they mattered. Don't save what the repo or plan already
records; update an existing note rather than creating a duplicate; delete
notes that turn out to be wrong. Read the lessons during Orient and fold the
relevant ones into new work item text during Evolve — that is how runs
compound.

## Journal

Append one entry per cycle to `plans/execution/loop-journal.md`:

```markdown
## Cycle N — YYYY-MM-DD

- Item: ID — title
- Outcome: done | blocked | replanned (validation evidence: command + result)
- Plan changes: items added/updated/retired
- Checkpoints raised: (if any)
- Next: selected item or stop condition
```

The journal is the loop's resume point and the user's audit trail. It is
bookkeeping — commit it with the plan changes, never with feature work.

## Stop conditions

Stop, report, and end the turn when:

- no `Ready` or justifiable `Proposed` work remains in scope — the plan says
  so explicitly;
- every remaining item is `Blocked` on user input or an external dependency;
- a checkpoint-level decision is the only work left;
- a safety limit set for the run (max cycles, max wall-clock, or the
  project's own limits) is reached.

The final report is the user's first look at the run. Open with the outcome
— items completed, plan changes made — in plain language, then what you need
from them. Drop working shorthand; spell things out.

## Harness integration

Cycles are long. Prefer a harness that checks on the run asynchronously
(scheduled wake-ups, cron-style triggers, background tasks) over one that
blocks on each turn. Use a send-to-user mechanism, where available, for
mid-run deliverables and checkpoint proposals the user must see verbatim.
These are affordances of the runtime, not requirements of the loop — on a
plain interactive harness, the journal and per-cycle reports carry the same
information.

## Cross-references

- `f5-dev-workflow` — implements one item (step 4)
- `f5-planning-workflow` — corrects and extends the plan (steps 3, 7)
- `aps-planning` — APS context loading, truth gate, reconciliation engine
- `plans/aps-rules.md` — item and action format rules
- `autonomous-execution` — prior-model predecessor (checkpoint/retry framework)
- Tuning rationale: `docs/fable5-agent-tuning.md` (eddacraft-skills catalogue)
