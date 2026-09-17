# Agent Guidelines

Shared behaviour contract for all agents. Map and vocabulary:
[`CONTEXT.md`](CONTEXT.md). Skills, agents, and commands:
`docs/guides/agent-surface-inventory.md` — do not re-describe them in adapters.
Claude Code keeps a thin `CLAUDE.md` that imports this file.

Local directory conventions belong in the nearest local `AGENTS.md`; do not add
nested `CONTEXT.md` files.

## Start Here

- This repository uses the published `anvil` CLI by default. The rolling
  `anvil-main` channel is an optional dogfood path; see the
  [rolling-main runbook](docs/runbooks/anvil-home-side-by-side.md).

## Operating Rules

- Use UK English in plans and documentation.
- Use `eddacraft` and `anvil` lowercase in prose when naming the entities or
  products, including headings. Preserve exact casing only in code identifiers,
  external titles, quotations, and legal names. Correct existing prose when it
  is materially touched; do not create broad casing-only rewrites.
- Do not leave inline deferred-work markers. Track follow-up work in APS or
  GitHub Issues.
- Do not create shadow indexes, duplicate module lists, or source-of-truth
  summaries outside the owning document.
- Prefer links to authoritative docs over restating procedure details here.
- Validate at system boundaries; trust internal code.
- Do not add secrets to code, docs, plans, config, examples, or logs.
- Before the final response on non-trivial work, run
  `pnpm ci-log:append -- --task "<summary>"` (pending by default).
  `Improvement: none` is valid. Do not skip it because the note looks unrelated
  to the feature PR — pending is PR-independent (CIB-191). Pending notes live
  under the git common dir and stay invisible in feature-PR diffs until
  `pnpm ci-log:harvest` on a bookkeeping branch; check `pnpm ci-log:status`, not
  the tracked log. `agentic-loop` names docs-workflow closeout, not this step.
  Full form: `docs/guides/continuous-improvement-log.md`.
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
5. When starting an APS work item, open or reuse a private GitHub claim issue.
   See `plans/project-context.md#work-item-claim-issues`.

Standard lifecycle:

```text
APS Ready -> claim issue -> Worktrunk branch -> /dev-loop -> Council -> PR -> Merged -> cleanup offer -> Released/Shipped -> Complete
```

`/dev-loop` is the implementation orchestrator. Skills are vendored by
`eddaskills sync` (`eddaskills.toml`). Do not depend on a user-global `code-env`
or `~/.claude/skills` copy for the loop.

Use Worktrunk-managed worktrees from `main`. See
`docs/guides/branching-strategy.md` and `docs/guides/worktree-policy.md`.

APS vocabulary, status extensions, progress counters, release metadata, feature
flags, commit format, local validation, and continuous-improvement harvest:
`plans/project-context.md` and `docs/guides/continuous-improvement-log.md`.
Repository-management commands and local setup:
`docs/guides/repository-operations.md`.

## Architecture And Scope

Before changing architecture, technology choices, public contracts, or system
boundaries, read:

- `plans/decisions/DECISION-LOG.md`
- `docs/vision/anvil-scope-guard.md`
- `docs/architecture/overview.md`

Durable architectural decisions require an ADR using
`docs/guides/adr-process.md` and an entry in the decision log.

## Documentation Changes

Documentation is operational context, not prose cleanup. When changing
`docs/**`, `plans/**`, `README.md`, `CONTRIBUTING.md`, `AGENTS.md`, `CLAUDE.md`,
or package/crate READMEs, follow `docs/guides/documentation-governance.md` and
`plans/project-context.md`.

Code and contract changes must review documentation and diagram impact in the
same change when they match any trigger in the
[change-impact review](docs/guides/documentation-governance.md#change-impact-review).

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

For full confidence, run `pnpm validate:full`. Stack-specific selection:
`docs/guides/testing.md`.

## Anvil Developer Functions

This repository is anvil-enabled. When the anvil MCP tools are available, prefer
them over blind file reads and unchecked writes:

- Graph-context tools (`anvil_status`, `anvil_search_symbols`,
  `anvil_symbol_context`, `anvil_find_callers`, `anvil_find_dependents`,
  `anvil_impact_of_change`, `anvil_affected_tests`, `anvil_query_boundary`)
  before reading whole files. They are bounded, deterministic, and never block.
- `anvil_validate_write` before a file write, or `anvil_apply_patch` for a
  unified diff. Honour a `block` decision; surface `warn` diagnostics and
  continue.

If the tools are not wired into your harness, fall back to ordinary file reads
and note that anvil's developer functions were unavailable; do not stall.
Procedure: `anvil-developer-functions` skill. Setup, `anvil check`,
`anvil gate`, watch mode, and CI: `using-anvil`.
