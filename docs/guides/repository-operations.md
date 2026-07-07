# Repository Operations

| Type  | Authority     | Owner | Status | Freshness                                                                           |
| ----- | ------------- | ----- | ------ | ----------------------------------------------------------------------------------- |
| Guide | Authoritative | AICON | Live   | Last reviewed 2026-07-07 against `AGENTS.md`, `plans/project-context.md`, AICON-003 |

| Upstream                                                                                                       | Downstream                                            |
| -------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------- |
| `AGENTS.md`, `plans/project-context.md`, `docs/guides/worktree-policy.md`, `docs/guides/branching-strategy.md` | `docs/guides/README.md`, `AGENTS.md`, agent workflows |

This guide owns local repository-management conventions for Anvil agents and
contributors. `AGENTS.md` stays lean and links here instead of carrying command
tables.

## Repository Manager

Use `gx` for local repository management. Do not use raw `git clone` for normal
Anvil setup because `gx` preserves the expected project layout and local tooling
assumptions.

| Task              | Command                  |
| ----------------- | ------------------------ |
| Clone a repo      | `gx clone <url-or-name>` |
| Jump to a project | `gx <name>`              |
| Scaffold configs  | `gx init`                |
| List projects     | `gx list`                |

## Branch And Worktree Boundaries

Use Worktrunk-managed worktrees for implementation work. Create task branches
from `main` and follow the branch naming and cleanup rules in
[`branching-strategy.md`](branching-strategy.md) and
[`worktree-policy.md`](worktree-policy.md).

Do not reuse a feature worktree for unrelated work unless the operator
explicitly asks you to continue there. When a PR is opened, merged, abandoned,
or paused with no near-term action, offer safe local worktree cleanup instead of
deleting branches autonomously.

## Local Setup Expectations

- Keep repository state in the expected `gx` project tree.
- Prefer project-local scripts and package-manager commands over ad-hoc shell
  aliases.
- Do not add secrets to repository configuration, examples, plans, docs, or
  logs.
- Treat `.env`, auth, token, and local credential files as secret-bearing even
  if they look empty.
- Preserve another person's uncommitted work; never reset, overwrite, or clean
  it without explicit approval.
