---
name: aps-planning
description: >-
  Anvil APS awareness, truth validation, and reconciliation. Owns APS status and
  drift checks after planning-workflow has mapped intent to work, and before
  dev-workflow starts branch/code.
---

# APS Planning Skill

## Source And Variant

This is the Anvil vendored variant of the neutral EddaCraft skill at
`eddacraft-skills/skills/eddacraft/aps-planning`. Keep the APS truth-validation
contract aligned, but preserve Anvil-specific status vocabulary, index rules,
Worktrunk/main-first lifecycle, and release/documentation closeout here.

## OpenCode Surface

Use OpenCode tools directly for APS reads/edits and the `task` tool with
`.opencode/agents/anvil-plan-spec.md` for non-trivial APS rewrites. If a
workflow reference mentions a Claude slash command, translate it to the
equivalent OpenCode skill/tool flow.

## Activation Guard

If `plans/index.aps.md` does not exist, do nothing. This skill is inert outside
APS projects.

## Anvil Authorities

Before plan edits or non-trivial implementation, read these files when present:

- `AGENTS.md`
- `plans/aps-rules.md`
- `plans/index.aps.md`
- Relevant `plans/modules/<module>.aps.md`
- `docs/guides/branching-strategy.md`
- `docs/guides/worktree-policy.md`

For architecture, scope, documentation, feature flag, release, or branch-policy
changes, also check the relevant ADRs and guides cited by `AGENTS.md`. If those
questions are unsettled, hand off to `planning-workflow` rather than inventing a
plan inside this skill.

## APS Status Model

Use the current APS vocabulary from `plans/aps-rules.md`:

- Module schema statuses: `Proposed`, `Ready`, `In Progress`, `Done`, `Blocked`
- Legacy aliases: `Draft` means `Proposed`; `Complete` means `Done`
- Task execution statuses, when written explicitly, are `open`, `locked`,
  `completed`, or `cancelled`
- Narrative labels such as `Merged`, `Released`, `Shipped`, `Complete`, and
  `Archived` are not schema status values

Implementation is authorised only for `Ready` or `In Progress` work unless the
operator explicitly approves urgent execution of `Proposed` work and that
authorisation is recorded inline.

## Session Context

At session start, quietly build APS awareness:

1. Read `plans/index.aps.md` and identify active modules whose status is not
   `Done`, `Complete`, or `Archived`.
2. Read each active module under `plans/modules/`.
3. Extract item ID, title, status, files, validation, dependencies, priority,
   and cross-reference callouts.
4. Build a file-to-item map from `Files:` fields.
5. Keep a compact working-memory summary: active modules, in-progress items,
   tracked file count, and next Ready item.

Do not print this context unless the user asks for plan status.

## APS Truth Validation

Run this mode when `dev-workflow` asks for an APS gate, when `planning-workflow`
needs a readiness decision, when the user asks if a plan is current, or when
scope appears stale, ambiguous, or cross-cutting.

Steps:

1. Confirm the user goal maps to exactly one primary APS work item. If not,
   return `needs-plan-update` and hand off to `planning-workflow`.
2. If ownership, scope, behaviour, or architecture is unclear, hand off to
   `planning-workflow`; it decides whether to invoke `brainstorming` or
   `planning-council`.
3. Confirm the module and work item status allow implementation.
4. Check dependencies and `Blocks on:`, `Coordinates with:`, `Supersedes:`, and
   `Superseded by:` callouts.
5. Read referenced files from `Files:` plus directly related tests, schemas,
   docs, ADRs, workflows, and feature flag definitions.
6. Compare expected outcome and validation commands against current project
   truth.
7. Identify drift: already-completed work, stale assumptions, moved files,
   changed APIs, invalid commands, missing dependencies, release-state mismatch,
   documentation authority conflicts, or scope conflicts.

Return this report before branch or code:

```markdown
## APS Truth Validation

- Module:
- Work item:
- Status:
- Project truth checked:
- Drift found:
- Decision: valid | needs-plan-update | blocked
- Required APS updates:
- Implementation notes:
```

Decision meanings:

- `valid`: implementation may proceed through `dev-workflow`.
- `needs-plan-update`: update APS via `planning-workflow` or an authorised
  `anvil-plan-spec` agent run before implementation.
- `blocked`: resolve prerequisite, dependency, status, or user decision first.

Implementation MUST NOT begin from a stale, ambiguous, unauthorised, or blocked
APS item.

## APS Updates While Working

In Anvil, APS state is not passive bookkeeping. Keep it current as work moves:

1. Before substantive changes, mark the module/work item `In Progress` when repo
   rules require it.
2. After completing a work item, update its status and validation evidence.
3. Update per-item `Status:` lines in feature work; do not bump stored `N/M`
   counts (ADR-053). Reconcile rollups with `pnpm aps:index` when needed.
4. When all active items are done, mark the module `Done` in schema fields and
   use narrative closeout labels only where the APS rules allow them.
5. Archive completed modules with `git mv` into `plans/archive/modules/` and
   update the index path in the same change.

Ask before plan writes unless the user explicitly requested the change or repo
instructions require the APS update as part of execution. Use the vendored
`anvil-plan-spec` agent for non-trivial module/task edits.

## Reconciliation

Run reconciliation at natural boundaries: after commits, before PRs, after PR
merge/close, or when the user asks for plan status.

Report proposed status changes, files to add, new work items, unblocked items,
validation results, and reviewed items needing no change. Apply changes only
when authorised.

If reconciliation discovers new scope, stale design, or a missing APS item,
return `needs-plan-update` and hand off to `planning-workflow`.

## What This Skill Does Not Do

- It does not replace planning or design work. Use `planning-workflow` for
  unclear goals, architecture, ownership, or scope.
- It does not silently edit plan files.
- It does not execute work from stale or unauthorised APS items.
- It does not create shadow indexes; `plans/index.aps.md` is canonical.
