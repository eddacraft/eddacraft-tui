# Agent Guidelines

These conventions apply to all agents working in this repository.

For orientation, vocabulary, and repository layout, read
[`CONTEXT.md`](CONTEXT.md). This file is the shared behaviour contract;
`CONTEXT.md` is the map.

Tool-specific adapters live outside this contract. Claude Code keeps a **thin**
`CLAUDE.md` (import this contract + Claude-only notes). Shared skills, agents,
and commands are inventoried in `docs/guides/agent-surface-inventory.md` — do
not re-describe them in adapters.

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
- Treat administrator and policy-bypass operations as a separate authority
  boundary. A request to merge, rebase merge, auto-merge, or "merge on green"
  never authorises `gh pr merge --admin`, a branch-protection bypass, review
  dismissal, or any equivalent override.
- Use an administrator or policy override only when the operator explicitly
  authorises that exact override for that exact pull request in the current
  request. If a normal merge is blocked, diagnose the specific policy gate and
  address it or leave auto-merge waiting; never bypass pending or failing checks
  or unresolved review conversations.

## Workflow

All multi-step work uses APS.

Before implementation:

1. Read `plans/index.aps.md`.
2. Read the relevant module under `plans/modules/`.
3. Read `plans/project-context.md` for anvil-specific workflow rules.
4. Mark work `In Progress` only on **exclusive** modules. Do **not** edit shared
   multi-writer APS modules (for example CIB) from feature PRs — see
   `plans/project-context.md#keeping-plans-current`.

Standard lifecycle:

```text
APS Ready -> Worktrunk branch -> Code -> Council -> PR -> Merged -> cleanup offer -> Released/Shipped -> Complete
```

Use Worktrunk-managed worktrees from `main` for task branches. See
`docs/guides/branching-strategy.md` and `docs/guides/worktree-policy.md`.

For APS vocabulary, status extensions, progress counters, release metadata,
feature flags, commit format, and local validation policy, read
`plans/project-context.md`. Continuous-improvement closeout (pending queue +
harvest) is also defined there and in
`docs/guides/continuous-improvement-log.md`. For repository-management commands
and local setup, read `docs/guides/repository-operations.md`.

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

Code and contract changes should review documentation and diagram impact in the
same change when they match any trigger in the authoritative
[change-impact review](docs/guides/documentation-governance.md#change-impact-review).
That authority defines the triggers, exemptions, and expected disposition; the
review remains advisory until DOCRB-009.

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

For full primary-CLI crate validation, run
`cargo test -p eddacraft-anvil --no-fail-fast`. The `--no-fail-fast` flag is
required so an earlier test-binary failure cannot hide integration-test
failures.

For full confidence, run `pnpm validate:full`.

For test selection and stack-specific commands, read `docs/guides/testing.md`.

## Anvil Developer Functions

This repository is anvil-enabled. When the anvil MCP tools are available, prefer
them over blind file reads and unchecked writes:

- Use the graph-context tools (`anvil_status`, `anvil_search_symbols`,
  `anvil_symbol_context`, `anvil_find_callers`, `anvil_find_dependents`,
  `anvil_impact_of_change`, `anvil_affected_tests`, `anvil_query_boundary`) to
  understand code before reading whole files. They are bounded, deterministic,
  and never block.
- Call `anvil_validate_write` before applying a file write, or
  `anvil_apply_patch` when applying a unified diff. Honour a `block` decision;
  surface `warn` diagnostics and continue.

If the tools are not wired into your harness, fall back to ordinary file reads
and note that anvil's developer functions were unavailable; do not stall. For
procedure use the `anvil-developer-functions` skill; for setup, `anvil check`,
`anvil gate`, watch mode, and CI use `using-anvil`.

## Agent Surfaces

Repo-local and global skills, agents, and commands are inventoried in
`docs/guides/agent-surface-inventory.md`.

Use the relevant skill, command, or agent surface instead of copying procedure
text into this file.

Local directory conventions belong in the nearest local `AGENTS.md`; do not add
nested `CONTEXT.md` files.
