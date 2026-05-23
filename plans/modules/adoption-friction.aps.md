# Adoption Friction Removal

<!-- Executable only if tasks exist and status is Ready or In Progress. -->

| ID    | Owner  | Status | Progress |
| ----- | ------ | ------ | -------- |
| ADOPT | @aneki | Merged | 6/6 |

**Last reviewed:** 2026-05-18 (counter and narrative refreshed for
ADOPT-003 CLI-wiring + `anvil-run` half merge via PR #1700; module
status In Progress → Merged. Progress: **6/6** — ADOPT-005
`anvil uninstall` merged 2026-05-14 (PR #1521) and **Released/Shipped
via [`v0.6.3-beta`](../releases/v0.6.3-beta.md) on 2026-05-15**;
ADOPT-001 hook coexistence Done 2026-05-15 (runbook at
`docs/runbooks/anvil-hook-coexistence.md`); ADOPT-002 resource-budget
enforcement completed 2026-05-16; ADOPT-004 shared ignore policy
merged 2026-05-16 via PR #1658; ADOPT-006 editor coexistence matrix +
harness + CI gate merged 2026-05-17 via PR #1682; **ADOPT-003 AI tool
auto-detect — primitive merged 2026-05-14 via PR #1543, CLI wiring +
`anvil-run` half merged 2026-05-18 via PR #1700**. Cleanup agent
advances Merged → Released/Shipped → Complete when release evidence
from the v0.7.0-beta runbook lands.)

## Purpose

A senior engineer who tries Anvil for the first time has already configured
their development environment to their taste. They have a pre-commit hook
manager. They have a battery they pay attention to. They have AI tooling
configured. They notice surprises and they uninstall on surprise.

WATCHUX exists because beta surfaced exactly this class of issue: the curl
installer ignored Homebrew, audit scanned agent worktrees, watch looked hung,
advisory rendered as failure. This module owns the **next layer** of the same
problem: friction that does not appear in the first 60 seconds but does appear
in the first week.

The success test: three internal users run Anvil on their normal work for a
calendar week (Boring Week, per the release plan proposal) and none of them
disables any check, suppresses without resolution, or bypasses a hook.

## In Scope

- Pre-commit hook coexistence with lefthook, husky, and pre-commit-framework
  (the three dominant managers in 2026)
- Resource budget measurement and ceiling — CPU steady-state and RSS bound
- AI tool auto-detect for Claude Code, Cursor, Aider, Windsurf, Codex
- Generated-file / agent-worktree / cache-dir ignore policy extended across
  every Anvil entry surface (watch, audit, hooks, `anvil-run`)
- Clean uninstall path that leaves repo + git config exactly as found
- Editor surface coexistence — no LSP, formatter, or language-server conflicts
  on representative real configurations

## Out of Scope

- Org-level deployment policy (Horizon 2)
- Windows-specific shell wrapper polish (covered by INTL-006 follow-up)
- Cross-organisation user analytics (INSIGHTS module owns local-only signal;
  cross-org is post-v0.7.0)
- New language support beyond what the parser already covers
- Hosted control plane / cloud sync

## Interfaces

- **Depends on:**
  - `crates/anvil-cli/src/util.rs` (shared ignore policy from WATCHUX-002)
  - `crates/anvil-cli/src/commands/start.rs`, `audit.rs`, `watch.rs`
  - `crates/anvil-hook/*` (from MLP-003)
  - `crates/anvil-bench/*` (resource budget measurement)
  - `crates/anvil-run/*` (from INTL-001 once landed)
  - `WATCHUX-002` shared local-noise ignore policy
  - `MLP-008` hook bootstrap recovery (extended for coexistence)
- **Exposes:**
  - Hook-coexistence install paths that respect lefthook/husky/pre-commit-
    framework managed surfaces
  - Documented resource ceiling with CI-enforced upper bound
  - `anvil uninstall` command
  - AI tool detection helper consumed by `anvil start` and `anvil-run`

## Tasks

### ADOPT-001: Pre-Commit Hook Coexistence

- **Intent:** Install and run Anvil pre-commit / pre-push / post-commit hooks
  alongside lefthook, husky, and pre-commit-framework without conflict.
- **Expected Outcome:** When a host hook manager exists, Anvil registers as
  a managed hook in that manager's config (lefthook `pre-commit.commands`,
  husky `.husky/pre-commit`, pre-commit-framework `.pre-commit-config.yaml`)
  instead of overwriting `.git/hooks/`. On uninstall, Anvil removes only its
  own entries. Detection precedence is documented and tested.
- **Files:**
  - `crates/anvil-hook/src/coexistence.rs` (NEW — primitive landed
    2026-05-14)
  - `crates/anvil-cli/src/commands/hook.rs` (CLI wiring — follow-up)
  - `docs/runbooks/anvil-hook-coexistence.md` (NEW — follow-up)
- **Validation:**
  - `cargo test -p eddacraft-anvil-hook coexistence` (25 tests green
    on `feat/adopt-001-hook-coexistence`)
  - Integration: install Anvil into fixture repos preconfigured with
    each host manager; verify hooks fire in expected order; verify
    uninstall restores byte-identical state (deferred to CLI-wiring
    follow-up)
- **Status:** Released/Shipped via v0.7.0-beta (d7873161 · 2026-05-21)
- **Shipped:** 2026-05-15 (primitive + CLI wiring merged to main;
  runbook filed at `docs/runbooks/anvil-hook-coexistence.md`)
- **changeType:** feature
- **releaseIntent:** candidate
- **releaseScope:** minor
- **releaseNote:**
  - audience: user
  - type: added
  - text: "Anvil hooks now install alongside lefthook, husky, and
    pre-commit-framework without conflict."

### ADOPT-002: Resource Budget Measurement And Ceiling

- **Intent:** Pin a documented, CI-enforced resource ceiling so senior users
  do not notice Anvil on their battery or CPU graph.
- **Expected Outcome:** `crates/anvil-bench` produces measured CPU
  steady-state and peak-RSS numbers on a deterministic generated reference
  repository. CI fails the build if steady-state CPU > 5% or RSS > 200MB on the
  reference repo. `docs/policies/resource-budget.md` documents the ceiling and
  the measurement protocol.
- **Files:**
  - `crates/anvil-bench/src/budget.rs` (NEW — evaluator primitive
    landed 2026-05-14 on `feat/adopt-002-resource-budget`)
  - `docs/policies/resource-budget.md` (NEW — landed 2026-05-14)
  - `crates/anvil-bench/benches/watch_resource_budget.rs` (NEW —
    drives `anvil watch` on the fixture and emits a `BudgetVerdict`)
  - `.github/workflows/resource-budget.yml` (NEW — runs the bench
    scenario and asserts on the JSON verdict)
- **Validation:**
  - `cargo test -p anvil-bench budget` (11 tests green; covers
    pinned ceiling, pass/fail axes, JSON shape and round-trip)
  - `cargo bench -p anvil-bench --bench watch_resource_budget`
  - CI: `resource-budget` workflow green on the candidate SHA
- **Status:** Released/Shipped via v0.7.0-beta (d7873161 · 2026-05-21)
- **Completed:** 2026-05-16 on `feat/adopt-002-resource-budget`; added
  Linux `/proc` sampler, `watch_resource_budget` bench, and CI workflow.
- **changeType:** internal
- **releaseIntent:** candidate
- **releaseScope:** minor
- **releaseNote:**
  - audience: user
  - type: added
  - text: "Anvil now publishes a measured resource budget and CI fails on
    regression."

### ADOPT-003: AI Tool Auto-Detect

- **Intent:** Detect Claude Code, Cursor, Aider, Windsurf, and Codex
  installations without configuration so the user does not have to wire
  Anvil into each one.
- **Expected Outcome:** `anvil start` and `anvil-run` enumerate installed
  AI tools via documented detection heuristics (binary on PATH, well-known
  config paths, env-var hints) and print a short summary. Detection is
  cached in `.anvil/cache/detected-agents.json` (non-authoritative) and
  reconciled on next start. Detection covers macOS, Linux, and Windows.
- **Files:**
  - `crates/anvil-cli/src/activation/detect_agents.rs`
    (primitive landed 2026-05-14, PR #1543; CLI wiring
    follow-up 2026-05-18 adds `RealDetectionEnv`,
    `detect_and_cache`, `cache_path`,
    `render_inventory_summary`, executable-bit check)
  - `crates/anvil-cli/src/commands/start.rs` (CLI wiring —
    runs detection on every `anvil start`, writes cache under
    non-read-only modes, prints `AI tools detected: …`
    summary with `(not cached)` qualifier under `--verify`)
  - `crates/anvil-run/src/detection.rs` (NEW — narrow
    consumer that reads the cache and returns kebab-case
    agent ids; 9 tests pin the wire contract from the
    consumer side)
- **Validation:**
  - `cargo test -p eddacraft-anvil --bin anvil` — 1615 tests
    green (27 in `activation::detect_agents`, covering
    read-compare-skip, write-error inventory preservation,
    Unix executable-bit check)
  - `cargo test -p eddacraft-anvil-run --lib detection` — 9
    tests green on the cache-consumer surface
  - `cargo clippy --workspace --all-targets -- -D warnings` clean
  - Integration: fixture environments with each tool installed
    (deferred — manual Boring Week validation)
- **Status:** Released/Shipped via v0.7.0-beta (d7873161 · 2026-05-21)
- **Picked up:** 2026-05-14 (primitive merged via PR #1543;
  CLI wiring + `anvil-run` half on
  `feat/adopt-003-cli-wiring` 2026-05-18)
- **Evidence:** Primitive merged via PR #1543 on 2026-05-14
  (commit `7a730c61`). CLI wiring + `anvil-run` half merged
  via PR #1700 on 2026-05-18 (rebase commits `985002bb` +
  `a0967e96`). Validation: 1615 CLI tests + 9 anvil-run
  detection tests green; `cargo clippy --workspace
  --all-targets -- -D warnings` clean; full pnpm gate chain
  green. Cleanup agent advances Merged → Released/Shipped →
  Complete when v0.7.0-beta release evidence lands.
- **Dependencies:** INTL-001 (for the `anvil-run` half) —
  landed via PR #1528 on 2026-05-14, no longer blocking.
- **changeType:** feature
- **releaseIntent:** candidate
- **releaseScope:** minor
- **releaseNote:**
  - audience: user
  - type: added
  - text: "Anvil now auto-detects Claude Code, Cursor, Aider, Windsurf, and
    Codex without configuration."

### ADOPT-004: Complete Local-Noise Ignore Policy Across All Surfaces

- **Intent:** Extend the WATCHUX-002 shared ignore policy so every Anvil
  walking surface (watch, audit, baseline, check, drift, gate) honours the
  same list, with the cli-side helper re-exporting the kernel-owned
  canonical const so they cannot drift.
- **Expected Outcome:** Single canonical const lives in
  `anvil-kernel::watcher::filter::IGNORE_DIRS` — kernel is the lowest crate
  every walking consumer can reach (cli depends on kernel; the reverse
  cycle is what motivated moving the const down from
  `anvil-cli/src/util.rs`). `anvil-cli/src/util.rs::IGNORE_DIRS` becomes a
  `pub use` re-export of the kernel const so existing cli command
  call-sites keep working without churn. Coverage adds `.venv` and
  reconciles `__pycache__` into the kernel list. `anvil-baseline`,
  `anvil-run`, and `anvil-hook` continue to operate on caller-supplied
  file lists and do not walk directories themselves, so they inherit the
  policy transitively via the cli wrappers (audit/baseline/check/drift/
  gate/hook command modules). A conformance test asserts the kernel and
  cli helpers resolve to the same set.
- **Files:**
  - `crates/anvil-kernel/src/watcher/filter.rs` (canonical const moves
    here; `default_patterns()` derives from it)
  - `crates/anvil-cli/src/util.rs` (re-exports the kernel const)
  - Surface call-sites already consume `is_ignored_dir_name` (audit,
    baseline, check, drift, gate command modules) — no churn beyond the
    re-export shape
- **Validation:**
  - `cargo test -p eddacraft-anvil-kernel watcher::filter::tests::ignore_policy_covers_all_surfaces`
  - `cargo test -p eddacraft-anvil util::tests::cli_helper_matches_kernel_canonical`
  - Existing per-surface tests in `audit.rs`, `check.rs`, `baseline.rs`,
    `watcher/filter.rs` continue to pass against the unified set
- **Status:** Released/Shipped via v0.7.0-beta (d7873161 · 2026-05-21)
- **Evidence:** Merged via PR #1658 (`feat(kernel): unify local-noise
  ignore policy across surfaces`) on 2026-05-16, rebase-merged as
  `34671da7`. Canonical const now lives at
  `crates/anvil-kernel/src/watcher/filter.rs::IGNORE_DIRS`; cli
  `is_ignored_dir_name` is a re-export. `.venv` added; `__pycache__`
  reconciled. Conformance tests (`ignore_policy_covers_all_surfaces`,
  `default_patterns_derives_from_canonical_const`,
  `cli_helper_matches_kernel_canonical`) prevent drift.
- **Dependencies:** WATCHUX-002 (shared helper)
- **changeType:** feature
- **releaseIntent:** candidate
- **releaseScope:** minor
- **releaseNote:**
  - audience: user
  - type: fixed
  - text: "Generated, cache, and agent-worktree directories are now ignored
    across watch, audit, hooks, and `anvil-run`."

### ADOPT-005: Clean Uninstall

- **Intent:** `anvil uninstall` returns the repo to byte-identical pre-
  install state for tracked files, removes hooks safely, and stops/removes
  the daemon.
- **Expected Outcome:** `anvil uninstall` is a documented top-level
  command. On execution: hooks are removed (respecting coexistence per
  ADOPT-001), `.anvil/` is removed, the daemon is stopped and its install
  artefacts are removed, Homebrew/symlink installs are surfaced (with a
  pointer to `brew uninstall`) but not forcibly removed. The command emits
  a diff summary of changed files before any destructive action and
  requires `--yes` for non-interactive mode.
- **Files:**
  - `crates/anvil-cli/src/commands/uninstall.rs`
  - `crates/anvil-cli/src/commands/hooks.rs` (added
    `uninstall_all_managed_hooks` helper)
  - `crates/anvil-cli/src/commands/mod.rs`, `main.rs` (wiring)
  - `docs/runbooks/anvil-uninstall.md` (deferred — runbook still to land)
- **Validation:**
  - `cargo test -p eddacraft-anvil --bin anvil uninstall` — 10 new
    tests green; 10 existing hooks tests still pass
  - Integration: install + uninstall in a fixture repo, verify
    `git status` is clean and `.anvil/` is gone (verified manually on
    fixture; runbook task is to formalise this as a script)
- **Status:** Released/Shipped via v0.6.3-beta (PR #1521 · 2026-05-14)
- **Evidence:** Merged via PR #1521 (`feat(cli): add 'anvil uninstall' command +
  ADR-044 MCP entry ownership`) on 2026-05-14. Released in
  [`v0.6.3-beta`](../releases/v0.6.3-beta.md) (2026-05-15).
- **Coordinates with:** ADOPT-001 — uninstall ships ahead of the hook
  coexistence work. The shipped implementation calls the silent
  `hooks::uninstall_all_managed_hooks_silent` helper (which clears
  both file-mode and Git 2.54 config-mode managed hooks). When
  ADOPT-001 lands, that helper will need to grow awareness of
  husky / lefthook / pre-commit-framework managed surfaces — captured
  under ADOPT-001, not as a blocking dependency for -005.
- **changeType:** feature
- **releaseIntent:** candidate
- **releaseScope:** minor
- **releaseNote:**
  - audience: user
  - type: added
  - text: "`anvil uninstall` cleanly removes Anvil and leaves your repo
    exactly as it was. `--global` extends to user-level state and MCP
    entries (surgical JSON edit, other servers preserved)."
- **Shipped:** PR #1521 (2026-05-14)

### ADOPT-006: Editor Surface Coexistence

- **Intent:** Verify Anvil does not conflict with the LSP servers, language
  servers, and formatters senior users already have installed in their
  editor.
- **Expected Outcome:** Compatibility matrix documented in
  `docs/policies/editor-coexistence.md` covering VS Code, Cursor, JetBrains,
  Neovim with the common Rust/TypeScript/Python toolchains
  (`rust-analyzer`, `tsserver`, `pyright`, `ruff`, `prettier`, `eslint`).
  Coexistence test cases run as part of CI on a fixture set; failures block
  the candidate.
- **Files:**
  - `tools/test-harness/editor-coexistence/` (NEW)
  - `docs/policies/editor-coexistence.md` (NEW)
  - `.github/workflows/editor-coexistence.yml` (NEW)
- **Validation:**
  - CI: `editor-coexistence` workflow green on candidate SHA
  - Manual: each combination spot-checked in Boring Week
- **Status:** Released/Shipped via v0.7.0-beta (d7873161 · 2026-05-21)
- **Picked up:** 2026-05-17 (urgent-authorised by @aneki via `/goal`;
  promoted from Draft → In Progress per `plans/aps-rules.md` status rule 1.)
- **Evidence:** Merged via PR #1682 (`feat(adopt-006): editor coexistence
  matrix + harness + CI gate`) on 2026-05-17, rebase-merged as `01987faf`.
  Shipped: matrix policy at `docs/policies/editor-coexistence.md`,
  headless harness at `tools/test-harness/editor-coexistence/` with six
  per-target runners (rust-analyzer, tsserver, pyright, ruff, prettier,
  eslint) over rust / typescript / python fixtures, and the
  `Editor Coexistence` CI gate at
  `.github/workflows/editor-coexistence.yml`. Desktop editor cells
  (VS Code, Cursor, JetBrains IDEA, Neovim) remain BORING WEEK manual per
  the task validation note. Open follow-ups (filed under ADOPT-006, not
  blocking this slice): wire `anvil watch --json` so the `anvil_events`
  cell becomes a hard gate; move inotify capacity from warn → refuse so
  the policy claim becomes load-bearing; LSP initialize-response
  validation in `targets/rust-analyzer.sh` when the wire-protocol path
  is restored.
- **changeType:** internal
- **releaseIntent:** candidate
- **releaseScope:** minor
- **releaseNote:**
  - audience: user
  - type: added
  - text: "Anvil ships a documented editor compatibility matrix and CI gate."

## Sequencing

1. **ADOPT-005** (shipped 2026-05-14, PR #1521) is out ahead of
   ADOPT-001. The two coordinate (uninstall must remove hooks installed
   under whichever manager ADOPT-001 supports) but neither blocks the
   other — see the cross-reference in ADOPT-005's task body.
2. **ADOPT-001**, **ADOPT-002**, **ADOPT-003**, **ADOPT-006** are
   parallel-safe after that.
3. **ADOPT-004** waits for WATCHUX-002 (shared ignore helper) to land —
   this is already in flight.
4. **ADOPT-003** depends on INTL-001 only for the `anvil-run` half; the
   `anvil start` half is independent.

## Release Notes

ADOPT items collectively justify a "Anvil is now polite to the rest of your
toolchain" line in `v0.7.0-beta`. Per-task `releaseNote` text above covers
the user-visible specifics.

## Cross-References

- Coordinates with: [`WATCHUX-002`](../archive/modules/watch-ux-advisory-rules.aps.md) (shared
  ignore helper), [`MLP-008`](../archive/modules/multilayer-protection.aps.md) (hook bootstrap),
  [`INTL-001`](intercept-launcher.aps.md) (`anvil-run` scaffold needed for
  ADOPT-003's launcher half).
- Blocks on: WATCHUX-002 landing (already in flight).
