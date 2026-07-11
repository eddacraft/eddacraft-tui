# Agent Guidelines

These conventions apply to all agents working in this repository.

For orientation, vocabulary, and repository layout, read
[`CONTEXT.md`](CONTEXT.md). This file is the shared behaviour contract;
`CONTEXT.md` is the map.

Tool-specific adapters live outside this contract. Claude Code-specific hooks,
commands, skills, MCP/config notes, and Claude-only gotchas live in `CLAUDE.md`.

## Operating Rules

- Use UK English in plans and documentation.
- Do not leave inline deferred-work markers. Track follow-up work in APS or
  GitHub Issues.
- Do not create shadow indexes, duplicate module lists, or source-of-truth
  summaries outside the owning document.
- Prefer links to authoritative docs over restating procedure details here.
- Validate at system boundaries; trust internal code.
- Do not add secrets to code, docs, plans, config, examples, or logs.
- Never revert or overwrite another person's uncommitted work unless explicitly
  asked.

## Workflow

All multi-step work uses APS.

Before implementation:

1. Read `plans/index.aps.md`.
2. Read the relevant module under `plans/modules/`.
3. Read `plans/project-context.md` for anvil-specific workflow rules.
4. Mark work `In Progress` where applicable.

Standard lifecycle:

```text
APS Ready -> Worktrunk branch -> Code -> Council -> PR -> Merged -> cleanup offer -> Released/Shipped -> Complete
```

Use Worktrunk-managed worktrees from `main` for task branches. See
`docs/guides/branching-strategy.md` and `docs/guides/worktree-policy.md`.

For APS vocabulary, status extensions, progress counters, release metadata,
feature flags, commit format, and local validation policy, read
`plans/project-context.md`. Continuous-improvement closeout (pending queue + harvest) is also
defined there and in `docs/guides/continuous-improvement-log.md`. For repository-management commands and local setup,
read `docs/guides/repository-operations.md`.

## Architecture And Scope

Before changing architecture, technology choices, public contracts, or system
boundaries, read:

- `plans/decisions/DECISION-LOG.md`
- `docs/vision/anvil-scope-guard.md`
- `docs/architecture/overview.md`

Durable architectural decisions require an ADR using
`docs/guides/adr-process.md` and an entry in the decision log.

## Documentation Changes

Documentation is operational context, not prose cleanup.

When changing `docs/**`, `plans/**`, `README.md`, `CONTRIBUTING.md`,
`AGENTS.md`, `CLAUDE.md`, or package/crate READMEs, follow:

- `docs/guides/documentation-governance.md`
- `plans/project-context.md`

Include a short `Docs Closeout` note in the final response.

## Validation

Prefer the narrowest relevant validation first.

Common checks:

- `pnpm validate:changed`
- `pnpm validate:staged`
- `pnpm format:check`
- `pnpm lint:check`
- `pnpm typecheck`
- `pnpm test`
- `pnpm docs:check`
- `pnpm aps:active-lint`
- `pnpm aps:index:check`

For full confidence, run `pnpm validate:full`.

For test selection and stack-specific commands, read `docs/guides/testing.md`.

## Agent Surfaces

Repo-local and global skills, agents, and commands are inventoried in
`docs/guides/agent-surface-inventory.md`.

Use the relevant skill, command, or agent surface instead of copying procedure
text into this file.

Local directory conventions belong in the nearest local `AGENTS.md`; do not add
nested `CONTEXT.md` files.
