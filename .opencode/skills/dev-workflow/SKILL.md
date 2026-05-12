---
name: dev-workflow
description:
  Use at the start of any development task to route to the correct skill, agent,
  and command for each lifecycle stage in this repo. Covers the full APS →
  branch → code → council → PR → cleanup loop, tuned to anvil's main-first
  cutover and risk-tiered Council tiers.
---

# Dev Workflow

Routing layer for the development lifecycle. Every task follows this sequence —
do not skip stages.

```
APS (Ready) → Branch → Code → Review → PR → Merged → [cleanup] → Released/Shipped
```

## Surface inventory

This skill is the **anvil project-local** snapshot. Most referenced skills and
agents are not vendored under `.opencode/` or `.claude/` here — they resolve
from the agent runtime (OpenCode native skills, Claude Code globals, or
`joshuaboys/code-env`).

| Surface                        | Repo-local                                                                                                                                                   | Global / external                                                                                                                                                                                                                           |
| ------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Skills (`.opencode/skills/`)   | `dev-workflow` (this skill)                                                                                                                                  | OpenCode-native skills loaded on demand via the `skill` tool                                                                                                                                                                                |
| Skills (`.claude/skills/`)     | `dev-workflow`, `planning-council`, `release`, `dependabot` (symlink)                                                                                        | `brainstorming`, `writing-plans`, `using-git-worktrees`, `test-driven-development`, `systematic-debugging`, `verification-before-completion`, `addressing-pr-reviews`, `finishing-a-branch`, `parallel-agents`, `council`, `commit`, others |
| Agents (`.claude/agents/`)     | `council-reviewer`, `adversarial-reviewer`, `kernel-maintainer`, `operations-reviewer`, `pragmatic-lead`, `anvil-plan-spec`, `plan-synthesizer`, `tdd-coach` | `debugger`, `autonomous`, others                                                                                                                                                                                                            |
| Commands (`.claude/commands/`) | `/council` (see [`.claude/commands/council.md`](../../../.claude/commands/council.md)), `/plan`, `/release`, others                                          | `/test`, `/debug`, `/delegate`, `/commit`                                                                                                                                                                                                   |

If a referenced skill or agent is not vendored locally, it is expected to be
available via the agent runtime — not vendored drift. CIB-002 is the open work
item for producing the definitive inventory.

## Stage Map

| Stage               | What                                 | Skill                               | Agent                                        | Command                                 |
| ------------------- | ------------------------------------ | ----------------------------------- | -------------------------------------------- | --------------------------------------- |
| **Idea / spec**     | Explore intent, design before code   | `brainstorming`                     | —                                            | —                                       |
| **Plan**            | Write implementation plan from spec  | `writing-plans`, `planning-council` | `anvil-plan-spec`                            | `/plan`                                 |
| **Branch**          | Create isolated worktree from `main` | `using-git-worktrees`               | —                                            | —                                       |
| **Code**            | TDD implementation                   | `test-driven-development`           | `tdd-coach`                                  | `/test`                                 |
| **Debug**           | Root cause analysis                  | `systematic-debugging`              | `debugger`                                   | `/debug`                                |
| **Verify**          | Evidence before completion claims    | `verification-before-completion`    | —                                            | —                                       |
| **Review (pre-PR)** | Risk-tiered Council                  | `council`                           | `council-reviewer` + role-mapped specialists | `/council [quick\|mini\|full] <target>` |
| **Finish**          | Commit, push, open PR                | `finishing-a-branch`                | —                                            | `/commit`                               |
| **Address review**  | Resolve PR feedback and CI failures  | `addressing-pr-reviews`             | —                                            | —                                       |
| **Parallelise**     | Independent tasks concurrently       | `parallel-agents`                   | `autonomous`                                 | `/delegate`                             |

## Rules

1. **Always start from APS.** Pick a `Ready` work item in `plans/index.aps.md`.
   Mark it `In Progress` before writing code.
2. **Branch from `main`.** Hotfixes also branch from `main` (or the latest good
   tag if `main` is unreleasable). Use the project naming convention (`feat/*`,
   `fix/*`, `docs/*`, `chore/*`).
3. **Council is the review surface.** Run `/council [quick|mini|full] <target>`
   before opening the PR. Default to `quick`; escalate to `mini` for
   cross-boundary / CI / release / security / workflow risk, and to `full` for
   branch / release-operating-model changes or high-risk design. See
   [`.claude/commands/council.md`](../../../.claude/commands/council.md) for the
   tier table. Address CRITICAL and MAJOR findings before push.
4. **Mark Merged on PR merge.** Not Complete — the cleanup agent advances
   `Merged → Released/Shipped → Complete` when release evidence confirms ship.
5. **Extract post-merge test plans.** Do not leave them in the PR description
   only. Write to `plans/reviews/post-merge/<branch-slug>.md`; the gitignore
   exception `!plans/reviews/post-merge/` keeps these tracked.
6. **Verify before claiming complete.** Evidence before assertions — use
   `verification-before-completion`.

## Decision Points

**Starting a new task:** → Check `plans/index.aps.md` for next Ready item →
`using-git-worktrees` (from `main`) → code

**Implementation unclear:** → `brainstorming` → `writing-plans` (or
`planning-council` for multi-persona design) → `using-git-worktrees` → code

**Tests failing unexpectedly:** → `systematic-debugging` before any other action

**About to commit:** → `verification-before-completion` gate → `/council`
(default `quick`; escalate to `mini`/`full` for risk) → `finishing-a-branch`

**PR review feedback returned:** → `addressing-pr-reviews` — fix CI first, then
walk each unresolved thread

**Multiple independent tasks:** → `parallel-agents` to dispatch subagents per
task

## APS Status Lifecycle

```
Draft → Proposed → Ready → In Progress → Merged → Released/Shipped → Complete
                              ↑              ↑              ↑              ↑
                        (you start)   (PR merged)   (release record)  (cleanup agent)
```

`Committed` is legacy wording for `Merged`. New APS text should prefer `Merged`
and `Released/Shipped`. Cleanup agent (`scripts/aps-cleanup.sh` where present)
auto-advances post-merge states when release evidence is recorded.

## Project References

- Branching strategy: `docs/guides/branching-strategy.md`
- Worktree policy: `docs/guides/worktree-policy.md`
- APS rules: `plans/aps-rules.md`
- Post-merge template: `plans/reviews/post-merge/TEMPLATE.md`
- Council command (repo-local):
  [`.claude/commands/council.md`](../../../.claude/commands/council.md)
- Parallel Claude version of this skill:
  [`.claude/skills/dev-workflow/SKILL.md`](../../../.claude/skills/dev-workflow/SKILL.md)
