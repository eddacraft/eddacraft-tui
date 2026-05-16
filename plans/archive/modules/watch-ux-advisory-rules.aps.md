# Watch UX and Advisory Rules

| ID       | Owner  | Status      | Progress |
| -------- | ------ | ----------- | -------- |
| WATCHUX | @aneki | Complete | 8/8 done |

**Last reviewed:** 2026-05-14 (`WATCHUX-001` through `WATCHUX-004` reconciled
against merged PR #1497; `WATCHUX-005` through `WATCHUX-007` merged via PR #1524;
`WATCHUX-008` implemented on `feat/watchux-008-config-cache`.)

Created from beta-user incident timeline and
[`plans/specs/2026-05-13-watch-warmup-and-advisory-rules.md`](../../specs/2026-05-13-watch-warmup-and-advisory-rules.md).
Module starts In Progress because urgent beta remediation is already underway on
`fix/beta-user-bug`; follow-up UX/config work remains sequenced below.)

## Purpose

Make first-run and save-time watch behaviour understandable, fast-feeling, and
honest for large real repositories. The beta incident showed four coupled UX
failures: curl upgrade did not detect Homebrew, audit scanned local agent
worktrees, watch looked hung during expensive setup, and initial-snapshot
diagnostics were rendered as `Failing` even when they were advisory baseline
noise.

WATCHUX owns the user-facing correction of those behaviours:

- first-run installer and activation copy should tell users what is happening
- audit and watch should ignore local tool/worktree noise by default
- watch should open immediately and show warm-up progress
- initial graph construction should not be treated as new user changes
- advisory findings should render as warnings unless config opts into enforcement
- start/status/config surfaces should show which rules are configured and how
  they are enforced

## In Scope

- Homebrew-aware curl installer preflight
- Built-in ignored-directory policy for audit/check/gate/drift/sample/watch
- Watch initial-scan semantics: build graph and readiness state without emitting
  policy findings for existing repo contents
- Watch startup visibility and progressive warm-up UX
- Advisory vs enforcement language in the watch TUI
- Rule-mode configuration shape (`off` / `warn` / `enforce`)
- Start/status config summary copy
- Config command surface for showing, setting, and converting rule modes
- Warm-up cache design and implementation for later fast watch entry

## Out of Scope

- Cloud policy distribution or hosted rule management
- Organisation-level policy hierarchy
- GitHub App / server-side branch protection integration
- Full graph-v2 persistence beyond the watch warm-up accelerator
- New language support beyond existing parser/check surfaces

## Interfaces

- **Depends on:**
  - `install.sh` public curl installer
  - `crates/anvil-cli/src/commands/audit.rs`
  - `crates/anvil-cli/src/commands/start.rs`
  - `crates/anvil-cli/src/commands/watch.rs`
  - `crates/anvil-cli/src/util.rs`
  - `crates/anvil-kernel/src/watch.rs`
  - `crates/anvil-kernel/src/watcher/filter.rs`
  - `crates/anvil-tui/src/surfaces/watch/*`
  - MLP baseline and config work (`anvil-config`, `anvil-baseline`)
- **Exposes:**
  - clearer installer upgrade behaviour
  - lower-noise audit/watch defaults
  - progressive watch warm-up state
  - explicit advisory/enforcement rule modes
  - config visibility from start/status/config commands

## Tasks

### WATCHUX-001: Homebrew-Aware Curl Installer Preflight

- **Intent:** Stop the curl installer from running the standalone installer when
  Anvil is already installed via Homebrew.
- **Expected Outcome:** `install.sh` detects `/opt/homebrew/bin/anvil`,
  `/usr/local/bin/anvil`, and symlinked `Cellar/anvil/.../bin/anvil` before
  download; it exits successfully with `brew upgrade eddacraft/tap/anvil`
  guidance.
- **Files:** `install.sh`, `scripts/install.test.sh`
- **Validation:** `bash scripts/install.test.sh`
- **Status:** Complete
- **Evidence:** Merged via PR #1497 (`fix(watch): reduce first-run beta noise`);
  `install.sh` now detects direct Homebrew binaries and Cellar symlinks before
  download and prints `brew upgrade eddacraft/tap/anvil` guidance.

### WATCHUX-002: Shared Local-Noise Ignore Policy

- **Intent:** Prevent audit/watch and adjacent walkers from scanning local
  generated, cache, and agent worktree/tool-state directories by default.
- **Expected Outcome:** Shared CLI ignore list includes `.claude`, `.opencode`,
  `.gemini`, `.serena`, `.worktrees`, generated/cache dirs, and existing build
  outputs; audit uses the shared helper; the kernel watcher mirrors the policy.
- **Files:** `crates/anvil-cli/src/util.rs`,
  `crates/anvil-cli/src/commands/audit.rs`,
  `crates/anvil-kernel/src/watcher/filter.rs`
- **Validation:**
  `cargo test -p eddacraft-anvil util::tests::is_ignored_dir_name_matches_full_list`,
  `cargo test -p eddacraft-anvil commands::audit::tests::skips_generated_and_agent_worktree_dirs`,
  `cargo test -p eddacraft-anvil-kernel watcher::filter::tests::ignores_local_tool_and_worktree_dirs --lib`
- **Status:** Complete
- **Evidence:** Merged via PR #1497; shared CLI ignore policy and kernel watcher
  defaults now include local tool-state, agent-worktree, generated, cache, and
  build directories.

### WATCHUX-003: Initial Watch Scan Is Baseline State

- **Intent:** Ensure the initial watch graph build does not emit policy findings
  for existing repo contents as if they were new changes.
- **Expected Outcome:** Initial scan builds graph state and emits a readiness
  snapshot; later file changes still evaluate normally. Existing public exports
  do not appear as `new public symbol` findings on startup.
- **Files:** `crates/anvil-kernel/src/watch.rs`
- **Validation:**
  `cargo test -p eddacraft-anvil-kernel watch::tests::initial_scan_does_not_emit_existing_api_as_violations --lib`
- **Status:** Complete
- **Evidence:** Merged via PR #1497; initial watch graph construction emits a
  snapshot without surfacing existing public exports as new violations.

### WATCHUX-004: Immediate Watch Startup Feedback

- **Intent:** Avoid the blank-terminal experience while watcher setup is still
  synchronous.
- **Expected Outcome:** `anvil watch` prints a terse startup line before slow
  setup in non-JSON TUI mode, and falls back to plain mode when stdin or stdout
  are not terminals.
- **Files:** `crates/anvil-cli/src/commands/watch.rs`
- **Validation:**
  `cargo test -p eddacraft-anvil commands::watch::tests::output_mode`
- **Status:** Complete
- **Evidence:** Merged via PR #1497; watch mode selection falls back to plain mode
  when stdin/stdout are not both terminals and emits startup feedback before the
  watch loop.

### WATCHUX-005: Watch Status Language and Advisory Rendering

- **Intent:** Stop rendering advisory diagnostics as `Failing`.
- **Expected Outcome:** The watch TUI distinguishes health (`Starting`,
  `Warming up`, `Watching`, `Error`), findings (`Warnings`), and action outcomes
  (`Action failed`, `Blocked`). Public API expansion and new dependency findings
  default to warnings unless configured as enforced.
- **Files:** `crates/anvil-tui/src/surfaces/watch/mod.rs`,
  `crates/anvil-tui/src/surfaces/watch/render.rs`,
  `crates/anvil-tui/src/surfaces/watch/event_adapter.rs`,
  `crates/anvil-cli/src/commands/watch.rs`
- **Validation:** `cargo test -p eddacraft-anvil-tui watch --lib` and
  `cargo test -p eddacraft-anvil commands::watch`
- **Status:** Complete

### WATCHUX-006: Progressive Watch Warm-Up TUI

- **Intent:** Open the watch pane immediately and show warm-up progress instead
  of blocking before the TUI appears.
- **Expected Outcome:** Watch setup and initial scan emit progress events with
  phases such as `Discovering files`, `Registering watchers`, and
  `Building graph`; the TUI renders a spinner/progress bar and large-repo hint
  when warm-up is slow.
- **Files:** `crates/anvil-kernel/src/watch.rs`,
  `crates/anvil-kernel/src/watcher/mod.rs`,
  `crates/anvil-kernel-types/src/*`,
  `crates/anvil-cli/src/commands/watch.rs`,
  `crates/anvil-tui/src/surfaces/watch/*`
- **Validation:** `cargo test -p eddacraft-anvil-kernel watch --lib`,
  `cargo test -p eddacraft-anvil-tui watch --lib`, and a manual large-repo smoke
  run showing immediate TUI entry
- **Authorisation:** Operator requested continuing WATCHUX work on 2026-05-14;
  `WATCHUX-006` is the next sequenced slice after completed `WATCHUX-005`.
- **Status:** Complete

### WATCHUX-007: Configured Rule Modes and Start Summary

- **Intent:** Make advisory vs enforced behaviour explicit and visible after
  first activation.
- **Expected Outcome:** Config carries rule modes (`off`, `warn`, `enforce`) for
  public API expansion, new dependency, cross-layer import, and privilege
  expansion. `anvil start` / `anvil status` render a concise config summary and
  point to the config file for changes.
- **Files:** `crates/anvil-config/src/*`, `crates/anvil-cli/src/commands/start.rs`,
  `crates/anvil-cli/src/commands/status.rs`,
  `crates/anvil-cli/src/activation/render.rs`,
  `crates/anvil-kernel/src/policy/*`
- **Validation:** `cargo test -p eddacraft-anvil-config`,
  `cargo test -p eddacraft-anvil start`,
  `cargo test -p eddacraft-anvil status`, and fixture tests for default advisory
  rule modes
- **Authorisation:** Operator requested continuing WATCHUX work on 2026-05-14;
  implementation is bounded to the narrow typed rule-mode view and start/status
  summaries, not full config command editing.
- **Status:** Complete

### WATCHUX-008: Config Command Surface and Warm-Up Cache

- **Intent:** Provide explicit config operations and a safe watch warm-up cache
  without overloading `anvil start` flags.
- **Expected Outcome:** Add or extend config commands for `show`, `set`, and
  `convert --to <format>`; do not support `anvil start --toml` as a conversion
  path. `start` can write a non-authoritative `.anvil/cache/watch-warmup.json`
  with invalidation keys; `watch` consumes it opportunistically and reconciles
  against the filesystem.
- **Files:** `crates/anvil-cli/src/commands/config*.rs`,
  `crates/anvil-config/src/*`, `crates/anvil-cli/src/commands/start.rs`,
  `crates/anvil-cli/src/commands/watch.rs`, `crates/anvil-kernel/src/watch.rs`
- **Validation:** `cargo test -p eddacraft-anvil-config`,
  `cargo test -p eddacraft-anvil config`, and integration tests proving stale
  warm-up cache falls back safely
- **Authorisation:** Operator requested completing `WATCHUX-008` on 2026-05-14;
  implementation is limited to explicit config show/set/convert operations and
  an opportunistic, non-authoritative watch warm-up cache.
- **Status:** Complete
- **Evidence:** Implemented on `feat/watchux-008-config-cache`; `anvil config`
  now supports `show`, `set <rule> <mode>`, and `convert --to <format>`. `anvil
  start` writes `.anvil/cache/watch-warmup.json` opportunistically, and `anvil
  watch` validates the cache before using it as advisory warm-up evidence;
  stale or corrupt cache data falls back safely.

## Sequencing

1. **Beta hotfix:** WATCHUX-001 through WATCHUX-004.
2. **Language correction:** WATCHUX-005 so advisory diagnostics stop saying
   `Failing`.
3. **Progressive startup:** WATCHUX-006.
4. **Config truth:** WATCHUX-007.
5. **Cache and command surface:** WATCHUX-008 — complete.

## Release Notes

- WATCHUX-001 through WATCHUX-004 are beta-user bug fixes and should be eligible
  for the next beta patch if validation is green.
- WATCHUX-005 through WATCHUX-008 change product semantics and should be called
  out as first-run/watch UX improvements in the next minor beta.
