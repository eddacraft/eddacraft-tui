---
name: dev-workflow
description:
  MANDATORY for any Anvil development, docs, config, planning, PR, review,
  debugging, release, or repository-maintenance task. Use before touching files,
  running implementation commands, committing, opening PRs, or declaring work
  complete. Routes to the correct skill, agent, and command for the full APS →
  branch → code → council → PR → cleanup → continuous-improvement loop.
---

# Dev Workflow

## Source And Variant

This is the Anvil vendored variant of the neutral EddaCraft skill at
`eddacraft-skills/skills/eddacraft/dev-workflow`. Keep the workflow contract
aligned with the neutral source, but preserve Anvil-specific APS, Worktrunk,
main-first, Council, and release-closeout rules here.

Routing layer for the development lifecycle. Invoke this skill first for every
non-trivial Anvil task, including documentation/configuration changes and review
remediation. Every task follows this sequence — do not skip stages.

```
APS Truth Gate → APS (Ready) → Worktrunk Branch → TDD Code → Review → PR → Merged → cleanup offer → Released/Shipped → continuous-improvement note
```

## Trigger Contract

Use this skill whenever the user asks to:

- implement, fix, refactor, test, debug, review, release, or document anything
  in this repository
- edit `.opencode/`, `.claude/`, `AGENTS.md`, `docs/**`, `plans/**`, source,
  tests, workflows, scripts, or package/crate metadata
- continue an existing branch, address PR comments, investigate CI, or prepare a
  commit/PR
- make a process, agent, skill, workflow, or repository-maintenance change

Do not wait for the user to say "use dev-workflow". If the task touches the
repository lifecycle, load this skill first, then route to the specific skill
for the current stage.

## Surface inventory

This skill is the **anvil project-local** snapshot. Most referenced skills and
agents are not vendored under `.opencode/` or `.claude/` here — they resolve
from the agent runtime (OpenCode native skills, Claude Code globals, or
`joshuaboys/code-env`).

| Surface                        | Repo-local                                                                                                                                                   | Global / external                                                                                                                                                                                                                                       |
| ------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Skills (`.opencode/skills/`)   | `dev-workflow` (this skill), `addressing-pr-reviews`, `planning-workflow`, `aps-planning`, `test-driven-development`                                         | Other OpenCode-native skills loaded on demand via the `skill` tool                                                                                                                                                                                      |
| Skills (`.claude/skills/`)     | `dev-workflow`, `addressing-pr-reviews`, `planning-council`, `release`, `dependabot` (symlink)                                                               | `planning-workflow`, `aps-planning`, `test-driven-development`, `brainstorming`, `writing-plans`, `using-git-worktrees`, `systematic-debugging`, `verification-before-completion`, `finishing-a-branch`, `parallel-agents`, `council`, `commit`, others |
| Agents (`.claude/agents/`)     | `council-reviewer`, `adversarial-reviewer`, `kernel-maintainer`, `operations-reviewer`, `pragmatic-lead`, `anvil-plan-spec`, `plan-synthesizer`, `tdd-coach` | `debugger`, `autonomous`, others                                                                                                                                                                                                                        |
| Commands (`.claude/commands/`) | `/council` (see [`.claude/commands/council.md`](../../../.claude/commands/council.md)), `/plan`, `/release`, others                                          | `/test`, `/debug`, `/delegate`, `/commit`                                                                                                                                                                                                               |

If a referenced skill or agent is not vendored locally, it is expected to be
available via the agent runtime — not vendored drift. CIB-002 is the open work
item for producing the definitive inventory.

## Stage Map

| Stage               | What                                  | Skill                                                                    | Agent                                        | Command                                 |
| ------------------- | ------------------------------------- | ------------------------------------------------------------------------ | -------------------------------------------- | --------------------------------------- |
| **Idea / spec**     | Explore intent, design before code    | `brainstorming`                                                          | —                                            | —                                       |
| **Plan / APS gate** | Plan intent and validate APS truth    | `planning-workflow`, `aps-planning`, `writing-plans`, `planning-council` | `anvil-plan-spec`                            | `/plan`                                 |
| **Branch**          | Create Worktrunk worktree from `main` | `using-git-worktrees`                                                    | —                                            | `wt switch --create <branch>`           |
| **Code**            | TDD implementation                    | `test-driven-development`                                                | `tdd-coach`                                  | `/test`                                 |
| **Debug**           | Root cause analysis                   | `systematic-debugging`                                                   | `debugger`                                   | `/debug`                                |
| **Verify**          | Evidence before completion claims     | `verification-before-completion`                                         | —                                            | —                                       |
| **Review (pre-PR)** | Risk-tiered Council                   | `council`                                                                | `council-reviewer` + role-mapped specialists | `/council [quick\|mini\|full] <target>` |
| **Finish**          | Commit, push, open PR, offer cleanup  | `finishing-a-branch`                                                     | —                                            | `/commit`                               |
| **Address review**  | Resolve PR feedback and CI failures   | `addressing-pr-reviews`                                                  | —                                            | —                                       |
| **Parallelise**     | Independent tasks concurrently        | `parallel-agents`                                                        | `autonomous`                                 | `/delegate`                             |

## Rules

1. **Always start from planning truth.** If the requested work is not already a
   clearly valid `Ready` or `In Progress` APS item, invoke `planning-workflow`.
   Before branch or code, invoke `aps-planning` for APS Truth Validation. If the
   gate returns `needs-plan-update` or `blocked`, update APS or resolve the
   blocker before implementation. Mark the work item `In Progress` before
   writing code.
2. **Use Worktrunk for task branches.** Create task branches with
   `wt switch --create <branch>` unless the user explicitly asks to continue in
   the current worktree. Hotfixes also branch from `main` (or the latest good
   tag if `main` is unreleasable). Use the project naming convention (`feat/*`,
   `fix/*`, `docs/*`, `chore/*`).
3. **Code through TDD.** Invoke `test-driven-development` for the Code stage.
   Write or update the smallest failing test first, prove the red state, make it
   pass with the smallest correct implementation, then refactor while keeping
   targeted tests green. If the work truly cannot be tested first, record why in
   the APS item or PR test plan.
4. **Council is the review surface.** Run `/council [quick|mini|full] <target>`
   before opening the PR. Default to `quick`; escalate to `mini` for
   cross-boundary / CI / release / security / workflow risk, and to `full` for
   branch / release-operating-model changes or high-risk design. See
   [`.claude/commands/council.md`](../../../.claude/commands/council.md) for the
   tier table. Address CRITICAL and MAJOR findings before push.
5. **Mark Merged on PR merge.** Not Complete — the cleanup agent advances
   `Merged → Released/Shipped → Complete` when release evidence confirms ship.
6. **Extract post-merge test plans.** Do not leave them in the PR description
   only. Write to `plans/reviews/post-merge/<branch-slug>.md`; the gitignore
   exception `!plans/reviews/post-merge/` keeps these tracked.
7. **Verify before claiming complete.** Evidence before assertions — use
   `verification-before-completion`.
8. **Always run post-PR review remediation.** After opening a PR, wait up to 10
   minutes for Copilot and other automated reviewers to complete or time out,
   then run `addressing-pr-reviews` even when no automated review comments were
   left. That skill also catches late CI failures. Do not mention or tag bots to
   request a review unless the user explicitly asks.
9. **Use the review closure loop.** `addressing-pr-reviews` must re-inventory
   CI, unresolved review threads, and mergeability after every
   push/rebase/thread resolution. Fix the highest-priority blocker first: CI,
   conflicts/stale base, automated review threads, then human review threads. Do
   not claim the PR is ready after fixing only one blocker class.
10. **Prefer rebase before merge.** When a PR branch needs to catch up to `main`
    or resolve merge conflicts, rebase it onto latest `main` and push with
    `--force-with-lease` unless the user explicitly asks for a merge commit.
11. **Offer branch/worktree cleanup at the end.** After a PR is opened, merged,
    abandoned, or paused with no near-term action, ask whether to run
    `wt remove` for the Worktrunk worktree and local branch if it is safe. Never
    delete a branch that is unmerged, unpushed, or still needed without explicit
    user approval.
12. **Run local CI-equivalent gates before opening the PR.** The repo-mandated
    validation commands in `CLAUDE.md`
    (`pnpm format:check && pnpm lint:check && pnpm typecheck && pnpm test`;
    `cargo test --workspace`) must run green locally before push. CI is a
    backstop, not the primary signal — relying on CI alone risks blocking `main`
    for every other PR if a check fails post-merge. Tick the test-plan
    checkboxes only after the command actually ran, not aspirationally.
13. **Cargo.lock and ACKNOWLEDGEMENTS.md are one atomic change.** If a PR
    touches `Cargo.lock`, run
    `bash tools/starters/acknowledgements/generate-acknowledgements.sh` and
    include the resulting `ACKNOWLEDGEMENTS.md` diff in the same PR. The
    `Acknowledgements freshness` workflow (`.github/workflows/rust.yml`) blocks
    `main` for every subsequent PR if this is split.
14. **Keep bookkeeping PRs single-purpose.** APS status updates, index counter
    adjustments, and runbook fixes ship as standalone PRs. Do not bundle them
    with dep updates (`Cargo.lock`, `package.json`), code refactors, or feature
    work — a CI failure on the heavyweight change blocks the trivial bookkeeping
    for hours, and the broader review surface erodes the "trivially mergeable"
    property bookkeeping PRs depend on.
15. **Surface hook errors to the user.** If `PreToolUse:Bash hook error` or
    similar hook-validation messages recur in tool output, flag them in a
    one-line note. They indicate a misconfigured Claude Code / opencode hook
    (often invalid `decision` values like `"allow"` / `"ask"` in the legacy
    schema) that may be silently allowing or blocking commands. Do not just keep
    running.
16. **Write a continuous-improvement note.** Before the final response on any
    non-trivial task, append one compact entry to
    `plans/reviews/continuous-improvement-log.md`. If the user or task contract
    explicitly forbids writes, do not edit the log; say the CI note was skipped
    because the task was read-only. Keep entries factual and short; raw
    observations belong in the log, recurring or executable improvements should
    be promoted to `plans/modules/continuous-improvement-backlog.aps.md`.

## Decision Points

**Starting a new task:** → `planning-workflow` if no validated Ready item exists
→ `aps-planning` APS Truth Validation → check `plans/index.aps.md` for next
Ready item → mark `In Progress` when authorised → `using-git-worktrees` via
`wt switch --create <branch>` from `main` → `test-driven-development` → code

**Implementation unclear or APS truth gate finds ambiguous scope:** →
`planning-workflow` → `brainstorming` when required → `writing-plans` (or
`planning-council` for multi-persona design) → `aps-planning` validation →
`using-git-worktrees` → `test-driven-development` → code

**Tests failing unexpectedly:** → `systematic-debugging` before any other action

**About to commit or open PR:** → `verification-before-completion` gate →
`/council` (default `quick`; escalate to `mini`/`full` for risk) →
`finishing-a-branch`

**After PR open:** → wait up to 10 minutes for Copilot and automated reviewers
to complete or time out → always run `addressing-pr-reviews`, even with no bot
comments, because it also catches late CI failures; fix CI first, then automated
review comments, then human review comments inside the closure loop; after every
push/rebase/thread resolution, re-check CI, unresolved threads, and
mergeability; do not tag bots unless the user explicitly asks

**After PR merge / abandon / pause:** → offer `wt remove`; perform cleanup only
after confirming the branch is pushed or safely merged, and only with user
approval for unmerged work. Do not offer cleanup while review fixes are still
expected unless the user says local iteration is done.

**PR review feedback returned:** → `addressing-pr-reviews` — run the closure
loop until CI, unresolved review threads, and mergeability are clean in the same
pass, or stop with evidence for the remaining blocker

**PR branch behind or conflicted:** → rebase onto latest `main`, resolve
conflicts, validate, then push with `--force-with-lease`; use a merge commit
only when the user explicitly asks

**Multiple independent tasks:** → `parallel-agents` to dispatch subagents per
task

**Before final response on non-trivial work:** → append a compact
continuous-improvement note to `plans/reviews/continuous-improvement-log.md` →
if writes are explicitly forbidden, skip the edit and report that skip → promote
only concrete follow-up work to `CIB-NNN` if it has an observable outcome and
validation path

## Continuous Improvement Closeout

Append exactly one lightweight note per non-trivial session or meaningful failed
attempt. Do not write long retrospectives. Prefer this compact Markdown shape
because it is quick for agents, renders visibly, and is easy to search:

```md
### YYYY-MM-DD — opencode|claude|other

- **Task:** ...
- **Outcome:** ...
- **Worked:** ...
- **Failed:** ...
- **Friction:** ...
- **Improvement:** ...
- **Follow-up:** ...
```

Rules:

- If there is no useful learning, write `improvement: none` rather than forcing
  a suggestion.
- Do not include secrets, private tokens, or raw environment dumps.
- Keep command output out of the log unless the exact command is the lesson.
- Promote repeated friction or executable fixes to the CIB APS module; do not
  let the log become a second backlog.
- Respect explicit read-only/no-write task contracts; in that case, mention the
  skipped CI note in the final response instead of editing the log.

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
- Continuous-improvement log: `plans/reviews/continuous-improvement-log.md`
- Continuous-improvement backlog:
  `plans/modules/continuous-improvement-backlog.aps.md`
- Council command (repo-local):
  [`.claude/commands/council.md`](../../../.claude/commands/council.md)
- Parallel Claude version of this skill:
  [`.claude/skills/dev-workflow/SKILL.md`](../../../.claude/skills/dev-workflow/SKILL.md)
