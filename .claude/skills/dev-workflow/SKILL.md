---
name: dev-workflow
description: Use at the start of any development task to route to the correct skill, agent, and command for each lifecycle stage. Covers the full APS → branch → code → council → PR → cleanup loop.
---

# Dev Workflow

Routing layer for the development lifecycle. Every task follows this sequence — do not skip stages.

```
APS (Ready) → Branch → Code → Review → PR → Merged → [cleanup] → Released/Shipped
```

## Stage Map

| Stage | What | Skill | Agent | Command |
|---|---|---|---|---|
| **Idea / spec** | Explore intent, design before code | `brainstorming` | — | — |
| **Plan** | Write implementation plan from spec | `writing-plans`, `planning-council` | `anvil-plan-spec` | `/plan` |
| **Branch** | Create isolated worktree from `main` | `using-git-worktrees` | — | — |
| **Code** | TDD implementation | `test-driven-development` | `tdd-coach` | `/test` |
| **Debug** | Root cause analysis | `systematic-debugging` | `debugger` | `/debug` |
| **Verify** | Evidence before completion claims | `verification-before-completion` | — | — |
| **Review (streaming)** | Iterative council during implementation | `local-review-council` | `council-reviewer` | `/council` |
| **Review (batch)** | Formal multi-persona dossier at milestone | `council` | `council-reviewer` + `adversarial-reviewer` + specialists | `/council batch` |
| **Finish** | Commit, push, open PR | `finishing-a-branch` | — | `/commit` |
| **Address review** | Resolve PR feedback and CI failures | `addressing-pr-reviews` | — | — |
| **Parallelise** | Independent tasks concurrently | `parallel-agents` | `autonomous` | `/delegate` |

## Rules

1. **Always start from APS.** Pick a `Ready` work item. Mark it `In Progress` before writing code.
2. **Branch from `main`.** Hotfixes also branch from `main` (or the latest good tag if `main` is unreleasable). Use the project naming convention (`feat/*`, `fix/*`, `docs/*`, `chore/*`).
3. **Council is the review surface.** Run Streaming Council during implementation for fast iteration; run Batch Council before opening the PR for the formal review dossier. GitHub PRs are publication artifacts, not the primary review workspace. Address CRITICAL and MAJOR findings before push.
4. **Mark Merged on PR merge.** Not Complete — the cleanup agent advances `Merged → Released/Shipped → Complete` when release evidence confirms ship.
5. **Extract post-merge test plans.** Do not leave them in the PR description only. Write to `plans/reviews/post-merge/<branch-slug>.md` when the project uses that path.
6. **Verify before claiming complete.** Evidence before assertions — use `verification-before-completion`.

## Decision Points

**Starting a new task:**
→ Check `plans/index.aps.md` for next Ready item → `using-git-worktrees` (from `main`) → code

**Implementation unclear:**
→ `brainstorming` → `writing-plans` (or `planning-council` for multi-persona design) → `using-git-worktrees` → code

**Tests failing unexpectedly:**
→ `systematic-debugging` before any other action

**About to commit:**
→ `verification-before-completion` gate → `local-review-council` (streaming) or `council` (batch) → `finishing-a-branch`

**PR review feedback returned:**
→ `addressing-pr-reviews` — fix CI first, then walk each unresolved thread

**Multiple independent tasks:**
→ `parallel-agents` to dispatch subagents per task

## APS Status Lifecycle

```
Draft → Proposed → Ready → In Progress → Merged → Released/Shipped → Complete
                              ↑              ↑              ↑              ↑
                        (you start)   (PR merged)   (release record)  (cleanup agent)
```

`Committed` is legacy wording for `Merged`. New APS text should prefer
`Merged` and `Released/Shipped`. Cleanup agent (`scripts/aps-cleanup.sh` where
present) auto-advances post-merge states when release evidence is recorded.

## Project References

- Branching strategy: `docs/guides/branching-strategy.md`
- Worktree policy: `docs/guides/worktree-policy.md`
- APS rules: `plans/aps-rules.md`
- Post-merge template: `plans/reviews/post-merge/TEMPLATE.md`
- Council architecture: see `council` skill (Streaming vs Batch modes)
