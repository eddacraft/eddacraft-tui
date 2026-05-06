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
| LAUNCH | —     | In Progress | 17/18    |

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

> Status: In Progress. LAUNCH-001, LAUNCH-002, LAUNCH-003, LAUNCH-004,
> LAUNCH-005, LAUNCH-006, LAUNCH-007, LAUNCH-008, LAUNCH-009,
> LAUNCH-009.5, LAUNCH-009.6, LAUNCH-010, LAUNCH-012, LAUNCH-013,
> LAUNCH-014, LAUNCH-015, and LAUNCH-016 are complete; LAUNCH-011
> stays Todo until picked up.

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
- **Status:** Complete — landed via PR #1294
  ([`feat/launch-014-tutorial`](https://github.com/eddacraft/anvil-001/pull/1294)).
  New `TutorialPath::ProtectionLoop` variant is listed first and pre-
  selected in `TutorialState::new()`, so hitting Enter on `anvil
  tutorial` lands the user on a five-step value-first walk:
  protection-loop intro → fixture description → simulated check
  result → activation-state vocabulary explainer → run `anvil start
  --verify`. The four legacy paths (Policy / Architecture / Drift /
  CI) remain as the deeper-learning track. Copy invariants are
  test-pinned: `protection_loop_copy_uses_activation_state_vocabulary`
  enforces the five user-actionable LAUNCH-008 literals (`protecting`,
  `ready_restart_required`, `watching`, `needs_action`,
  `unsupported`) are referenced by name — the sixth `error` variant
  is intentionally not asserted here because the tutorial does not
  pre-walk failure modes;
  `protection_loop_copy_does_not_claim_pre_write_protection` rejects
  present-tense protection claims and requires the final step to
  point at `anvil start --verify`. Round-1 review feedback closed
  the watch-fallback over-claim — the final step now enumerates
  what `--verify` actually probes today (config, MCP entries,
  baseline, language profile) and explicitly notes watch-liveness
  probing is unwired pending LAUNCH-011. `filter_by_domain` returns
  every finding for `ProtectionLoop` (no narrowing in v1; pinned by
  `filter_by_domain_protection_loop_gets_all_and_preserves_metadata`).

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
- **Status:** Complete — landed via PR #1279
  ([`launch/a1-start-entrypoint`](https://github.com/eddacraft/anvil-001/pull/1279)).
  The mutual-exclusion guard at the old `watch.rs:297-302` is gone;
  `ActionDispatcher` (now `crates/anvil-cli/src/commands/watch.rs:287-420`)
  owns the worker, cancellation token, and in-flight child, and joins
  on `Drop` so Ctrl-C no longer leaks the prior fire-and-forget
  worker. `build_action_command` forces `--no-tui` on the child
  whenever the parent is in TUI mode, regardless of `global.no_tui`,
  so two Ratatui sessions can't fight over the alt-screen.
  `WatchLoopEvent { Engine, Action }` multiplexes engine + action
  events through the existing single receiver; `ActionResultLine`
  lives in `anvil-tui::surfaces::watch` (consumer-side, no CLI
  dependency edge); `WatchEventAdapter::handle_action_result` is the
  only writer of `WatchData.last_action` and asserts in tests that
  `data.status`, `data.stats.*`, and `data.history` are unchanged
  across an action result. Footer renders as `[*] gate (1.2s)` /
  `[x] gate (1.2s, exit 1)` below the 2x2 grid; non-TUI mode keeps
  inherited stdio bit-for-bit.

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
  7. **Exit code:** 0 on every state including `error` — matches the
     project's "warnings over blocks, exit 0 by default" rule
     (`architecture.md`). The literal `state: error` in stdout is the
     signal; CI consumers parse the JSON `state` field, not the exit
     code.
  8. **`--verify` flag** mirrors `anvil status --verify` (LAUNCH-012's
     comment in `commands/status.rs` already anticipates this). Read-
     only equivalent: skips init / first-scan, just probes and
     renders. No fragmentation — both surfaces forward to the same
     `activation::verify` backend.
  9. **`--json` implies read-only.** Init's own JSON output (the
     `AnvilConfig` record it prints in `--json` mode) would otherwise
     concatenate with the activation diagnostic JSON and break
     parseable consumers. JSON mode behaves like `--verify` — single
     activation diagnostic JSON document on stdout.
  10. **Behavioural promotion, not breaking change.** `anvil start`
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
- **Status:** Complete — landed via PR #1280
  ([`launch/a1-anvil-start`](https://github.com/eddacraft/anvil-001/pull/1280)).
  `Commands::Start(StartArgs)` is its own variant in
  `crates/anvil-cli/src/main.rs`; the `#[command(alias = "start")]`
  on `Commands::Welcome` was removed and `welcome` keeps sole
  ownership of the `.anvil/first-run` marker. Thin
  `crates/anvil-cli/src/commands/start.rs` delegates to
  `activation::orchestrator::run`. As shipped under this task, the
  orchestrator composed only the read-safe / idempotent primitives
  the plan above pinned: `verify_with_home` → init-if-absent
  (LAUNCH-004's `services::sample_analyser` runs inline through
  `init::run_in`) → re-verify. MCP install was deliberately deferred
  here per the council revision; LAUNCH-009 part 2 (PR #1284, in the
  same release window) subsequently extended the orchestrator with
  the install step, which is why current `crates/anvil-cli/src/activation/orchestrator/mod.rs`
  includes a write-side step inside the same composition. `--verify`
  and `--json` short-circuit to the read-only path so init's own
  JSON record can't concatenate with the activation diagnostic. The
  diagnostic is rendered via the existing `activation::render_*`
  module so `start`, `status --verify`, and the JSON schema all
  share one literal `ProtectionState` vocabulary. Server-startable
  spawn probes, watch fallback, and doctor composition remain
  deferred to LAUNCH-009.5 / LAUNCH-011 as planned.

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
  the exact tier reached. Existing editor config is parsed before
  modification, written atomically, and left untouched on parse failure
  or unsafe drift. Rule/instruction files are not edited.
- **Council revision (2026-05-05):** Standard pack interrogated three
  questions: (a) v1 scope, (b) the user's expansion request to
  OpenCode/Zed/VS Code, (c) hosted-MCP-server pre-investment. All
  three reviewers converged on staying at Cursor + Claude Code for v1
  on the grounds that:
  - The 2026-05-03 activation council explicitly banned VS Code,
    Windsurf, Copilot CLI, Codex CLI; reversing requires fresh
    council, not a user request.
  - The existing `mcp_config.rs::Target::Vscode` writes to the wrong
    file (`.vscode/settings.json` with `mcp.servers`) — VS Code 1.99+
    moved MCP to `.vscode/mcp.json` with `servers`. Shipping that
    today claims success while doing nothing.
  - Zed's `experimental.context_servers` is upstream-experimental
    with jsonc-with-comments support that needs a comment-preserving
    editor (cost > rest of LAUNCH-009 combined).
  - OpenCode has no MCP-protocol-compliance evidence in the repo or
    docs.
  - Adversarial flagged 5 critical v1 spec gaps (RestartRequired
    detection mechanism, Windows TOCTOU, drift tri-state, LiveValidation
    INTD-only, JSONC support for Claude Code) — all addressed in the
    revised plan below.
  - Architect proposed introducing a `McpClient` trait in v1 itself
    (~150 LOC) so future expansion is one-impl-per-editor rather
    than a refactor under release pressure.
- **Hosted-MCP-server pre-investment** (approved 2026-05-05): the
  trait shape and diagnostic schema reserve room for future remote
  transports without baking in any code that uses them. v1 ships only
  `Stdio` transport but the types are future-shaped:
  - `AnvilEntry` is an enum with one variant today (`Stdio { command,
    args, env }`); future adds `RemoteSse` / `RemoteHttp` additively.
  - `McpTransport` enum reserved with `Stdio` variant; doc names the
    future variants.
  - Diagnostic JSON: each client's tier carries a `transport` tag
    (`{"tier": "config_present", "transport": "stdio"}`) so the
    schema doesn't need migration when hosted lands.
  - `verify_tier(&AnvilEntry)` is the trait method; for stdio it
    spawns a probe, for future remote it would HTTP-GET. Trait
    signature is transport-agnostic.
  - No actual hosted-server code, no auth flow, no token UX — those
    ship when the hosted server itself ships.
- **Revised plan:**
  1. Introduce `McpClient` trait in
     `crates/anvil-cli/src/activation/mcp_client.rs`:
     - `id() -> McpClientId`
     - `config_paths(workspace, home) -> Vec<ConfigCandidate>`
     - `parse(raw: &str) -> Result<ParsedConfig, ParseError>`
     - `merge(parsed, entry: &AnvilEntry) -> Result<MergedConfig, DriftClass>`
       where `DriftClass` is `UpToDate | SafeDrift | UnsafeDrift`
     - `render(merged) -> String`
     - `verify_tier(parsed, entry: &AnvilEntry) -> McpTier`
     - `restart_hint() -> &'static str`
  2. Implement the trait for `Cursor` and `ClaudeCode`. Two impls;
     ~75 LOC each before tests.
  3. **Tier transition spec** (closes adversarial finding 1):
     - `ConfigAbsent → ConfigPresent`: the anvil entry parses cleanly
       in the file.
     - `ConfigPresent → RestartRequired`: ALWAYS emit RestartRequired
       on a fresh write — we cannot observe restart without IPC.
       Demote only if the user has explicit evidence (manual `anvil
       status --verify` after restart).
     - `RestartRequired → ServerStartable`: `verify_tier` spawns
       `anvil mcp serve --stdio` against the entry and observes a
       clean MCP handshake within a 1-second budget.
     - `ServerStartable → LiveValidation`: **OUT OF SCOPE for v1.**
       LiveValidation requires INTD daemon evidence (RMCP issue
       #1197). McpTier::LiveValidation is reserved for a future PR
       that wires INTD into the diagnostic; LAUNCH-009 caps at
       ServerStartable.
  4. **Drift tri-state** (closes adversarial finding 3):
     - `UpToDate`: existing entry matches our `AnvilEntry` byte-for-byte.
     - `SafeDrift`: same shape but a different anvil-binary path
       (e.g. user has nix-managed anvil); update if the new path
       is reachable, else flag UnsafeDrift.
     - `UnsafeDrift`: existing entry's `command` field doesn't
       resolve to anvil, OR an unrecognised key shape. Don't write;
       diagnostic emits `state: error` with detail naming the drift.
  5. **Windows concurrency** (closes adversarial finding 2): wrap
     the read-merge-write cycle in a side-car `.lock` file (POSIX
     `flock` / Windows `LockFileEx`). Idempotent merge as the safety
     net if locking is unavailable.
  6. **JSONC support** (closes adversarial finding 5): empirically
     verify Claude Code's settings.json format on a real install
     before shipping. If comments are present, add `jsonc-parser`
     as a dependency and use it for parse-without-strip-comments.
  7. **`AnvilEntry` enum** (hosted-future pre-investment):
     ```
     enum AnvilEntry {
         Stdio { command: PathBuf, args: Vec<String>, env: BTreeMap<String, String> },
         // RemoteSse / RemoteHttp reserved for hosted-server future.
     }
     ```
  8. **`McpTransport` enum** (hosted-future pre-investment):
     ```
     enum McpTransport {
         Stdio,
         // RemoteSse / RemoteHttp reserved.
     }
     ```
  9. **Diagnostic JSON schema** (hosted-future pre-investment): each
     client's entry becomes `{"tier": "...", "transport": "stdio"}`
     instead of just `"..."`. Renderer + tests updated.
  10. **Drop dead/broken Target paths** (closes architect cleanup):
      - Remove `Target::Windsurf` from `mcp_config.rs` — council-banned.
      - Remove or feature-flag `Target::Vscode` — writes the wrong
        file shape. Keep test fixtures as expected-failure regression
        guards.
  11. **Plug orchestrator into `activation::verify`**: replace the
      empty `BTreeMap::new()` stub at `diagnostic.rs:242` with a real
      probe that calls each `McpClient::verify_tier` and assembles
      the `mcp` map.
- **NOT included in v1:**
  - **VS Code, Zed, OpenCode** — see council revision above. Each
    has a specific blocker the council documented; revisit once the
    blocker resolves (VS Code MCP exits experimental + verified path,
    Zed schema exits experimental, OpenCode protocol-compliance
    evidence).
  - **`LiveValidation` tier** — INTD-only; future PR.
  - **Hosted MCP server itself** — separate workstream.
  - **Token / auth / OAuth UX** — wait for the hosted server.
  - **Co-existence policy** (local stdio + remote on same client) —
    wait until we know what users want.
- **Coordinates with:** RMCP follow-ups for client config paths
  (especially #1195 — Claude Code path-gap), `anvil mcp install
  --verify`, the `anvil_validate_write` tool contract, and INTD for
  the future `LiveValidation` tier.
- **Validation:**
  - Fixture-backed tests for each `McpClient` impl: config absent,
    config written, idempotent rerun, parse failure, safe drift,
    unsafe drift, restart-required, server-startable. 8 scenarios ×
    2 editors = 16 fixtures.
  - Unit test on the tier-promotion ladder for each editor.
  - Concurrent-invocation regression test (Windows-skip on Unix
    since `flock` is reliable there; Windows-only test if the lock
    file approach is used).
  - Diagnostic JSON schema test asserting the `transport` tag
    appears on every client entry.
  - Trait contract test: `AnvilEntry::Stdio` round-trips through
    every impl's `merge()` and `render()` cleanly.
- **LOC budget:** ~750-900 production + ~250-350 test = ~1000-1250
  total. Originally planned as a single PR; split into two for review
  scope. Part 1 (trait + read probe) merged as PR #1283; part 2
  (install path + picker + orchestrator integration) merged as PR
  #1287. The remaining spawn-probe step (`RestartRequired →
  ServerStartable`) and the cleanup follow-ups live in the new
  LAUNCH-009.5 task below.
- **Confidence:** medium (council resolved scope; spec gaps closed
  in the plan).
- **Status:** Complete — Cursor and Claude Code MCP entries are
  installed safely with drift handling, atomic writes, and
  `ConfigAbsent → RestartRequired` tier promotion. Spawn probe and
  cleanup deferrals tracked in LAUNCH-009.5.

---

### LAUNCH-009.5: MCP install follow-ups and spawn probe

- **Intent:** Close the remaining `LAUNCH-009` deltas — the spawn
  probe that promotes `RestartRequired → ServerStartable`, the
  dead-Target cleanup, and the four council deferrals from the
  LAUNCH-009 part-2 review that were judged "not blocking for v1
  but worth fixing before A1 release week."
- **Expected Outcome:**
  1. `anvil mcp serve --stdio` is spawned against each installed
     entry and a clean MCP handshake is observed within a 1-second
     budget; on success the per-client tier promotes from
     `RestartRequired` to `ServerStartable`.
  2. `Target::Windsurf` and the broken `Target::Vscode` paths in
     `commands/mcp_config.rs` are removed (council-banned in
     LAUNCH-009 v1 scope; kept in part-2 because the diff was
     already large).
  3. Council follow-ups from PR #1284 review (LAUNCH-009 part 2):
     - **Symlink-parent guard:** refuse to install when the target
       file's parent directory is itself a symlink (`lstat` check
       before `tempfile_in`). Closes kernel MAJOR; current threat
       model requires attacker-controlled HOME, but the cost is one
       call.
     - **Hosted-transport `type` field handling:** `classify_drift_by_args`
       must classify a `type: sse` / `type: http` existing entry as
       `UnsafeDrift` rather than falling through to `SafeDrift` and
       overwriting with the `stdio` shape. Lands together with the
       `RemoteSse`/`RemoteHttp` variants in the hosted-server
       workstream — defer until either lands.
     - **SafeDrift extra-key behaviour:** document the wholesale
       replacement policy in the user-facing copy (or migrate to a
       per-key merge that preserves unrecognised top-level keys in
       the anvil entry). Default in v1 is wholesale replace; revisit
       once we observe real configs in the wild.
     - **Install report in `--json`:** the `render_json` shape
       intentionally omits per-client install outcomes (only the
       post-install diagnostic state is in the JSON contract).
       Decide whether CI dashboards need an `install:` block; either
       extend the schema or document the limitation in `render_json`'s
       doc comment.
  4. Telemetry counters for install attempts / outcomes (deferred
     pending a metrics framework in `anvil-cli`).
- **NOT included:** `LiveValidation` tier promotion (still INTD-only,
  separate PR), hosted MCP server itself, OAuth UX.
- **Coordinates with:** the same surfaces as LAUNCH-009 plus
  RMCP-level handshake helpers.
- **Validation:**
  - Spawn probe round-trip integration test (Cursor + Claude Code
    fixture configs).
  - Symlink-parent refusal unit test.
  - `--json` install schema test (whichever direction the decision
    goes — extended block or documented omission).
- **LOC budget:** ~250-400 production + ~150-200 test.
- **Confidence:** medium.
- **Status:** Complete — split into two PRs:
  - **Cleanup half** (PR #1291): dropped `Target::Windsurf` and
    `Target::Vscode`, added the symlink-parent guard (scoped to MCP
    install path per council review), documented the SafeDrift
    wholesale-replace policy, and clarified the `--json`
    install-report omission contract.
  - **Spawn-probe half** (this PR): added
    `mcp_client::probe_startable` and wired it into `verify_with_home`
    via `probe_handshake_for_observability`. The probe drives a real
    JSON-RPC `initialize` handshake against the installed binary
    within a 1-second budget. **Tier promotion deferred to LAUNCH-009.6**
    (see deviation note below) — the probe runs purely for tracing
    observability in v1.

  **Tier-promotion deviation:** the original spec described
  `RestartRequired → ServerStartable` as a promotion. The actual
  `McpTier` enum and the diagnostic test harness deliberately position
  `ServerStartable` as a **weaker** tier than `RestartRequired`
  (it means "server runs but no client wiring detected"). Promoting
  on probe success would lose information and break the existing
  `protection_state()` mapping. Reconciling the tier semantics
  is a discrete refactor and lives in **LAUNCH-009.6** below.

  Hosted-transport `type` field handling, telemetry counters, and
  the JSON install-block decision remain deferred per the original
  task body.

---

### LAUNCH-009.6: Reconcile MCP tier semantics

- **Intent:** Resolve the `ServerStartable` / `RestartRequired` ladder
  inconsistency surfaced during LAUNCH-009.5 implementation so the
  spawn probe can graduate from observability-only to a real tier
  promotion.
- **Background:** the LAUNCH-008 council placed `ServerStartable`
  *below* `RestartRequired` in the `McpTier` enum on the reading that
  ServerStartable means "server can spawn but no client wiring is
  detected". The LAUNCH-009 spec text described the spawn probe as
  promoting `RestartRequired → ServerStartable`, which conflicts with
  this ordering: a successful probe of an installed entry would lose
  the "config wired and matching" information by sliding the tier
  back down the ladder. LAUNCH-009.5 ships the probe under
  observability-only semantics to avoid the regression; this task
  fixes the ambiguity end-to-end.
- **Expected Outcome:** one of:
  1. **Reorder the enum** so `ServerStartable > RestartRequired`,
     update `protection_state()` to treat `ServerStartable` as at
     least as strong as `RestartRequired`, refresh the existing
     diagnostic tests that encode the older reading, and let
     `probe_handshake_for_observability` promote on success.
  2. **Add a new tier** between `RestartRequired` and `LiveValidation`
     (e.g. `RestartHandshakeVerified`) that captures "config wired
     AND server starts" without overloading `ServerStartable`.
- **Coordinates with:** LAUNCH-009.5 (probe code was already in place;
  this task swaps the observability-only path for a real promotion
  path), LAUNCH-008 (tier vocabulary owner), and any future INTD-driven
  `LiveValidation` work.
- **Validation:**
  - Existing `watch_running_plus_server_startable_does_not_overclaim`
    and `server_startable_without_watch_falls_to_needs_action` tests
    are updated or replaced with the new semantic.
  - End-to-end: `anvil status --verify` after install +
    successful probe lands at the chosen elevated state (likely
    `ReadyRestartRequired` until LiveValidation lands).
- **LOC budget:** ~50-150 production + ~50-100 test (pure refactor;
  most of the cost is updating tests that encode the old reading).
- **Confidence:** medium.
- **Status:** Complete — implemented the additive tier option as
  `McpTier::RestartHandshakeVerified`, preserving `ServerStartable` as
  the weaker "server can spawn without confirmed client wiring" tier.
  `activation::verify` now promotes each `RestartRequired` client to
  `RestartHandshakeVerified` only after that client's installed entry is
  extracted and its configured command completes the MCP initialise
  handshake. If extraction falls back to `current_exe`, the probe stays
  informational and does not promote. `protection_state()` treats the
  new tier as `ReadyRestartRequired`, not `Protecting`; `LiveValidation`
  remains the only `Protecting` evidence. Validation covers the new
  tier in activation diagnostic unit tests and the end-to-end
  `anvil status --verify` spawn-probe integration.

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
- **Status:** Complete — landed via PR #1293
  ([`feat/launch-010-baseline`](https://github.com/eddacraft/anvil-001/pull/1293)).
  New `crates/anvil-cli/src/activation/baseline.rs` defines the
  on-disk schema (`Baseline` / `BaselineCounts` with `schema_version`,
  `created_at`, `fingerprints` set, per-kind counts), the atomic
  writer (tempfile + persist), and a fingerprint-based reader with
  schema-version guard. Fingerprints are namespaced by kind
  (`antipattern:` vs `secret:`) so a same-line collision can't merge
  across check kinds; secret paths are normalised to repo-relative
  forward slashes so baselines round-trip across checkouts and OSes.
  `services::sample_analyser::run_baseline_scan` re-uses the
  LAUNCH-004 sample selection and runs both antipattern and secret
  scanners on the same files. Orchestrator step 1b writes the
  baseline once on first activation, idempotent on re-runs;
  failures log and continue rather than blocking activation. The
  diagnostic gains `baseline_summary: Option<BaselineSummary>` and
  the JSON schema gains a `baseline` object alongside the existing
  `baseline_present` boolean (additive — old consumers keep
  working). Render copy is honest about the deferred wiring:
  "future scans will diff against this set as wiring lands". The
  contract surface (`Baseline::contains_warning`,
  `Baseline::contains_secret`) ships here so downstream PRs can
  wire watch / check / audit filtering without further refactor.
  Round-1 review remediation closed eight Copilot findings on path
  portability, error-variant naming, counts/fingerprints
  documentation, and over-claiming copy.

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
- **Status:** In Progress — branch `launch/a1-watch-fallback`. Diagnostic
  gains explicit MCP pre-write predicates so surfaces can label fallback
  honestly; `verify` sets `WatchTier::Offered` when MCP cannot attach;
  `anvil start --watch` runs the kernel watcher inline scoped to the
  repo when MCP is below `LiveValidation`.

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
