# APS TUI Dashboard Implementation Plan

**Goal:** Ship a read-only terminal dashboard that summarises active APS work,
highlights local reconciliation hints, and leaves room for later GitHub/CI
enrichment.
**Architecture:** `crates/anvil-cli` owns command registration and local APS file
scanning. `crates/anvil-tui` owns the Ratatui surface and view-state behaviour.
The shared boundary is a `PlanStatusSnapshot` view model with an empty v1
`enrichments` field so later GitHub enrichment can annotate, not replace, APS
truth.
**Tech Stack:** Rust, clap, Ratatui, `anvil-tui`, `eddacraft-tui`, local Markdown
fixture parsing, cargo unit tests.

---

## File Map

- `plans/specs/2026-05-24-aps-tui-dashboard.md` — approved design authority.
- `plans/modules/aps-canonical-alignment.aps.md` — add the executable APS work
  item before implementation starts.
- `plans/index.aps.md` — update APSCAN progress/count when the work item is
  added or completed.
- `crates/anvil-cli/src/main.rs` — add the top-level `plan` command variant,
  dispatch branch, auth classification, and command-name mapping.
- `crates/anvil-cli/src/commands/mod.rs` — expose the new `plan` command module.
- `crates/anvil-cli/src/commands/plan.rs` — clap arguments for
  `anvil plan dashboard`, command handler, TTY gating, and handoff to the TUI.
- `crates/anvil-cli/src/plan_dashboard.rs` — local APS snapshot builder, minimal
  Markdown parsing, progress reconciliation, warning rules, and fixture tests.
- `crates/anvil-tui/src/lib.rs` — no direct change expected unless module export
  shape needs adjustment.
- `crates/anvil-tui/src/surfaces/mod.rs` — expose the `plan_dashboard` surface.
- `crates/anvil-tui/src/surfaces/plan_dashboard/mod.rs` — dashboard app state,
  selection/filter/rescan actions, and public API for CLI launch.
- `crates/anvil-tui/src/surfaces/plan_dashboard/render.rs` — Ratatui rendering for
  summary strip, release-focus list, module table, detail pane, and help footer.
- `crates/anvil-tui/src/surfaces/plan_dashboard/event_adapter.rs` — key handling
  for navigation, filter mode, detail toggle, rescan request, help, and quit.
- `crates/anvil-tui/src/test_utils.rs` — reuse existing test-backend helpers;
  modify only if the new surface needs a small shared helper.
- `crates/anvil-cli/Cargo.toml` — no dependency change expected; modify only if
  tests need an existing workspace dev-dependency explicitly listed.
- `crates/anvil-tui/Cargo.toml` — no dependency change expected.

## Tasks

### Task 1: Authorise The APS Work Item

**Files:**

- Modify: `plans/modules/aps-canonical-alignment.aps.md`
- Modify: `plans/index.aps.md`
- Reference: `plans/specs/2026-05-24-aps-tui-dashboard.md`

- [x] Add `APSCAN-011: Add APS TUI dashboard` and mark it `Status: In Progress`
      before implementation.
- [x] Set expected outcome to a read-only `anvil plan dashboard` that builds a
      local `PlanStatusSnapshot`, renders active work, and flags APS-only
      reconciliation hints.
- [x] Set validation to:
      `cargo test -p eddacraft-anvil plan_dashboard && cargo test -p eddacraft-anvil-tui plan_dashboard && pnpm format:check && pnpm docs:check`.
- [x] Update APSCAN count from `1/10` to `1/11` in the module header and index.
- [x] Run `pnpm docs:check` and verify the APS surface reports no new errors.
- [x] Commit: `docs(apscan): add APS TUI dashboard work item`.

### Task 2: Add Local APS Snapshot Builder

**Files:**

- Create: `crates/anvil-cli/src/plan_dashboard.rs`
- Modify: `crates/anvil-cli/src/main.rs`

- [x] Write failing tests in `crates/anvil-cli/src/plan_dashboard.rs` using
      temporary fixture directories for:
      `loads_index_modules`, `detects_index_module_count_mismatch`,
      `detects_missing_module_path`, `detects_open_item_without_validation`, and
      `leaves_enrichments_empty`.
- [x] Run `cargo test -p eddacraft-anvil plan_dashboard --lib` and verify the new
      tests fail to compile or fail because the builder is not implemented.
- [x] Define `PlanStatusSnapshot`, `ModuleSummary`, `WorkItemSummary`,
      `PlanWarning`, and `PlanEnrichment` in `plan_dashboard.rs`.
- [x] Implement `build_plan_status_snapshot(repo_root: &Path) -> Result<PlanStatusSnapshot>`.
- [x] Parse `plans/index.aps.md` tables narrowly: module path, scope, status,
      progress, and containing section heading.
- [x] Parse active module files narrowly: H1 title, header owner/status/progress,
      `### ID: title` work item headings, status lines, validation lines,
      dependencies lines, and files lines.
- [x] Implement advisory warnings for count mismatch, missing module path,
      in-progress-with-all-items-done, merged-prose-with-open-status, missing
      validation, completed dependency on blocked item, and no ready next item.
- [x] Run `cargo test -p eddacraft-anvil plan_dashboard --lib` and verify it passes.
- [x] Commit: `feat(apscan): build APS dashboard snapshot`.

### Task 3: Wire `anvil plan dashboard` CLI

**Files:**

- Create: `crates/anvil-cli/src/commands/plan.rs`
- Modify: `crates/anvil-cli/src/commands/mod.rs`
- Modify: `crates/anvil-cli/src/main.rs`

- [x] Write failing CLI tests in `crates/anvil-cli/src/main.rs` for
      `parse_command(&["plan", "dashboard"])`, canonical command name `plan`,
      and unauthenticated access classification matching local planning commands.
- [x] Run `cargo test -p eddacraft-anvil plan_command --lib` and verify the tests
      fail because the command is not wired.
- [x] Add `pub mod plan;` to `commands/mod.rs`.
- [x] Add `Commands::Plan(commands::plan::PlanArgs)` to the top-level clap enum.
- [x] Add dispatch for `Commands::Plan(args)` in the existing command runner.
- [x] Add command-name mapping and auth/interactive classification entries.
- [x] Implement `PlanArgs` with subcommand `Dashboard(DashboardArgs)`.
- [x] In `DashboardArgs`, support `--json` only if it can print
      `PlanStatusSnapshot` without launching the TUI; otherwise leave JSON out of
      v1 and make `--no-tui` print a concise non-interactive summary.
- [x] Run `cargo test -p eddacraft-anvil plan_command --lib` and verify it passes.
- [x] Commit: `feat(cli): add plan dashboard command`.

### Task 4: Render APS Dashboard Surface

**Files:**

- Create: `crates/anvil-tui/src/surfaces/plan_dashboard/mod.rs`
- Create: `crates/anvil-tui/src/surfaces/plan_dashboard/render.rs`
- Modify: `crates/anvil-tui/src/surfaces/mod.rs`

- [x] Write failing Ratatui test-backend tests for:
      `renders_summary_counts`, `renders_module_rows`, `renders_warning_marker`,
      and `collapses_on_narrow_terminal`.
- [x] Run `cargo test -p eddacraft-anvil-tui plan_dashboard --lib` and verify the
      tests fail because the surface does not exist.
- [x] Define the TUI-facing snapshot structs or re-export compatible structs from
      the CLI snapshot boundary if crate layering permits it. If layering does
      not permit it, keep duplicate serialisable view structs in `anvil-tui` and
      convert in `commands/plan.rs`.
- [ ] Render a title line with branch/SHA when present.
- [x] Render summary counts for `In Progress`, `Ready`, `Blocked`, and warning
      count.
- [ ] Render release-focus rows first when section metadata identifies current
      release rows; otherwise show all in-progress rows.
- [x] Render module table with scope, progress, status, and warning marker.
- [ ] Render detail pane with incomplete work items, warnings, validation, and
      dependencies.
- [x] Render footer keybindings.
- [x] Run `cargo test -p eddacraft-anvil-tui plan_dashboard --lib` and verify it
      passes.
- [x] Commit: `feat(tui): render APS work dashboard`.

### Task 5: Add Interaction And CLI Launch

**Files:**

- Create: `crates/anvil-tui/src/surfaces/plan_dashboard/event_adapter.rs`
- Modify: `crates/anvil-tui/src/surfaces/plan_dashboard/mod.rs`
- Modify: `crates/anvil-cli/src/commands/plan.rs`

- [x] Write failing tests for selection movement, filter text, detail toggle,
      rescan request, help toggle, and quit action.
- [x] Run `cargo test -p eddacraft-anvil-tui plan_dashboard --lib` and verify the
      interaction tests fail.
- [x] Implement dashboard state transitions for `Up`, `Down`, `/`, `Enter`, `r`,
      `?`, `q`, and `Esc`.
- [x] Implement a CLI launch path that builds the snapshot, enters the existing
      TUI shell, and handles rescan requests by rebuilding from disk.
- [x] Make non-TTY or `--no-tui` mode print a deterministic text summary rather
      than attempting raw-mode terminal launch.
- [x] Run `cargo test -p eddacraft-anvil-tui plan_dashboard --lib` and
      `cargo test -p eddacraft-anvil plan_dashboard --lib` and verify both pass.
- [x] Commit: `feat(tui): add APS dashboard navigation`.

### Task 6: Validate And Close Out

**Files:**

- Modify: `plans/modules/aps-canonical-alignment.aps.md`
- Modify: `plans/index.aps.md`
- Modify: `plans/execution/2026-05-24-aps-tui-dashboard.md`

- [x] Run `cargo test -p eddacraft-anvil plan_dashboard`.
- [x] Run `cargo test -p eddacraft-anvil-tui plan_dashboard`.
- [ ] Run `pnpm format:check`.
- [x] Run `pnpm docs:check`.
- [ ] Run `pnpm lint:check` if Rust-only changes do not make it redundant in the
      current CI classification.
- [ ] Mark `APSCAN-011` complete only after validation is green.
- [ ] Update APSCAN count from `1/11` to `2/11` in the module header and index.
- [ ] Record closeout evidence in the `APSCAN-011` task body.
- [ ] Commit: `docs(apscan): close APS TUI dashboard work item`.

## Execution Notes

- Keep v1 read-only. Do not add edit, resolve, or auto-reconcile actions.
- Keep GitHub/CI out of v1. The only future-facing requirement is the empty
  `enrichments` field and an internal boundary that can later attach optional
  annotations.
- Prefer narrow Markdown parsing over a broad APS parser rewrite. If the parser
  work expands, stop and split it into a separate APSCAN task.
- Do not use `TUIDASH` as the owning module unless the implementation pivots to
  generic json-render dashboard rendering. This plan is APS status-specific.
