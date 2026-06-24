---
name: using-git-worktrees
description: Use when starting feature work that needs isolation, or before executing implementation plans. Creates an isolated git worktree with safety verification and a clean test baseline.
---

# Using Git Worktrees

Set up an isolated workspace using this repo's Worktrunk-managed worktrees so multiple branches can be checked out at once without conflicting.

## Directory selection (priority order)

1. `.worktrees/` exists → use it
2. `worktrees/` exists → use it
3. Both exist → use `.worktrees/`
4. Neither → check project README/AGENTS.md for preference → ask user

## Safety check (project-local directories only)

Before creating a worktree in `.worktrees/` or `worktrees/`, verify the directory is gitignored:

```bash
git check-ignore -q .worktrees
```

If not ignored → add it to `.gitignore` and commit before proceeding.

Not needed for global paths (e.g. `~/.config/worktrees/`).

## Create worktree

```bash
wt switch --create <branch-name>
```

Branch names should follow the project's convention (`feat/`, `fix/`, `docs/`, `chore/`).

## Setup

Auto-detect and run the project's install step:

```bash
[ -f package.json ]      && pnpm install
[ -f Cargo.toml ]        && cargo build
[ -f requirements.txt ]  && pip install -r requirements.txt
[ -f go.mod ]            && go mod download
```

## Verify baseline

Run the test suite. If tests fail, report and ask whether to proceed.

## Report

```
Worktree ready at <path>
Tests passing (<N> tests, 0 failures)
Ready to implement <feature-name>
```

## Rules

- Never create a project-local worktree without verifying it's gitignored
- Never skip baseline test verification
- Never assume directory location — follow priority order
- Report failing baseline tests before proceeding

## Cleanup

When work is finished and the PR has merged, remove the worktree:

```bash
wt remove <branch-name>
```

For more on completing the work in a worktree, see the `finishing-a-branch` skill.
