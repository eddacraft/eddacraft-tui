# Agent Guidelines

These conventions apply to all agents working in this project.

## Planning — Anvil Plan Spec (APS)

All multi-step work MUST use APS format. Read `plans/aps-rules.md` for the full
spec before writing or modifying any `.aps.md` file.

### Single source of truth

`plans/index.aps.md` is the canonical index of all modules. Do NOT create
separate module lists, summary files, or shadow indexes — they drift and cause
confusion.

### Key rules

- **Read before writing:** check `plans/index.aps.md` for active work and
  current status before starting any implementation
- **Modules:** `plans/modules/<module>.aps.md` (active),
  `plans/archive/modules/` (completed)
- **Work item IDs:** `PREFIX-NNN` (3-digit zero-padded)
- **Statuses:** Draft → Proposed → Ready → In Progress → Complete
- **Wave-based** parallel execution for independent work items
- **UK English** spelling in all plan text

### Keeping plans current

Agents MUST update APS state as they work — do not leave this for later:

1. **Before starting:** mark module status **In Progress** in the module file
2. **After completing a work item:** update its status (checkbox, Status field,
   or table row) in the module file
3. **After completing a work item:** update the done/total count in the module
   file's header table
4. **After all active items done:** update module status to **Complete**
5. **Update `plans/index.aps.md`** whenever a module's count or status changes
6. **Archive completed modules:** `git mv` to `plans/archive/modules/` and
   update the path in `index.aps.md`

Reference spec: <https://github.com/eddacraft/anvil-plan-spec>

## Architecture and Design Decisions

Before proposing new architecture, changing technology choices, or planning work
that touches system boundaries, check existing decisions first:

1. **Decision log:** `plans/decisions/DECISION-LOG.md` — condensed index of all
   ADRs with one-line summaries. Start here.
2. **Scope guard:** `docs/vision/anvil-scope-guard.md` — defines what Anvil is
   and isn't. Check before proposing new scope.
3. **Architecture overview:** `docs/architecture/overview.md` — design
   philosophy (planless-first, deterministic, composable, safety by default).
4. **Full ADRs:** `plans/decisions/NNN-*.md` — read the specific ADR when you
   need trade-off context beyond the one-line summary.

When introducing a new architectural decision, follow
`docs/guides/adr-process.md` and add the entry to the decision log.

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
- Branch name must reference the APS module: `fix/<module-slug>` or
  `feat/<module-slug>`
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
- Works through `plans/reviews/post-merge/` — verifies agent-runnable steps,
  flags human-required steps
- Sends notification for anything needing attention
- Logs to `plans/reviews/cleanup-log.md`

### Quick Reference

| Stage  | Agent/Command           | Skill                            |
| ------ | ----------------------- | -------------------------------- |
| Plan   | `anvil-plan-spec` agent | `aps-planning`                   |
| Branch | —                       | `docs/guides/worktree-policy.md` |
| Code   | `tdd-coach` agent       | `test-driven-development`        |
| Debug  | `debugger` agent        | `systematic-debugging`           |
| Review | `/council` command      | `code-review`                    |
| PR     | `/commit` command       | —                                |
| Verify | cleanup agent (cron)    | —                                |

Reference: `plans/aps-rules.md` · `docs/guides/branching-strategy.md` ·
`docs/guides/worktree-policy.md`

> Global workflow skill: `dev-workflow` in `joshuaboys/code-env` — canonical
> routing layer for all lifecycle stages.

## Feature Flags

When introducing or modifying feature flags, follow the governance rules in
`docs/guides/feature-flag-governance.md` and `plans/aps-rules.md`. Key points:

- Every flag needs `createdFor` linking to an APS work item
- `rollout` flags must have a sunset date (`expiryOrReviewDate`)
- Retirement follows: `active` → `retiring` → `retired` → delete
- Kill switch and entitlement flags fail closed on error

## Code Quality

- UK English spelling in plan text and documentation
- Clean, modular, functional code following the project's conventions
- Validate at system boundaries; trust internal code
- No secrets in code — use env vars or secret managers
