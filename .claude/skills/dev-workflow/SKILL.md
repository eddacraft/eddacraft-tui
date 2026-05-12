---
name: dev-workflow
description: Use at the start of any development task to route to the correct skill, agent, and command for each lifecycle stage. Covers the full APS → branch → code → council → PR → cleanup loop.
---

# Dev Workflow

Routing layer for the development lifecycle. Every task follows this sequence — do not skip stages.

```
APS (Ready) → Worktrunk Branch → Code → Review → PR → Merged → cleanup offer → Released/Shipped
```

## Surface inventory

This is a project-local snapshot, but most referenced skills and agents are **globally available** via `joshuaboys/code-env` rather than vendored under `.claude/` here.

| Surface | Repo-local (`.claude/`) | Global (`code-env`) |
|---|---|---|
| Skills | `dev-workflow`, `planning-council`, `release`, `dependabot` (symlink) | `brainstorming`, `writing-plans`, `using-git-worktrees`, `test-driven-development`, `systematic-debugging`, `verification-before-completion`, `addressing-pr-reviews`, `finishing-a-branch`, `parallel-agents`, `council`, `commit`, others |
| Agents | `council-reviewer`, `adversarial-reviewer`, `kernel-maintainer`, `operations-reviewer`, `pragmatic-lead`, `anvil-plan-spec`, `plan-synthesizer`, `tdd-coach` | `debugger`, `autonomous`, others |
| Commands | `/council` (repo-local — see [`commands/council.md`](../../commands/council.md)), `/plan`, `/release`, others | `/test`, `/debug`, `/delegate`, `/commit` |

If a referenced skill or agent is missing locally, it is expected to be globally available — not vendored drift. CIB-002 is the open work item for producing a definitive inventory.

## Stage Map

| Stage | What | Skill | Agent | Command |
|---|---|---|---|---|
| **Idea / spec** | Explore intent, design before code | `brainstorming` | — | — |
| **Plan** | Write implementation plan from spec | `writing-plans`, `planning-council` | `anvil-plan-spec` | `/plan` |
| **Branch** | Create Worktrunk worktree from `main` | `using-git-worktrees` | — | `wt switch --create <branch>` |
| **Code** | TDD implementation | `test-driven-development` | `tdd-coach` | `/test` |
| **Debug** | Root cause analysis | `systematic-debugging` | `debugger` | `/debug` |
| **Verify** | Evidence before completion claims | `verification-before-completion` | — | — |
| **Review (pre-PR)** | Risk-tiered Council | `council` | `council-reviewer` + role-mapped specialists | `/council [quick\|mini\|full] <target>` |
| **Finish** | Commit, push, open PR, offer cleanup | `finishing-a-branch` | — | `/commit` |
| **Address review** | Resolve PR feedback and CI failures | `addressing-pr-reviews` | — | — |
| **Parallelise** | Independent tasks concurrently | `parallel-agents` | `autonomous` | `/delegate` |

## Rules

1. **Always start from APS.** Pick a `Ready` work item. Mark it `In Progress` before writing code.
2. **Use Worktrunk for task branches.** Create task branches with `wt switch --create <branch>` unless the user explicitly asks to continue in the current worktree. Hotfixes also branch from `main` (or the latest good tag if `main` is unreleasable). Use the project naming convention (`feat/*`, `fix/*`, `docs/*`, `chore/*`).
3. **Council is the review surface.** Run `/council [quick|mini|full] <target>` before opening the PR. Default to `quick`; escalate to `mini` for cross-boundary / CI / release / security / workflow risk, and to `full` for branch / release-operating-model changes or high-risk design. See [`commands/council.md`](../../commands/council.md) for the tier table. Address CRITICAL and MAJOR findings before push.
4. **Mark Merged on PR merge.** Not Complete — the cleanup agent advances `Merged → Released/Shipped → Complete` when release evidence confirms ship.
5. **Extract post-merge test plans.** Do not leave them in the PR description only. Write to `plans/reviews/post-merge/<branch-slug>.md`; the gitignore exception `!plans/reviews/post-merge/` keeps these tracked.
6. **Verify before claiming complete.** Evidence before assertions — use `verification-before-completion`.
7. **Always run post-PR review remediation.** After opening a PR, wait up to 10
   minutes for Copilot and other automated reviewers to complete or time out,
   then run `addressing-pr-reviews` even when no automated review comments were
   left. That skill also catches late CI failures. Do not mention or tag bots to
   request a review unless the user explicitly asks.
8. **Use the review remediation order.** Fix failing CI first, then automated
   review comments, then human review comments. Re-run targeted validation after
   each meaningful fix batch.
9. **Offer branch/worktree cleanup at the end.** After a PR is opened, merged,
   abandoned, or paused with no near-term action, ask whether to run
   `wt remove` for the Worktrunk worktree and local branch if it is safe. Never
   delete a branch that is unmerged, unpushed, or still needed without explicit
   user approval.

## Decision Points

**Starting a new task:**
→ Check `plans/index.aps.md` for next Ready item → `using-git-worktrees` via
`wt switch --create <branch>` from `main` → code

**Implementation unclear:**
→ `brainstorming` → `writing-plans` (or `planning-council` for multi-persona design) → `using-git-worktrees` → code

**Tests failing unexpectedly:**
→ `systematic-debugging` before any other action

**About to commit:**
→ `verification-before-completion` gate → `/council` (default `quick`; escalate
to `mini`/`full` for risk) → `finishing-a-branch`

**After PR open:**
→ wait up to 10 minutes for Copilot and automated reviewers to complete or time
out → always run `addressing-pr-reviews`, even with no bot comments, because it
also catches late CI failures; fix CI first, then automated review comments, then
human review comments; do not tag bots unless the user explicitly asks

**After PR merge / abandon / pause:**
→ offer `wt remove`; perform cleanup only after confirming the branch is pushed
or safely merged, and only with user approval for unmerged work. Do not offer
cleanup while review fixes are still expected unless the user says local
iteration is done.

**PR review feedback returned:**
→ `addressing-pr-reviews` — fix CI first, then automated review comments, then
human review comments

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
- Council command (repo-local): [`commands/council.md`](../../commands/council.md)
