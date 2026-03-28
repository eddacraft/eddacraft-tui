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

## Scope Constraint

All contributions must align with Anvil's role as a real-time control layer for
safe software creation. Features that do not directly improve prevention,
validation, or enforcement of unsafe outcomes must not be introduced.

See: @docs/vision/anvil-scope-guard.md

## Code Quality

- UK English spelling in plan text and documentation
- Clean, modular, functional code following the project's conventions
- Validate at system boundaries; trust internal code
- No secrets in code — use env vars or secret managers
