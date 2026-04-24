<!--
APS Module: Launch Flow Readiness
==================================
Cross-cutting polish to take the start (init/welcome/doctor) and watch
(file-watcher + dashboard) flows from "demo-quality" to "ship-quality".
Owns its own work items but coordinates with TUIDASH and DRVR.
RTVS has been superseded by this module + RTVF and is now archived.
See: plans/aps-rules.md
-->

# Launch Flow Readiness

| ID     | Owner | Status      |
| ------ | ----- | ----------- |
| LAUNCH | —     | In Progress |

## Purpose

Anvil has two flows that disproportionately shape the user's first and
daily experience:

- **Start** — `anvil init`, `anvil new`, `anvil welcome`, `anvil doctor`,
  the first-run service, onboarding surfaces.
- **Watch** — `anvil watch`, the kernel watch loop, the watch TUI
  dashboard surface.

A code-level audit (2026-04-24) found the foundations are solid but
several concrete gaps stop both flows being recommendable to a new
user. This module gathers those gaps under one cross-cutting goal so
the work can be sequenced and defended as a bundle, instead of
fragmenting into one-off fixes that nobody owns.

## Cross-cutting convention

This module is the first trial of a **cross-cutting module convention**
in this repository. APS does not (yet) have a first-class concept for
modules that coordinate work across other modules; rather than invent a
new primitive, this module follows three rules:

1. **Owns its own work items.** Every `LAUNCH-NNN` task is owned and
   counted by this module. Progress and archiving are local.
2. **Cross-references via callouts.** Where this module relates to
   work owned elsewhere, it uses one of three callouts inside the
   relevant task:
   - **Coordinates with:** the other item's outcome benefits this work
     but does not block it.
   - **Blocks on:** this task cannot land until the referenced item
     completes.
   - **Superseded by:** this task should be closed (with a pointer)
     if the referenced item lands first under a wider scope.
3. **Cleans up after itself.** Cross-references are prose, not a typed
   relation — they will silently rot when target items are renamed,
   re-scoped, or archived (modules archive via `git mv`). Whenever a
   `LAUNCH-NNN` task is closed, the closer must read each callout in
   the task body and either confirm the reference still resolves or
   delete it. When this module itself archives, the closer must do
   the same sweep across all remaining open callouts.

> ⚠️ **Do not copy this convention to a second module before it has been
> tried in anger here.** If the pattern proves useful across at least
> one further cross-cutting bundle, promote it to a first-class module
> type in [`aps-rules.md`](../aps-rules.md) — ideally with a
> machine-readable callout syntax (e.g. YAML frontmatter) so a lint can
> verify references, since prose callouts cannot be enforced. At the
> point a second author is tempted to copy, that is the trigger:
> promote with a real spec, or document why the pattern does not apply.
> Do not let inertia turn this trial into a default.

## Background

### Start flow — current state

- Welcome / onboarding / wizard / tutorial surfaces all exist
  (`crates/anvil-cli/src/commands/welcome.rs`, `wizard.rs`,
  `tutorial.rs`) and are functional.
- `anvil doctor` (`crates/anvil-cli/src/commands/doctor.rs`) runs nine
  checks with a TUI + JSON output and four auto-fixes.
- The first-run service (`crates/anvil-cli/src/services/first_run.rs`)
  manages the `.anvil/first-run` marker robustly.
- IFR-003 (post-init automatic analysis) shipped in the original
  TypeScript CLI but did **not** carry across to the Rust rewrite —
  `anvil init` in `crates/anvil-cli/src/commands/init.rs` ends with a
  text "Run `anvil doctor` to verify your setup." and stops there.

### Watch flow — current state

- The kernel watch loop (`crates/anvil-kernel/src/watch.rs`) is
  well-built: parallel initial scan capped at half cores, panic
  isolation, integrated policy engine.
- The CLI dispatcher (`crates/anvil-cli/src/commands/watch.rs`) is
  solid: argument parsing, path canonicalisation, debounce, action
  dispatch.
- The watch TUI surface (`crates/anvil-tui/src/surfaces/watch/`) has a
  2x2 grid skeleton (Status / Queue / History / Stats) and an event
  adapter, but `WatchData` stats are never rolled up from kernel
  events. The rich-dashboard work lives in TUIDASH.

### Audit findings (2026-04-24)

| Area  | Finding | Reference |
|-------|---------|-----------|
| Watch | `WatchConfig.include_patterns` and `exclude_patterns` are declared but never consumed. The existing `FileFilter` on `WatcherConfig` is an internal denylist (hardcoded directory components, fixed extension allowlist) — it serves a different purpose and has no glob engine. User-facing pattern filtering needs to be built, not just wired. | `crates/anvil-kernel/src/watch.rs:48-55`, `crates/anvil-kernel/src/watcher/filter.rs` |
| Watch | `--action` is mutually exclusive with TUI mode; users must choose dashboard or automation. | `crates/anvil-cli/src/commands/watch.rs:236` |
| Watch | TUI `WatchData.pass_rate`/`avg_duration_ms`/`total_runs` are declared but never populated from kernel events. | `crates/anvil-tui/src/surfaces/watch/event_adapter.rs` |
| Start | Post-init auto-analysis (IFR-003) regressed in the Rust port; new users hit a flat config-write with no first signal of value. | `crates/anvil-cli/src/commands/init.rs` |
| Start | `anvil doctor` surfaces problems but rarely auto-fixes or links to remediation; user is left to read README references. | `crates/anvil-cli/src/commands/doctor.rs` |
| Start | Onboarding is a 5-6 surface chain (welcome → onboarding → init → discovery → tutorial → welcome-hub) with no shortcut for users who want to land directly in `watch`. | `crates/anvil-cli/src/commands/welcome.rs` |

## Scope

**In scope:**

- Closing the six audit findings above as `LAUNCH-NNN` tasks.
- Absorbing the watch-flow intent of the now-superseded RTVS module
  (Phase 2 "Enhanced Watch Mode" and Phase 3 "Terminal TUI Dashboard"
  were written against the retired Ink TUI and pre-dated the Ratatui
  port).
- Defining the cross-cutting module convention in this repository (this
  document).

**Out of scope:**

- The full json-render TUI dashboard — owned by TUIDASH; this module
  only ensures the existing 2x2 skeleton stops shipping with empty
  stats so TUIDASH inherits a working baseline rather than a stub.
- Surface-driver migration (DRVR) — if surfaces become drivers on the
  intercept daemon, the watch CLI flow will shift. This module assumes
  the in-process Rust surfaces stay authoritative for now.
- Any change to the kernel's parser, graph, or policy engine; the
  audit found these are sound.
- Promoting the cross-cutting convention to APS itself — defer until
  pattern is reused.

## Dependencies

- **Coordinates with:** TUIDASH (the rich dashboard supersedes the
  current 2x2 grid; LAUNCH-003's adapter type is the inheritance
  contract, see that task).
- **Coordinates with:** DRVR (surface drivers may reshape the watch
  flow; LAUNCH stays in the in-process world for now).
- **Supersedes (in part):** RTVS — the watch-flow intent of RTVS
  Phase 2 and Phase 3 lives here as LAUNCH-001..003; its validation
  engine intent is folded into RTVF. RTVS itself has been archived
  (`plans/archive/modules/real-time-validation-simplified.aps.md`).

## Tasks

> Status: Draft. Tasks are listed for review. Module is **not yet
> Ready** — see open questions and confidence notes below before
> promoting.

### LAUNCH-001: Implement user-facing glob filter for watch loop

- **Intent:** A watch invocation respects the include / exclude
  patterns the user passes on the command line.
- **Expected Outcome:** `anvil watch --patterns "src/**/*.ts" --exclude "vendor/**"`
  causes only matching files to trigger re-evaluation. Both
  `--patterns` and `--exclude` are treated as user-supplied glob
  filters, and the CLI help / behaviour is updated to document that
  `--exclude` now matches glob patterns rather than the current
  comma-separated list of directory names. A glob engine (e.g.
  `globset`) is introduced so user-supplied patterns are matched
  against changed paths in the watch loop. The dead
  `WatchConfig.include_patterns` / `exclude_patterns` fields are
  either consumed or removed; the existing internal `FileFilter` is
  retained for its hardcoded denylist role and is not conflated with
  user-supplied filters.
- **Validation:** Integration test in `crates/anvil-kernel/tests/`
  drives the watch loop with a fixture repo and asserts that an
  excluded path does not raise an event and an included path does.
- **Confidence:** low — the working scope is a feature build, not a
  wire-up; total cost depends on the glob engine choice and how the
  user-pattern path interacts with the existing internal denylist.
- **Status:** Todo

---

### LAUNCH-002: Allow `--action` with TUI mode

- **Intent:** A user can run `anvil watch --action gate` and still
  see the live dashboard.
- **Expected Outcome:** The mutual-exclusion guard between `--action`
  and TUI mode at `commands/watch.rs:236` is removed; action output
  reaches the user without freezing the TUI render loop. Whether
  output lands in a dedicated action-output pane or is routed to the
  existing history pane / stdout is decided by the spike below.
- **Spike (precondition):** Before committing to a UI surface for
  action output, do a short investigation of the action dispatch path
  to confirm whether non-blocking integration is local or requires
  reshaping the dispatcher. Record the decision in the task comment.
- **Validation:** Manual smoke test plus an integration test asserting
  the TUI render loop continues ticking while an action runs.
- **Confidence:** low — outcome scope depends on the spike result.
- **Status:** Todo

---

### LAUNCH-003: Roll up real-time stats in watch TUI

- **Intent:** The watch dashboard's Status / Stats panes show real
  numbers from kernel events instead of skeleton zeros.
- **Expected Outcome:** `WatchData.total_runs`, `pass_rate`, and
  `avg_duration_ms` are populated by the event adapter and refreshed
  on each event without blocking the render loop. The stats rollup
  logic is exposed via a named, documented type (e.g. a
  `WatchStatsSource` trait or a plain `WatchStats` struct in a shared
  location) that is the only consumer-visible surface for stat data.
  The bespoke `WatchData` struct depends on that type, not the other
  way around.
- **Coordinates with:** TUIDASH-009. The named type above is the
  inheritance contract — TUIDASH-009's json-render data binding for
  the watch dashboard consumes it, so the rollup work survives the
  surface swap. If TUIDASH-009 lands first and defines its own data
  model, this task should be **Superseded by:** TUIDASH-009 instead.
- **Supersedes:** RTVS Phase 3 ("Terminal TUI Dashboard") — that work
  now lives here.
- **Validation:** TUI snapshot test (insta) covering populated panes
  on a fixture event stream; unit test asserting `WatchStats` (or
  equivalent) is the only public surface for stat data.
- **Confidence:** medium
- **Status:** Todo

---

### LAUNCH-004: Restore post-init auto-analysis in the Rust CLI

- **Intent:** A user who runs `anvil init` sees a first scan result
  immediately after configuration completes, instead of being told
  to run `doctor`.
- **Expected Outcome:** After config is written, the init command
  selects a representative file sample and runs the default checks
  against it; results print inline. Behaviour matches the archived
  IFR-003 outcome from the TypeScript implementation; sampling
  strategy and time budget are implementation choices to be matched
  against IFR-003 intent, not pinned in this spec.
- **Coordinates with:** archived `intelligent-first-run` module
  (IFR-003 Complete) — port that intent to Rust; do not re-spec it.
- **Validation:** Integration test against a temp repo asserts that
  `anvil init --force` exits with sample-analysis output present.
- **Confidence:** medium
- **Status:** Complete

---

### LAUNCH-005: Deepen `anvil doctor` remediation guidance

- **Intent:** When `doctor` flags a problem, the user sees a concrete
  next action — a fix command, a doc link, or an auto-fix offer —
  rather than a bare "see README" reference.
- **Expected Outcome:** Every doctor check emits a remediation block
  (link, command, or auto-fix prompt). Every check whose failure has
  a deterministic fix exposes an auto-fix; checks without one emit
  either a remediation command or a doc link. No check terminates at
  a bare "see README" reference.
- **Validation:** JSON-mode output for each check includes a non-empty
  `remediation` field; snapshot tests cover the new fields.
- **Confidence:** medium
- **Status:** Todo

---

### LAUNCH-006: Onboarding shortcut to watch

- **Intent:** A returning user who knows what they want can skip the
  full welcome / onboarding chain and land directly in `anvil watch`.
- **Expected Outcome:** `anvil welcome --skip-to watch` (or an
  equivalent flag) bypasses the discovery / tutorial surfaces and
  starts the watch flow with default config; the first-run marker
  is still set.
- **Validation:** End-to-end test on a fresh temp repo asserts the
  flag advances the marker and starts the watch loop.
- **Confidence:** high
- **Status:** Todo

## Risks

- **TUIDASH supersession of LAUNCH-003.** LAUNCH-003 invests in the
  bespoke watch surface; if TUIDASH lands quickly, that surface goes
  away. Mitigation is the named adapter type required by LAUNCH-003
  — TUIDASH consumes it instead of re-implementing the rollup. If
  TUIDASH is well advanced before LAUNCH-003 starts, mark LAUNCH-003
  **Superseded by:** TUIDASH-009 rather than building the bespoke
  surface twice.
- **LAUNCH-001 scope expansion.** The glob filter is a feature build
  with several reasonable shapes (which library, where matching runs,
  how it composes with the internal denylist). Confidence is already
  marked low; expect a short design note before implementation.
- **Cross-reference rot.** Prose callouts to other modules' work
  items will silently break when targets are renamed or archived.
  The cleanup obligation is documented in the Cross-cutting
  convention section above; honour it on every task close.

## Open questions

- Should LAUNCH carry a doctor-on-first-watch step (auto-run `doctor`
  if `.anvil/first-run` is unset and the user goes straight to
  `watch`), or is that a LAUNCH-007?
- Does the IFR-003 port (LAUNCH-004) want the same 5-second budget the
  TypeScript version honoured, or a Rust-appropriate target?
