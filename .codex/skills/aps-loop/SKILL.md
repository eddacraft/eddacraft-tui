---
name: aps-loop
description: >-
  APS execution loop for projects that use interactive planning and design with
  autonomous implementation. Use after the user has approved a ready APS plan:
  "run the APS loop", "work through the plan", "execute the approved plan",
  "implement and keep the plan current".
---

# APS Loop

Run an approved APS plan through autonomous implementation cycles while keeping
the plan true. Planning and design are interactive; execution is autonomous once
the user has approved a ready item or action plan.

If `plans/index.aps.md` does not exist, this skill does not apply. Route to
`planning-workflow` to decide whether APS should be introduced.

## Contract

```
Interactive: Intent -> Discovery -> Design -> Plan -> Ready Item
Autonomous: Select -> Implement -> Verify -> Review -> Reconcile -> Evolve -> Repeat
Checkpoint: product, scope, safety, or irreversible decisions only
```

The loop starts only after the interactive planning contract is satisfied:

- the goal maps to an APS item or action plan;
- the user has approved the design or readiness decision;
- the expected outcome is testable;
- validation evidence is defined;
- dependencies are closed or explicitly documented.

## Operating Stance

Proceed autonomously for reversible implementation, validation, review, APS
bookkeeping, drift correction, and plan evolution within the approved scope. Ask
the user only for product intent, scope changes, safety decisions, destructive or
irreversible actions, or input no tool can discover.

Before reporting progress, audit each claim against evidence from this session:
files changed, commands run, review results, or APS updates. If something is not
verified, say so explicitly.

Do not end a turn with a plan or promise when the next action is safe and within
scope. Do the work, record the evidence, then report the outcome.

## The Loop

```
Orient -> Select -> Validate -> Implement -> Verify -> Review -> Reconcile -> Evolve -> Repeat
```

1. **Orient.** Load APS context via `aps-planning`. Read relevant module files,
   `plans/execution/lessons/` if present, and the tail of
   `plans/execution/loop-journal.md` if present. Resume interrupted iterations
   from the journal rather than starting fresh.
2. **Select.** Choose the highest-priority `Ready` item with no unmet
   dependencies. If the approved plan defines independent action waves, use
   `parallel-agents` where available and keep working while they run. If nothing
   is Ready, route to **Evolve**.
3. **Validate.** Run the APS truth gate. Treat drift as replanning input, not a
   reason to stop. If the item is stale, blocked, ambiguous, or already done,
   correct it through `planning-workflow` / `aps-planning`, then continue from
   the corrected state.
4. **Implement.** Route one ready item through `dev-workflow`: isolated branch or
   worktree, TDD where practical, focused changes, local verification, and review
   gates. Do not widen scope while implementing.
5. **Verify.** Run the item's validation command and any repo-mandated
   CI-equivalent checks. Where possible, ask a fresh-context reviewer or subagent
   to compare the diff against the item's Expected Outcome. Failures route back
   to implementation or `systematic-debugging`.
6. **Review.** Use `local-review-council` during implementation when available,
   and `council` for milestone or high-risk changes. Address critical and major
   findings before marking the item done.
7. **Reconcile.** Update APS status with validation evidence, add discovered
   files to `Files:` fields, append the journal entry, and keep APS bookkeeping
   separate from feature commits.
8. **Evolve.** Make the plan true for the next cycle within the authority table
   below: add Proposed items for discovered work, split or retire stale items,
   unblock dependencies, and refresh module/index status when evidence supports
   it.

## Plan Evolution Authority

| Change                                               | Loop may apply autonomously |
| ---------------------------------------------------- | --------------------------- |
| Item statuses and validation evidence                | yes                         |
| `Files:` fields and drift corrections                | yes                         |
| Action plans within an approved item                 | yes                         |
| New `Proposed` items within the approved scope       | yes                         |
| Splitting, merging, or retiring items with rationale | yes                         |
| Index Problem, Success Criteria, Constraints         | no - checkpoint             |
| Accepted ADRs or project policy                      | no - checkpoint             |
| Deleting modules or abandoning a milestone           | no - checkpoint             |
| Destructive or irreversible actions                  | no - checkpoint             |

A checkpoint means record the proposal in the journal, surface it plainly to the
user, and continue with other available work if any exists. End the turn only
when no safe in-scope work remains.

## Memory

Store durable execution learnings in `plans/execution/lessons/` when the project
uses that directory. Each lesson should explain what changed future execution and
why. Prefer updating or deleting existing lessons over creating duplicates.

Read relevant lessons during Orient and fold them into new work item text during
Evolve. The loop should compound rather than rediscover the same facts.

## Journal

Append one entry per cycle to `plans/execution/loop-journal.md` when the project
uses APS execution files:

```markdown
## Cycle N - YYYY-MM-DD

- Item: ID - title
- Outcome: done | blocked | replanned | validation-failed
- Validation: command plus result summary
- Review: reviewer/council result summary
- Plan changes: items added, updated, split, retired, or unblocked
- Checkpoints raised: none, or the exact user-level decision needed
- Next: selected item or stop condition
```

The journal is the resume point and audit trail. Treat it as bookkeeping: commit
it with plan changes, never hidden inside feature work.

## Stop Conditions

Stop, report, and end the turn when:

- no Ready work remains and no in-scope Proposed work can be promoted safely;
- every remaining item is blocked on user input or an external dependency;
- a product, scope, safety, or irreversible checkpoint is the only work left;
- validation fails after three focused attempts on one item;
- a user-specified cycle, time, or cost limit is reached.

The final report should open with outcomes: items completed, validation evidence,
plan changes made, and any checkpoint decision needed from the user.

## Cross-References

- `planning-workflow` - interactive planning and readiness gate
- `dev-workflow` - implementation of one ready item
- `aps-planning` - APS context, truth validation, and reconciliation
- `writing-plans` - implementation plans after design approval
- `parallel-agents` - independent action waves
- `systematic-debugging` - validation failures and regressions
- `verification-before-completion` - evidence before completion claims
