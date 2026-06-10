---
name: f5-dev-workflow
description: >-
  Fable 5-tuned development-stage orchestrator. Routes a ready work item
  through branch, implementation, verification, review, and PR with the same
  hard gates as dev-workflow but outcome-stated rather than step-enumerated,
  plus scope discipline, evidence-grounded completion, and parallel
  verification. Use at the start of any development task when running on
  Claude Fable 5; prior-model projects use dev-workflow instead.
---

# Fable 5 Dev Workflow

Turn one ready work item into a merged, reviewed, verified change.

```
Plan truth gate -> Branch -> Code -> Verify -> Review -> PR -> Cleanup
```

The lifecycle and its gates are identical to `dev-workflow`; this variant
trusts the model with the how and is strict about the what.

## Operating stance

Don't add features, refactor, or introduce abstractions beyond what the work
item requires. A bug fix doesn't need surrounding cleanup; do the simplest
thing that works well. Validate at system boundaries (user input, external
APIs) and trust internal code and framework guarantees.

Before reporting progress, audit each claim against a tool result from this
session. Only report work you can point to evidence for; if something is not
yet verified, say so explicitly. If tests fail, say so with the output; if a
step was skipped, say that.

Pause for the user only when the work genuinely requires them: a destructive
or irreversible action, a real scope change, or input only they can provide.
If you hit one, ask and end the turn rather than ending on a promise.

## Gates (non-negotiable)

1. **Start from planning truth.** No validated ready item → route to
   `f5-planning-workflow` first. In APS projects, run the `aps-planning`
   truth gate before branching; never implement from a stale, ambiguous, or
   blocked item.
2. **Branch in isolation** from the project integration branch, per the
   project's branch naming and worktree policy (`using-git-worktrees`).
3. **Tests carry the proof.** Test-first where practical
   (`test-driven-development`); where it isn't, record the replacement
   evidence before coding. Failing tests route to `systematic-debugging`
   before any other action.
4. **Verify before claiming complete** (`verification-before-completion`).
   Run the repo-mandated CI-equivalent commands locally and green before any
   push — check `CLAUDE.md` or project docs for the exact commands. CI is a
   backstop, not the primary signal. Tick test-plan boxes only after the
   command actually ran.
5. **Review before PR.** `local-review-council` while implementing, `council`
   at milestones; address critical and major findings before opening or
   updating the PR. PR feedback afterwards goes through
   `addressing-pr-reviews` — CI first, then each unresolved thread.
6. **Atomic artefacts, single-purpose PRs.** Lockfiles and files generated
   from them ship in the same PR as the change that caused them. APS
   bookkeeping (status updates, index counters) ships standalone, never
   bundled with feature or dependency work.

## Parallelism and verification

Delegate independent subtasks to subagents and keep working while they run;
intervene if one goes off track or is missing context. When dispatching, give
the reason behind the request, not just the request — subagents perform
better knowing what the output enables. For implementation runs longer than a
few commits, have a fresh-context subagent verify the work against the work
item's expected outcome at natural checkpoints (after each green validation
command, before the PR); fresh eyes outperform self-critique.

## Finish

`finishing-a-branch` handles commit, push, PR, and worktree cleanup. Commits
follow conventional format (`commit`); in APS projects the `APS:` trailer and
PR work-item section come from `aps-planning`. After merge, reconcile APS
state and record any lesson worth keeping (one note, the why included) — in
loop runs, `f5-aps-loop` owns this step.

## Cross-references

- `f5-planning-workflow` — upstream: produces the ready item
- `f5-aps-loop` — wraps this workflow in an autonomous implement→replan loop
- `aps-planning`, `using-git-worktrees`, `test-driven-development`,
  `systematic-debugging`, `verification-before-completion`,
  `local-review-council`, `council`, `finishing-a-branch`,
  `addressing-pr-reviews`, `parallel-agents` — stage leaves, unchanged
- Tuning rationale: `docs/fable5-agent-tuning.md` (eddacraft-skills catalogue)
