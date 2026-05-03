<!--
APS Module: Launch Flow Readiness
==================================
Cross-cutting polish to take the start (init/welcome/doctor/activation) and
watch (file-watcher + dashboard) flows from "demo-quality" to "ship-quality".
Owns its own work items but coordinates with TUIDASH, DRVR, RMCP, RTAI, and
INTD.
RTVS has been superseded by this module + RTVF and is now archived.
See: plans/aps-rules.md
-->

# Launch Flow Readiness

| ID     | Owner | Status      | Progress |
| ------ | ----- | ----------- | -------- |
| LAUNCH | —     | In Progress | 5/14     |

## Purpose

Anvil has two flows that disproportionately shape the user's first and
daily experience:

- **Start** — `anvil init`, `anvil new`, `anvil welcome`, `anvil doctor`,
  `anvil start`, the first-run service, activation and onboarding surfaces.
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
  adapter. LAUNCH-003 now rolls up `WatchData` stats from kernel
  events; the richer json-render dashboard work still lives in
  TUIDASH.

### Activation council outcome (2026-05-03)

- Five independent brainstorms in `plans/brainstorms/2026-05-02-wow-start-*.md`
  converged on the same gap: installation is credible, but the first
  minute after install does not yet prove protection inside the user's
  real AI/editor workflow.
- Planning council review approved the `anvil start` activation path
  with changes: use existing primitives, keep v1 narrow, avoid
  rule-file injection, avoid no-args TUI theatre, and make protection
  claims literal.
- The canonical first-run story becomes `install -> cd repo -> anvil
  start`; `anvil welcome` remains the menu/tutorial surface.
- V1 editor claims are limited to Cursor and Claude Code via the Rust
  MCP launch path. Watch mode is a save-time fallback, not pre-write
  interception.

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
- Closing the activation gap from the 2026-05-03 planning council by
  making `anvil start` the canonical local activation path.
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
- Rule/instruction-file injection (`.cursorrules`, `.clauderules`,
  global AI rules, or equivalent) as an enforcement mechanism. MCP
  pre-write validation is the only v1 editor enforcement claim.
- Broad editor or AI-session support beyond Cursor and Claude Code in
  the activation MVP. Windsurf, VS Code, Copilot CLI, Codex CLI, and
  running-process auto-attach remain downstream work unless RMCP/DRVR
  verifies them first.
- Git hook installation as a default activation step; hooks may be
  offered later, but they are not part of the hard wow-start path.
- Cloud login, team policy pull, CI setup, demo fixtures, challenge
  files, or guaranteed-catch prompt catalogues before local protection
  is working.
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
- **Coordinates with:** RMCP (Rust MCP launch shim owns the concrete
  Cursor / Claude Code config and `anvil_validate_write` server path).
- **Coordinates with:** RTAI (pre-write validation semantics and
  diagnostic contracts must remain consistent with activation status).
- **Coordinates with:** INTD (daemon-backed validation and live status
  may strengthen activation verification, but v1 must degrade honestly
  when daemon evidence is unavailable).
- **Supersedes (in part):** RTVS — the watch-flow intent of RTVS
  Phase 2 and Phase 3 lives here as LAUNCH-001..003; its validation
  engine intent is folded into RTVF. RTVS itself has been archived
  (`plans/archive/modules/real-time-validation-simplified.aps.md`).

## Tasks

> Status: In Progress. LAUNCH-001, LAUNCH-003, LAUNCH-004,
> LAUNCH-005, and LAUNCH-007 are complete; remaining activation,
> tutorial, upgrade, and watch work stays Todo until picked up.

### LAUNCH-013: Make version and upgrade guidance install-method aware

- **Intent:** A user can answer "am I current, what is latest, and how do I
  upgrade?" without knowing whether they installed Anvil through WinGet,
  Scoop, Homebrew, or the direct installer.
- **Expected Outcome:** Add an explicit `anvil version` surface that prints the
  current binary version, latest available release version, update availability,
  detected install method, and recommended upgrade command. Human and JSON
  output include `current_version`, `latest_version`, `update_available`,
  `install_method`, and `upgrade_command` when available. Detection covers
  Homebrew, Scoop, WinGet, direct cargo-dist installer / PowerShell installer,
  and unknown/manual installs. Network/latest lookup failures remain non-fatal
  and still print the local version. The recommendation accounts for older
  direct installs that predate `anvil update`: those users are told to rerun the
  latest installer rather than use a missing subcommand.
- **Coordinates with:** release metadata on `eddacraft/anvil` and the existing
  `anvil update --check` path in `crates/anvil-cli/src/commands/update.rs`.
- **Validation:** CLI tests cover human and JSON output, mocked latest-release
  responses, each detected install method, unknown installs, and network lookup
  failure. Manual smoke covers the currently published release metadata.
- **Confidence:** medium
- **Status:** Todo

---

### LAUNCH-014: Make the interactive tutorial prove value faster

- **Intent:** The tutorial should help a new user understand Anvil's protection
  loop quickly, not just walk command taxonomy.
- **Expected Outcome:** Rework `anvil tutorial` so the first path is a short,
  repo-local value path: explain what Anvil is about to check, run or simulate
  a high-signal check on safe fixture content, show the result, and point the
  user to the next activation step. The tutorial keeps an explicit learning
  path for deeper concepts, but the default flow prioritises a concrete first
  win and uses the same activation vocabulary as `anvil start`, `anvil status`,
  and `anvil doctor`.
- **Coordinates with:** LAUNCH-006 activation entrypoint, LAUNCH-008 activation
  protection states, LAUNCH-010 baseline copy, and LAUNCH-012 verification and
  retry paths.
- **Validation:** Snapshot or CLI tests cover the default tutorial path,
  no-config / already-initialised repo states, and copy that avoids claiming
  pre-write protection unless activation evidence supports it.
- **Confidence:** medium
- **Status:** Todo

---

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
- **Status:** Complete

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
- **Coordinates with:** TUIDASH-009 — *callout swept and closed
  2026-04-30 per ADR-034 rule 3.* LAUNCH-003 shipped first, so the
  conditional "Superseded by" branch did not fire. The named
  `WatchStats` contract above is the inheritance TUIDASH-009 will
  consume when the dashboard surface lands; no rework expected on
  that seam.
- **Supersedes:** RTVS Phase 3 ("Terminal TUI Dashboard") — that work
  now lives here.
- **Validation:** TUI snapshot test (insta) covering populated panes
  on a fixture event stream; unit test asserting `WatchStats` (or
  equivalent) is the only public surface for stat data.
- **Confidence:** medium
- **Status:** Complete

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
- **Status:** Complete

---

### LAUNCH-006: Make `anvil start` the activation entrypoint

- **Intent:** A new user can run one command in a repo and reach the
  shortest safe path to local protection without learning Anvil's
  command taxonomy.
- **Expected Outcome:** `anvil start` routes to an activation flow that
  composes existing init, first-scan, MCP, doctor/status, and watch
  primitives. `anvil welcome` remains the explicit menu/tutorial
  surface. No new top-level `activate` or `connect` command is added
  for the MVP.
- **Coordinates with:** RMCP for MCP install/verify, RTAI for pre-write
  semantics, INTD for live-status evidence, and DRVR for future editor
  expansion. This task supersedes the narrower watch-only shortcut
  originally described here.
- **Validation:** End-to-end test on a fresh temp repo asserts that
  `anvil start` enters activation, preserves access to `anvil welcome`,
  and emits one literal final state.
- **Confidence:** medium
- **Status:** Todo

---

### LAUNCH-007: Unify interactive fix handling across start-flow surfaces

- **Intent:** `f` means the same thing everywhere Anvil offers an
  interactive fix in the welcome / tutorial / audit / doctor flows.
- **Expected Outcome:** Start-flow surfaces emit one shared fix request
  shape and route it through one shared CLI-side handler. Surfaces only
  advertise `f` when the selected item has a deterministic fix behind
  that handler; dead prompts are removed. `doctor`, tutorial scan
  findings, and any fixable audit items all use the same dispatch path
  instead of each surface carrying bespoke `wants_fix` wiring.
- **Validation:** Targeted Rust tests prove the shared handler applies
  supported fixes and that each participating surface only exposes `f`
  when a request can actually be serviced.
- **Confidence:** medium
- **Status:** Complete

---

### LAUNCH-008: Define activation protection states

- **Intent:** `anvil start`, `anvil status`, and `anvil doctor` use the
  same literal vocabulary for activation and degraded modes.
- **Expected Outcome:** The shared state model distinguishes
  `protecting`, `ready_restart_required`, `watching`, `needs_action`,
  `unsupported`, and `error`. User-facing copy never describes config
  presence, restart-required setup, or watch-only fallback as pre-write
  protection.
- **Coordinates with:** RMCP verification tiers, INTD daemon-status
  evidence, and existing `doctor` / `status` output.
- **Validation:** Targeted CLI tests cover final-state rendering for at
  least protected, restart-required, watch fallback, unsupported, and
  config-error scenarios.
- **Confidence:** high
- **Status:** Todo

---

### LAUNCH-009: Safely activate Cursor and Claude Code MCP paths

- **Intent:** The activation flow wires only verified v1 MCP clients
  without corrupting user editor configuration or over-claiming live
  protection.
- **Expected Outcome:** `anvil start` detects Cursor and Claude Code,
  installs or verifies the Rust MCP launch shim where safe, and reports
  the exact tier reached: config absent, config written, restart
  required, server startable, or live validation observed. Existing
  editor config is parsed before modification, written atomically, and
  left untouched on parse failure or unsafe drift. Rule/instruction
  files are not edited.
- **Coordinates with:** RMCP follow-ups for client config paths,
  `anvil mcp install --verify`, and the `anvil_validate_write` tool
  contract.
- **Validation:** Fixture-backed tests cover Cursor and Claude Code
  config install, verify, idempotent rerun, parse failure, drift, and
  restart-required output.
- **Confidence:** medium
- **Status:** Todo

---

### LAUNCH-010: Baseline old findings before first activation signal

- **Intent:** First activation proves future-change protection without
  punishing users for inherited repository problems.
- **Expected Outcome:** Activation creates or reuses a local baseline
  for existing findings, runs a local-only high-signal first scan, and
  prints a concise summary such as `Existing findings baselined; future
  changes are checked`. Secret or credential-like findings are
  prioritised as the first security signal when present, but activation
  does not imply the repository is clean.
- **Coordinates with:** ADR-003 new-edges-only behaviour,
  `crates/anvil-checks`, RTAI diagnostic shape, and existing first-scan
  support from LAUNCH-004.
- **Validation:** Integration test on a fixture repo with legacy and new
  findings asserts that legacy findings are baselined, new findings are
  surfaced, and the final copy does not claim zero risk.
- **Confidence:** medium
- **Status:** Todo

---

### LAUNCH-011: Add honest watch fallback activation mode

- **Intent:** Users without confirmed MCP pre-write validation still get
  save-time protection with a clear degraded-state label.
- **Expected Outcome:** When no supported MCP client is detected, MCP
  verification fails safely, or restart is pending, activation can start
  or offer `anvil watch` scoped to the current repo. The final state is
  `watching` when the watcher is active and never claims pre-write AI
  interception.
- **Coordinates with:** LAUNCH-002 for action/TUI coexistence,
  LAUNCH-003 watch stats, and the kernel watch loop.
- **Validation:** End-to-end test with no supported client asserts that
  activation reaches watch fallback, scopes it to the repo, and renders
  `watching` rather than `protecting`.
- **Confidence:** medium
- **Status:** Todo

---

### LAUNCH-012: Add activation verification and retry path

- **Intent:** Users and support can re-check activation state without
  rewriting configuration or guessing which layer failed.
- **Expected Outcome:** A verification path (`anvil start --verify`, or
  equivalent `status` / `doctor` integration) distinguishes config
  presence, config validity, MCP server startability, restart-required
  state, live validation evidence where available, watch liveness,
  baseline state, and last activation error. Re-running activation is
  idempotent and gives a concrete repair or manual action when it
  cannot proceed safely.
- **Coordinates with:** `anvil doctor`, `anvil status`, RMCP verify,
  and INTD status APIs.
- **Validation:** CLI tests assert verification performs no writes,
  reports each degraded layer separately, and leaves existing config
  unchanged on repeated runs.
- **Confidence:** medium
- **Status:** Todo

## Risks

- **TUIDASH supersession of LAUNCH-003.** *Resolved 2026-04-30:*
  LAUNCH-003 shipped first; the bespoke watch surface stays. The
  named `WatchStats` adapter is the contract TUIDASH-009 will
  consume when the dashboard surface lands. The supersession
  branch is closed.
- **LAUNCH-001 scope expansion.** The glob filter is a feature build
  with several reasonable shapes (which library, where matching runs,
  how it composes with the internal denylist). Confidence is already
  marked low; expect a short design note before implementation.
- **Cross-reference rot.** Prose callouts to other modules' work
  items will silently break when targets are renamed or archived.
  The cleanup obligation is documented in the Cross-cutting
  convention section above; honour it on every task close.
- **Activation over-claiming.** The highest-risk launch failure is
  printing `protecting` when only config was written or a restart is
  still required. LAUNCH-008 owns the shared state model that prevents
  this.
- **Editor config trust.** MCP setup touches user-owned editor config.
  LAUNCH-009 must stop safely on parse failure or unsafe drift rather
  than doing a clever best-effort merge.
- **Watch fallback perception.** Watch mode is useful, but weaker than
  MCP pre-write validation. LAUNCH-011 must make the degraded state
  explicit.

## Open questions

- Can live MCP invocation be observed reliably enough in v1 to ever
  print `protecting`, or should first activation normally stop at
  `ready_restart_required` until a subsequent verify sees evidence?
- Should activation verification live primarily in `anvil start
  --verify`, `anvil status`, `anvil doctor`, or all three with one
  shared backend?
- Does the IFR-003 port (LAUNCH-004) want the same 5-second budget the
  TypeScript version honoured, or a Rust-appropriate target?
