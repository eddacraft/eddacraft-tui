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
| LAUNCH | —     | In Progress | 10/16    |

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
> LAUNCH-005, LAUNCH-007, LAUNCH-008, LAUNCH-012, LAUNCH-015, and
> LAUNCH-016 are complete; remaining activation, tutorial, upgrade,
> and watch work stays Todo until picked up.

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
- **Status:** Complete — `anvil version` lands as a new top-level
  command in `crates/anvil-cli/src/commands/version.rs`. Detects
  Homebrew (path-prefix match), Scoop (path marker), WinGet
  (`WindowsApps\\eddacraft` marker), the cargo-dist installer (via
  install receipt), and a `dev_build` tier (`target/debug` /
  `target/release`); falls back to `unknown / manual`. Per-method
  upgrade commands are pinned in
  [`upgrade_command_for`](../../crates/anvil-cli/src/commands/version.rs).
  Latest-release lookup uses an async `reqwest` call wrapped in a
  fresh tokio runtime (matching axoupdater's pattern); a 3-second
  timeout makes network failures non-fatal — the local version still
  prints. `--offline` skips the probe entirely for sandboxed CI
  environments. SemVer comparison is hand-rolled to ensure stable
  releases sort after pre-releases of the same core (`1.0.0 >
  1.0.0-rc1`); 11 unit tests pin the parse / order / install-method
  detection / per-method upgrade strings, and 3 integration tests
  exercise the human + JSON paths and confirm the command does not
  require auth.

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

### LAUNCH-015: Profile repo languages during activation

- **Intent:** A user running activation sees an explicit, honest accounting of
  which languages Anvil covers in their repository. The activation summary
  names each detected language and the coverage tier Anvil claims for it, so
  the user can judge fit without guessing from absence of findings.
- **Expected Outcome:** A repo language profile step runs as part of
  `anvil start` (and is reused by `anvil status` and `anvil doctor`). It
  inspects the working tree, classifies each detected language as
  `supported`, `partial`, or `unsupported` against the registered anchors and
  packs that ship in this release (TS supported, SQL partial pending SURFSQL
  Phase 1, Markdown partial pending MDGOV, Python and Rust unsupported as of
  this release), and produces a structured profile consumed by activation
  copy. Detection uses file extensions plus presence heuristics; vendored /
  generated paths (already filtered by the existing internal denylist) are
  not counted. Activation and tutorial copy never claim protection for an
  unsupported language; the JSON output exposes `repo_languages` with
  `{name, files_seen, coverage_tier, basis}` per entry. The coverage tier
  table lives in one place (a single registry, not duplicated in copy
  strings) so future anchors land in one edit.
- **Coordinates with:** LAUNCH-008 (the protection-state vocabulary the copy
  must respect), LAUNCH-014 (tutorial copy reuses the same coverage tiers),
  RELEASE-PLAN A5 (LANGTS / SURFSQL / OPSUP define which languages move from
  `partial` to `supported`).
- **Validation:** Integration test against a multi-language fixture asserts
  the profile classifies each detected language with the expected tier;
  snapshot test on activation copy confirms it lists detected languages and
  never asserts protection for an `unsupported` entry; JSON output schema
  test on `repo_languages`.
- **Confidence:** medium
- **Status:** Complete — landed in
  `crates/anvil-cli/src/activation/language_profile.rs`. The
  `LANGUAGE_REGISTRY` is the single-source-of-truth coverage table;
  `profile_repo(root)` walks the working tree (excluding vendored /
  generated paths) and returns a `RepoLanguageProfile` classifying
  each language as `supported` / `partial` / `unsupported`. Embedded
  in `ActivationDiagnostic.language_profile` and surfaced via human
  (per-language breakdown in `anvil status --verify`) and JSON
  (`repo_languages` array) renderers. The protection-state mapping
  collapses to `Unsupported` when `all_languages_unsupported &&
  highest_mcp < RestartRequired` (carve-outs for live MCP and
  one-restart-from-live preserved). Integration tests in
  `tests/status_verify_languages.rs` cover empty / TS-only /
  Python-only / mixed scenarios; vendored dirs do not bias counts.

---

### LAUNCH-016: Honour the language profile in scan and watch filters

- **Intent:** Files belonging to languages Anvil does not support in this
  release are not fed to language-specific antipattern checks, so users do
  not see false-positive findings on out-of-scope code (e.g. `.py` files
  flagged by JS-shaped rules).
- **Expected Outcome:** The default scan and watch file filters consult the
  repo language profile from LAUNCH-015. Files belonging to `unsupported`
  languages are excluded from language-specific antipattern checks unless
  the user explicitly opts in via config (`extensions:` override or an
  equivalent allow entry). Cross-language checks (secrets, etc.) still run
  on all files — only language-targeted antipattern checks are gated.
  Skipped files are recorded in the run summary so the behaviour is visible,
  not silent (`skipped: {language: count, reason: "unsupported"}`). The
  hardcoded extension allowlist in
  `crates/anvil-checks/src/antipattern/types.rs` becomes the fallback
  default for repos with no profile, and is overridden by the profile when
  one exists.
- **Coordinates with:** LAUNCH-015 (consumes the profile), LAUNCH-008 (the
  activation summary should reflect what was actually scanned vs skipped),
  `crates/anvil-kernel/src/watcher/filter.rs` (the existing internal
  denylist is preserved as-is and is not conflated with the user-facing
  language gate).
- **Validation:** Integration test on a fixture with `.ts` + `.py` files
  asserts: (a) the default behaviour scans `.ts` and skips `.py` for
  language-specific checks, (b) the secret scanner runs on both, (c) the
  run summary records the skip with language and count, (d) explicit
  opt-in (`extensions:` includes `.py`) reverses the language-specific
  skip without affecting the secret scan.
- **Confidence:** medium
- **Status:** Complete (with explicit hand-offs) —
  `language_profile::partition_for_language_specific_checks` is the
  canonical contract; it returns `(scannable, LanguageSkipLedger)`
  for any candidate file list, with the ledger keyed by language
  name and `reason: "unsupported"` for the v1 release.
  `services::sample_analyser` derives the ledger from the
  pre-filtered sample using the partition helper (so the ledger
  reflects what was actually skipped from the scan, not the broader
  repo); `commands::init` prints the skipped line if non-empty.
  Acceptance status:
  - (a) Default behaviour scans `.ts` and skips `.py` for
    language-specific checks: **met** by the existing
    `AntipatternCheckConfig::default().extensions` allowlist; the
    partition helper provides the visible ledger contract so the
    skip can be surfaced honestly when downstream PRs adopt it at
    sites without a pre-filter.
  - (b) Secret scanner runs on both: **hand-off**. The post-init
    activation path does not invoke the secret scanner; secret
    scanning happens via `commands::audit` and the MCP path. PR 3
    (LAUNCH-009) wires MCP secrets coverage; the
    cross-language-checks-still-run claim is preserved by leaving
    the partition helper out of secret-scan call sites.
  - (c) Run summary records the skip with language and count:
    **met** via `AnalysisOutcome.skipped_unsupported_languages` and
    the `repo_languages` array in `anvil status --verify --json`.
  - (d) Explicit `extensions:` opt-in to scan unsupported
    languages: **hand-off** to a follow-up PR that wires
    user-config-aware filtering through `commands::check`,
    `commands::watch`, and `commands::audit`. The partition helper
    is the seam; downstream consumers compose the user-config
    decision before invoking it.

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
- **Expected Outcome:** The mutual-exclusion guard at
  `crates/anvil-cli/src/commands/watch.rs:297-302` is removed. The
  most recent action's outcome (name, exit code, duration) surfaces
  as a single status-line footer below the existing 2x2 grid; no new
  pane, no kind glyph in History, no stderr capture in v1. Non-TUI
  mode is bit-for-bit identical (inherited stdio).
- **Spike (precondition):** Before committing to a UI surface for
  action output, do a short investigation of the action dispatch path
  to confirm whether non-blocking integration is local or requires
  reshaping the dispatcher. Record the decision in the task comment.
- **Spike outcome (2026-05-05):** The guard exists because
  `dispatch_action` spawns the child with `Stdio::inherit()` for
  stdout/stderr (`watch.rs:243-258`); inherited output collides with
  the Ratatui alternate-screen buffer. The dispatch loop today lives
  only in the non-TUI branch of `run()` (`watch.rs:405-460`); the TUI
  branch never sees `action`. Initial proposal: capture stderr tail,
  add `WatchData.action_history: Vec<ActionRun>`, render in History
  pane with a kind glyph.
- **Council revision (2026-05-05, plan-f684d971):** Standard pack
  (architect + pragmatic-lead + adversarial) all returned COUNTER.
  Convergent findings:
  - **Scope creep.** The audit finding is only "users must choose
    dashboard or automation". `Vec<ActionRun>` + History glyph +
    stderr tail solves a richer problem than the finding asked for
    and bundles cross-crate type movement into the same PR as
    LAUNCH-006.
  - **Cross-crate boundary undefined.** `anvil-tui` does not depend
    on `anvil-cli`; `ActionRun` cannot be referenced by `WatchData`
    while living in `anvil-cli` without inverting the dep edge.
  - **Sole-writer invariant.** `WatchEventAdapter` is the only
    writer of `WatchData` today. The original plan introduced a
    second writer and did not define how `data.status` and
    `WatchStats` arithmetic stayed isolated from action outcomes.
  - **Worker shutdown leak (pre-existing).** Today's fire-and-forget
    worker (`watch.rs:443`) is also leaked on Ctrl-C — holds the
    child stdio and the rerun atomics across the parent's exit.
    The dispatcher refactor must fix it.
  - **Unfalsifiable test.** "Render loop continues ticking" passes
    trivially; doesn't prove the surface redraws on action arrival.
  - **Status-icon flip.** Action failure that writes `data.status`
    would flip the Status pane to Failing despite no kernel
    violation — TUI theatre regression.

  Plus one finding the council missed: `build_action_command`
  (`watch.rs:236-258`) appends `--no-tui` to the child only when the
  parent's `--no-tui` flag is set. Drop the guard naively and a TUI
  parent spawns a TUI child — two Ratatui sessions fight over the
  same alternate-screen. The dispatcher must force `--no-tui` on the
  child whenever the parent is in TUI mode, regardless of
  `global.no_tui`.
- **Revised plan:**
  1. **Drop** the guard at `watch.rs:297-302`.
  2. Extract dispatch into an `ActionDispatcher` struct owning
     `action_running` / `action_pending` atomics, the worker
     `JoinHandle`, and a cancellation token. On `Drop` (or explicit
     `shutdown()`): cancel, `Child::kill()` if a child is in flight,
     then join. Closes the pre-existing Ctrl-C leak.
  3. **`--no-tui` propagation.** Dispatcher forces `--no-tui` on the
     child whenever the parent is in TUI mode, regardless of the
     parent's `global.no_tui`. Without this, two TUIs collide.
  4. In TUI mode, switch the child to `Stdio::piped()`; capture
     **only** action name, exit code, and duration. Discard stdout
     and stderr in v1 — the audit finding does not require stderr
     surfacing. Non-TUI mode keeps inherited stdio bit-for-bit.
  5. **Channel seam — wrapping enum, not a second receiver.** Define
     `WatchLoopEvent { Engine(EngineEvent), Action(ActionResultLine) }`
     and change `run_watch` / `watch_loop` to consume
     `Receiver<WatchLoopEvent>`. The welcome-hub callsite
     (`tui.rs:352-364 run_watch_in`) is a type swap, not a signature
     change. Action sender uses `sync_channel(1)` so producer
     back-pressure is implicit (no unbounded buffer).
  6. **Single-writer preserved.** `ActionResultLine` is a pure data
     type living in `anvil-tui::surfaces::watch` (consumer side, no
     CLI dependency edge). The adapter gains
     `WatchEventAdapter::handle_action_result(&mut self, line:
     &ActionResultLine, data: &mut WatchData)` — the only path that
     writes `data.last_action`. Adapter remains the sole writer.
  7. **Surface.** New scalar field `WatchData.last_action:
     Option<ActionResultLine>` (NOT `Vec`). Render as a single
     footer line below the 2x2 grid: `[*] gate (1.2s)` on success,
     `[x] gate (1.2s, exit 1)` on failure. ASCII-only to match the
     existing watch surface labels (`watch.rs:483-493`).
  8. **Isolation invariant.** `handle_action_result` writes ONLY
     `data.last_action` and dirties the surface. It MUST NOT touch
     `data.status`, `data.stats.*`, or `data.history`. Unit test
     asserts each field unchanged across an action result.
  9. Non-TUI mode: dispatcher's TUI sender is `None`; behaviour stays
     bit-for-bit (inherited stdio, no capture).
  10. **Deferred to LAUNCH-002b** (against TUIDASH-009's inheritance
      seam): `Vec<ActionRun>` history surface, kind glyph rendering,
      stderr tail capture, scrollable action log.
- **Validation:**
  - Unit test on `ActionDispatcher`: rerun-pending atomic; on
    `shutdown()`/`Drop` with an in-flight child, the child is killed
    and the worker joins.
  - Unit test on `build_action_command` (or dispatcher wrapper):
    when caller is in TUI mode, child cmd includes `--no-tui`
    regardless of `global.no_tui`.
  - Unit test on `WatchEventAdapter::handle_action_result`: writes
    `data.last_action`, dirties the surface, and leaves
    `data.status`, `data.stats.pass_rate`, `data.stats.total_runs`,
    `data.stats.avg_duration_ms`, and `data.history` unchanged.
  - Smoke: drive the `WatchLoopEvent::Action` arm of the multiplex
    with a fake `ActionResultLine` and assert `state.dirty == true`
    afterwards. The unfalsifiable "render loop ticks" assertion is
    dropped.
- **LOC budget:** ~150-200 (dispatcher + cancellation ~80;
  ActionResultLine + adapter handler + tests ~50; channel enum +
  loop / run_watch_in update ~40; footer renderer ~20).
- **Confidence:** medium (council revised; cross-crate boundary,
  channel seam, single-writer invariant, and `--no-tui` propagation
  now explicit).
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
- **Expected Outcome:** `anvil start` is promoted from a clap alias for
  `welcome` to its own `Commands::Start` variant backed by a thin
  orchestrator that composes only the read-safe / idempotent
  primitives shipped today: ensure config (init if absent), first-scan,
  and `activation::verify`. The orchestrator lands at whatever
  `ProtectionState` `verify` reports — `needs_action` for fresh repos
  with no editor wired, `unsupported` for unsupported-language repos,
  `ready_restart_required` once MCP install is wired (post LAUNCH-009).
  `anvil welcome` remains the explicit menu/tutorial surface. No new
  top-level `activate` or `connect` command is added for the MVP.
- **Council revision (2026-05-05):** Standard pack interrogated the
  original "compose init/first-scan/MCP/doctor/status/watch" framing.
  Adversarial returned 3 criticals tied to composing those unsafe
  primitives in v1: MCP install `bail!`s on malformed editor JSON,
  first-run marker semantics undefined, alias-breakage migration
  unspecified. Architect + pragmatic-lead converged on a thinner
  scope. Adopted: orchestrator runs ONLY safe / idempotent primitives.
  MCP install, doctor composition, and watch-fallback spawn are
  deferred to LAUNCH-009 / LAUNCH-011 — those tasks plug into the
  diagnostic stubs `activation::verify` already exposes.
- **Revised plan:**
  1. Remove `#[command(alias = "start")]` from `Commands::Welcome` and
     add a `Commands::Start(StartArgs)` variant in `crates/anvil-cli/src/main.rs`.
  2. New thin `crates/anvil-cli/src/commands/start.rs` that calls
     `activation::orchestrator::run`.
  3. New `crates/anvil-cli/src/activation/orchestrator.rs`:
     - Probe config status. If `.anvilrc` absent, call existing
       `commands::init::run`. If valid, skip init (idempotent rerun).
       If invalid, surface as `ProtectionState::Error` via verify.
     - Run `services::sample_analyser::run_post_init_analysis` for
       the first-scan step (LAUNCH-004 primitive — read-only,
       budget-bounded).
     - Call `activation::verify(repo_root)`; render via the existing
       `activation::render` module.
  4. **NOT included in v1:** MCP install (LAUNCH-009 territory),
     watch spawn (LAUNCH-011), doctor composition. Each would
     `bail!` on edge cases that turn the composed flow into an
     unrecoverable trace; deferring is the safe v1 cut.
  5. **First-run marker invariant:** `start` does NOT write
     `.anvil/first-run`. `welcome` keeps sole ownership of that
     marker.
  6. **`--json` parity:** reuse the `anvil status --verify --json`
     shape (LAUNCH-012 already shipped this) so CI consumers see a
     consistent schema between `start` and `status`.
  7. **Exit code:** 0 on every state except `error` (matches the
     project's "warnings over blocks, exit 0 by default" rule from
     `architecture.md`).
  8. **No `--verify` flag on `start`** — `anvil status --verify`
     already covers that need (LAUNCH-012); a duplicate flag would
     fragment the surface.
  9. **Behavioural promotion, not breaking change.** `anvil start`
     today is an undocumented clap alias. Pre-1.0 (`v0.5.1-beta`)
     this is a sub-major behavioural change; CHANGELOG carries the
     note, no deprecation cycle needed.
- **Coordinates with:** RMCP / LAUNCH-009 (MCP probes plug into the
  diagnostic's `mcp` map), LAUNCH-011 (watch fallback adds the
  `watching` final state), LAUNCH-010 (baseline plug-in to
  diagnostic), LAUNCH-014 (tutorial copy reuses the same vocabulary).
- **Validation:**
  - Subprocess test on a fresh temp repo: `anvil start` exits 0
    with a `ProtectionState` literal in stdout.
  - Subprocess test that `anvil welcome` still launches the
    welcome surface unchanged.
  - Unit test on `orchestrator::run`: matrix covering config
    absent (calls init), config valid (skips init), and the
    fresh-repo `needs_action` final state.
- **LOC budget:** ~250-300 production + ~150 test = ~450 total.
- **Confidence:** medium (was medium; council resolved the scope
  ambiguity that was the source of the risk).
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
- **Status:** Complete — landed alongside LAUNCH-012 in
  `crates/anvil-cli/src/activation/`. The shared `ProtectionState`
  vocabulary plus the `ActivationDiagnostic` shape now back
  `anvil status` (default + `--verify`) human and JSON output, with
  unit + CLI integration tests covering each required scenario.
  Downstream PRs (LAUNCH-006, -009, -010, -011, -015) extend the
  diagnostic with their probe layers; the contract is locked so they
  cannot add ad-hoc states.

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
- **Status:** Complete — landed alongside LAUNCH-008 via
  `anvil status --verify`. PR 1 (LAUNCH-006) is expected to add the
  `anvil start --verify` form that forwards to the same backend. The
  current implementation honours each layer in `ActivationDiagnostic`
  separately, never writes config, and is idempotent — covered by
  `crates/anvil-cli/tests/status_verify.rs`. Live MCP tier and watch
  liveness probes are stubbed today and plug in via PR 3.

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
