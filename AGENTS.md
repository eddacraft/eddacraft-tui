# Agent Guidelines

These conventions apply to all agents working in this project.

## Planning — Anvil Plan Spec (APS)

All multi-step work MUST use APS format:

- Master plan: `plans/index.aps.md`
- Modules: `plans/modules/<module>.aps.md`
- Work item IDs: `PREFIX-NNN` (3-digit zero-padded)
- Module statuses: Draft > Proposed > Ready > In Progress > Complete
- Wave-based parallel execution for independent work items
- Archive completed modules to `plans/archive/`

Before starting implementation, check `plans/index.aps.md` for active work items
and current status. Update task status as you progress.

Reference spec: <https://github.com/EddaCraft/anvil-plan-spec>

## Repository Operations — gx

Use `gx` for all repository management. Never use raw `git clone`.

| Task              | Command                  |
| ----------------- | ------------------------ |
| Clone a repo      | `gx clone <url-or-name>` |
| Jump to a project | `gx <name>`              |
| Scaffold configs  | `gx init`                |
| List projects     | `gx list`                |

All cloned repos land in `~/Projects/src/` automatically.

## Commit Format

Conventional commits with imperative mood, lowercase, no period:

```
<type>(<scope>): <subject>
```

Types: `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `chore`, `ci`

The `Authored-By:` trailer is added automatically — do not add it manually.

## Development Lifecycle

Every piece of work follows this sequence. Agents must not skip stages.

```
APS (Ready) → Branch → Code → Council → PR → Committed → [cleanup] → Complete
```

### 1. Start from APS
- Read `plans/index.aps.md` — pick the next **Ready** work item
- Mark module **In Progress** before touching any code
- Branch name must reference the APS module: `fix/<module-slug>` or `feat/<module-slug>`
- Create branch from `dev` (hotfixes from `main`)

### 2. Code
- Work in a disposable worktree — see `docs/guides/worktree-policy.md`
- Follow TDD: tests before implementation
- Run `pnpm typecheck && pnpm test` before committing
- Commit with conventional format referencing APS ID where applicable

### 3. Council Review (before PR)
- Run `/council` before opening any non-trivial PR
- Minimum: council-reviewer + adversarial-reviewer
- Address CRITICAL and MAJOR findings before opening PR

### 4. Open PR
- Target `dev` for normal work, `main` only for hotfixes
- If the PR has post-merge verification steps:
  - Extract them to `plans/reviews/post-merge/<branch-slug>.md`
  - Reference the APS module ID in the file
  - Do NOT leave test plans only in the PR description
- Mark module status **Committed** in the `.aps.md` file

### 5. Cleanup Agent (automated)
- Runs on schedule — checks all **Committed** modules
- Verifies branch merged + CI green → advances to **Complete**
- Works through `plans/reviews/post-merge/` — verifies agent-runnable steps, flags human-required steps
- Sends notification for anything needing attention
- Logs to `plans/reviews/cleanup-log.md`

### Quick Reference

| Stage | Agent/Command | Skill |
|---|---|---|
| Plan | `anvil-plan-spec` agent | `aps-planning` |
| Branch | — | `docs/guides/worktree-policy.md` |
| Code | `tdd-coach` agent | `test-driven-development` |
| Debug | `debugger` agent | `systematic-debugging` |
| Review | `/council` command | `code-review` |
| PR | `/commit` command | — |
| Verify | cleanup agent (cron) | — |

Reference: `plans/aps-rules.md` · `docs/guides/branching-strategy.md` · `docs/guides/worktree-policy.md`

> Global workflow skill: `dev-workflow` in `joshuaboys/code-env` — canonical routing layer for all lifecycle stages.

## Code Quality

- UK English spelling in plan text and documentation
- Clean, modular, functional code following the project's conventions
- Validate at system boundaries; trust internal code
- No secrets in code — use env vars or secret managers
