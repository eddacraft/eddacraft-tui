---
name: dev-workflow
description:
  MANDATORY for any Anvil development, docs, config, planning, PR, review,
  debugging, release, or repository-maintenance task in Codex. Use before
  touching files, running implementation commands, committing, opening PRs, or
  declaring work complete. Routes Codex through the APS, Worktrunk, code,
  Council, PR, review-remediation, cleanup, and continuous-improvement loop.
---

# Dev Workflow

## Source And Variant

This is the Codex-facing Anvil workflow skill. Keep it aligned with the
repo-local Claude and OpenCode variants:

- `.claude/skills/dev-workflow/SKILL.md`
- `.opencode/skills/dev-workflow/SKILL.md`

Use this skill first for every non-trivial Anvil task, including documentation,
configuration, review remediation, debugging, and repository maintenance.

```text
APS Truth Gate -> APS (Ready) -> Worktrunk Branch -> TDD Code -> Review -> PR -> Merged -> cleanup offer -> Released/Shipped -> continuous-improvement note
```

## Trigger Contract

Use this skill whenever the user asks Codex to:

- implement, fix, refactor, test, debug, review, release, or document anything
  in this repository
- edit `.codex/`, `.opencode/`, `.claude/`, `AGENTS.md`, `docs/**`, `plans/**`,
  source, tests, workflows, scripts, or package/crate metadata
- continue an existing branch, address PR comments, investigate CI, or prepare a
  commit/PR
- make a process, agent, skill, workflow, or repository-maintenance change

Do not wait for the user to say "use dev-workflow". If the task touches the
repository lifecycle, load this skill first, then route to the current stage.

## Stage Map

| Stage          | What                                                             | Codex action                                                                                                                                                |
| -------------- | ---------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------- |
| APS truth gate | Confirm the request is backed by a Ready or In Progress APS item | Read `plans/index.aps.md`, `plans/aps-rules.md`, and `plans/project-context.md`; update APS before implementation if needed                                 |
| Branch         | Create an isolated task worktree from `main`                     | `wt switch --create <branch>` using `feat/`, `fix/`, `docs/`, or `chore/`                                                                                   |
| Code           | Implement with the smallest useful validation loop               | Prefer test-first changes; record why TDD is not applicable for docs/config-only work                                                                       |
| Verify         | Prove the actual changed surface                                 | Run focused checks first, then broader repo gates when risk justifies them                                                                                  |
| Review         | Run risk-tiered Council before non-trivial PRs                   | `/council quick <target>` by default; escalate to `mini` or `full` for cross-boundary, CI, release, security, workflow, branch, or high-risk design changes |
| Finish         | Commit, push, open PR, and offer cleanup                         | Use conventional commits; target `main`; do not open empty or duplicate PRs                                                                                 |
| Address review | Resolve CI, review threads, and mergeability in one closure loop | Use `addressing-pr-reviews` after PR creation or when review feedback appears                                                                               |
| Cleanup        | Remove local worktrees only when safe                            | Offer `wt remove` after PR open, merge, abandon, or pause; ask before deleting unmerged, unpushed, or dirty work                                            |

## Rules

1. Always start from planning truth. If no suitable Ready or In Progress APS
   item exists, add or update one before implementation and mark it In Progress
   before substantive edits.
2. Use Worktrunk-managed worktrees unless the user explicitly asks to continue
   in the current worktree. Hotfixes also branch from `main` unless release
   context says otherwise.
3. Respect the repo's dirty state. Never revert unrelated user changes. If an
   unrelated dirty file is present, leave it alone and work in the task
   worktree.
4. Follow TDD for source changes. For docs/config-only work, use schema, lint,
   formatting, link, or manual validation instead of inventing irrelevant tests.
5. Run the repo-mandated gates before PR publication when practical:
   `pnpm format:check`, `pnpm lint:check`, `pnpm typecheck`, `pnpm test`, and
   `cargo test --workspace`. For narrow docs/config changes, run the relevant
   subset and state any skipped full gates.
6. Run Council before opening any non-trivial PR. Address CRITICAL and MAJOR
   findings before push.
7. After opening a PR, wait up to 10 minutes for automated reviewers to complete
   or time out, then run the PR remediation loop. Fix CI first, then automated
   review threads, then human review threads.
8. Keep bookkeeping PRs single-purpose. APS status updates, inventory changes,
   config changes, and runbook fixes should not be bundled with dependency
   updates or feature refactors unless they are required for the same outcome.
9. For documentation-affecting work, complete docs closeout: classify changed
   docs, update indexes/inventories, mark unresolved drift in APS, run relevant
   validation, and include a `Docs Closeout` note in the final response.
10. Before the final response on non-trivial work, append one compact entry to
    `plans/reviews/continuous-improvement-log.md` unless the task is explicitly
    read-only.

## Codex Config Expectations

The repo-local `.codex/config.toml` is intentionally permissive enough for the
Anvil workflow while avoiding full machine write access:

- `approval_policy = "on-request"` lets Codex request elevation for operations
  that genuinely need it.
- `approvals_reviewer = "auto_review"` reduces interruption for low-risk
  approvals while still reviewing requested actions.
- `sandbox_mode = "workspace-write"` keeps writes bounded to declared roots.
- `sandbox_workspace_write.network_access = true` permits dependency, GitHub,
  and documentation lookups needed by the workflow.
- `sandbox_workspace_write.writable_roots` includes `~/Projects/src` so
  Worktrunk can create sibling worktrees and Codex can work in them.

Do not switch this project to `danger-full-access` unless the user explicitly
asks and accepts the risk for that session.

## Project References

- `AGENTS.md`
- `plans/index.aps.md`
- `plans/aps-rules.md`
- `plans/project-context.md`
- `docs/guides/agent-surface-inventory.md`
- `docs/guides/documentation-governance.md`
- `.codex/skills/addressing-pr-reviews/SKILL.md`
