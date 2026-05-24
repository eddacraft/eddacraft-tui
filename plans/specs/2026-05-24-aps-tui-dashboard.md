# APS TUI Dashboard

| Type | Authority | Owner | Status | Freshness |
| ---- | --------- | ----- | ------ | --------- |
| Spec | Proposed | APSCAN | Draft | Created 2026-05-24 from operator-approved dashboard design |

| Upstream | Downstream |
| -------- | ---------- |
| `plans/index.aps.md`, `plans/modules/*.aps.md`, `plans/aps-rules.md`, `plans/modules/tui-dashboard-render.aps.md` | Future APS implementation plan, `crates/anvil-cli`, `crates/anvil-tui`, `crates/eddacraft-tui` |

## Purpose

Give operators and agents a fast terminal view of current APS work without
manually reading `plans/index.aps.md` and every active module file. The first
version is APS-only and local-only. It must leave a clean seam for future GitHub
and CI enrichment, but it must not depend on network state to answer the core
question: what work is active, stale, blocked, or ready to pick next?

## User-Facing Command

The dashboard should live under the planning surface:

```bash
anvil plan dashboard
```

This keeps the scope narrow and avoids overloading the broader future
`anvil dashboard` command described by `TUIDASH`. The future generic dashboard
surface may consume the same data model, but the APS dashboard is a planning
tool first.

## Scope

### In Scope

- Parse `plans/index.aps.md` for active module rows, release-window grouping,
  status, progress, and module paths.
- Parse active `plans/modules/*.aps.md` files for work-item headings, status,
  validation, owner, dependencies, and file references where present.
- Render an interactive Ratatui dashboard using existing Anvil TUI crates.
- Detect local APS consistency issues and show them as reconciliation hints.
- Expose a reusable `PlanStatusSnapshot` model that can back both TUI and future
  non-interactive output.
- Stay read-only. The dashboard may point at reconciliation opportunities, but it
  must not mutate APS files.

### Out of Scope

- GitHub API calls, PR state, review state, or CI state.
- Editing APS files from inside the TUI.
- Opening branches, PRs, or issues.
- Full `json-render` compatibility.
- Web dashboard integration.
- Historical trends or persisted dashboard state.

## Design Principles

1. **APS is the source of truth.** The dashboard reflects APS text and local git
   metadata only.
2. **Read-only first.** Surfacing stale state is safe; automatically fixing it is
   not.
3. **One data model, multiple renderers.** The TUI should not own APS parsing
   logic directly.
4. **Future enrichment is additive.** GitHub and CI data may annotate modules and
   work items later, but the core snapshot must remain valid without it.
5. **Small useful slice.** The first release answers "what is in progress?",
   "what needs reconciliation?", and "what is ready next?".

## Data Model

The implementation should introduce a local snapshot model with this shape:

```text
PlanStatusSnapshot
  generated_at
  repo_root
  git_branch
  git_sha
  modules[]
  work_items[]
  warnings[]
  enrichments[]
```

`enrichments[]` is intentionally present in v1 but empty. A future
`GithubPlanEnricher` can populate it with PR, review, merge, or CI evidence
without changing the dashboard renderer.

### Module Summary

Each module row should include:

- module ID / scope, for example `DOCGOV`
- title
- module path
- owner, if known
- status
- done count
- total count
- release area / index section, if discoverable
- short note extracted from index prose
- local warnings

### Work Item Summary

Each work item should include:

- APS ID, for example `DOCGOV-008`
- title
- status
- parent module
- validation command, if present
- dependencies, if present
- files, if present
- local warnings

## Local Warning Rules

The APS-only dashboard should detect issues that do not require network state:

- The index progress count disagrees with the module file count.
- The index marks a module `In Progress` while all parsed work items are done.
- A task says it is merged or shipped in prose while its status remains open or
  in progress.
- A module path referenced by `plans/index.aps.md` is missing.
- A non-complete work item has no validation command.
- A blocked item references a dependency that appears complete.
- A module has active work but no clear next ready item.

These warnings should be advisory. They should use wording such as "needs
reconcile" rather than "failed" unless APS parsing itself failed.

## TUI Layout

The first layout should fit a standard terminal and degrade gracefully on narrow
screens.

```text
┌ Anvil APS Work Dashboard ───────────────────────────── main @ <sha> ┐
│ In Progress 22  Ready 18  Blocked 2  Needs Reconcile 1              │
├ Release Focus ──────────────────────────────────────────────────────┤
│ MLP2      65/86  In Progress   daemon-working follow-ups            │
│ DOCGOV     7/10  stale         DOCGOV-008 appears merged            │
│ DISTRIB    4/5   In Progress   anvil migrate remains                │
├ Modules ─────────────────────────┬ Detail ──────────────────────────┤
│ Scope  Progress  Status         │ DOCGOV                            │
│ MLP2   65/86     In Progress    │ Open: DOCGOV-009, DOCGOV-010      │
│ DOCGOV 7/10      In Progress !  │ Hint: DOCGOV-008 needs reconcile  │
│ TUIR   5/8       In Progress    │ Validation: pnpm docs:check       │
└ ↑/↓ select  / filter  enter details  r rescan  q quit ─────────────┘
```

The detail pane should show the selected module's incomplete work items first,
then warnings, then validation commands. If the terminal is too small, the
dashboard may collapse to a single module table plus a footer hint.

## Interaction Model

- `↑` / `↓`: move selection.
- `/`: filter modules and work items by ID, title, or status.
- `Enter`: toggle between module summary and work-item detail.
- `r`: rescan APS files from disk.
- `?`: show keybindings.
- `q` / `Esc`: quit.

No key should mutate repository files in v1.

## Rendering Surfaces

The implementation should prefer existing TUI crates:

- `crates/anvil-cli` owns command registration and argument parsing.
- `crates/anvil-tui` owns the Anvil-specific dashboard surface.
- `crates/eddacraft-tui` provides reusable widgets such as data tables, status
  badges, help bars, and layout helpers.

The APS snapshot builder should live outside the renderer so it can later power
non-interactive output such as:

```bash
anvil plan status --json
```

That JSON command is not required for the first TUI slice, but the boundary
should not prevent it.

## Future GitHub Enrichment Seam

Future work may add a trait-shaped enrichment boundary:

```text
PlanStatusEnricher
  enrich(snapshot) -> snapshot_with_enrichments
```

The first concrete future enricher could attach:

- PR state for referenced PR numbers.
- CI state for open branches.
- merged commit evidence for tasks that say "merged" in prose.
- stale branch or deleted branch hints.
- review-thread counts.

The TUI should treat enrichments as optional annotations, never as the source of
truth for APS status.

## Validation Strategy

The first implementation should be testable without a terminal:

- Snapshot-builder tests using small fixture APS indexes and module files.
- Warning-rule tests for count mismatches, stale in-progress rows, missing
  validation, missing module paths, and completed dependencies.
- Renderer snapshot tests using Ratatui's test backend for the primary layout.
- CLI smoke test that `anvil plan dashboard --help` is wired without launching an
  interactive terminal.

Suggested validation commands for the implementation plan:

```bash
cargo test -p eddacraft-anvil-cli plan_dashboard
cargo test -p eddacraft-anvil-tui aps_dashboard
pnpm format:check
```

The exact package names and test filters should be confirmed during planning.

## APS Placement

This work should be planned as an APS item before implementation. The preferred
home is `APSCAN` because the dashboard exposes APS dialect drift and depends on
canonical parsing. If the work expands beyond a single planning/status surface,
create a dedicated module rather than overloading `TUIDASH`; `TUIDASH` remains
about generic dashboard rendering and json-render compatibility.

## Open Questions

- Should the snapshot builder reuse the existing `packages/aps` parser through a
  command boundary, or should Rust implement a narrow reader for the dashboard?
- Should `anvil plan dashboard` be hidden/experimental until APS canonical
  alignment advances beyond `APSCAN-001`?
- Should stale-plan warnings be duplicated in `pnpm aps:active-lint`, or remain
  dashboard-only until proven useful?

## Decision Summary

- Build an APS-only TUI first.
- Use `anvil plan dashboard` as the command.
- Keep the dashboard read-only.
- Design around `PlanStatusSnapshot` with an empty v1 `enrichments[]` seam.
- Leave GitHub/CI enrichment for a later additive pass.
