# Adoption Friction Removal

<!-- Executable only if tasks exist and status is Ready. -->

| ID    | Owner  | Status | Progress |
| ----- | ------ | ------ | -------- |
| ADOPT | @aneki | Ready  | 1/6 done |

**Last reviewed:** 2026-05-14 (promoted **Proposed → Ready** alongside
acceptance of
[`plans/specs/2026-05-14-release-plan-v0.7.0-sit-on.md`](../specs/2026-05-14-release-plan-v0.7.0-sit-on.md).
Progress: **1/6** — ADOPT-005 `anvil uninstall` shipped 2026-05-14 (see
PR #1521; `crates/anvil-cli/src/commands/uninstall.rs` with 10 tests).
Module-level `Ready` means "ready to begin Wave 3A"; individual tasks
remain `Draft` until picked up except ADOPT-005 which is **Done**.
ADOPT-001 hook coexistence is the next unblocking item; ADOPT-002 and
-003 are parallel-safe.)

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
  - `crates/anvil-hook/src/coexistence.rs` (NEW)
  - `crates/anvil-cli/src/commands/hook.rs`
  - `docs/runbooks/anvil-hook-coexistence.md` (NEW)
- **Validation:**
  - `cargo test -p eddacraft-anvil-hook coexistence::tests`
  - Integration: install Anvil into fixture repos preconfigured with each
    host manager; verify hooks fire in expected order; verify uninstall
    restores byte-identical state
- **Status:** Draft
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
  steady-state and peak-RSS numbers on a reference repository
  (committed as a benchmark fixture). CI fails the build if steady-state CPU
  > 5% or RSS > 200MB on the reference repo. `docs/policies/resource-
  budget.md` documents the ceiling and the measurement protocol.
- **Files:**
  - `crates/anvil-bench/benches/watch_resource_budget.rs` (NEW)
  - `.github/workflows/resource-budget.yml` (NEW)
  - `docs/policies/resource-budget.md` (NEW)
- **Validation:**
  - `cargo bench -p eddacraft-anvil-bench --bench watch_resource_budget`
  - CI: `resource-budget` workflow green on the candidate SHA
- **Status:** Draft
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
  - `crates/anvil-cli/src/activation/detect_agents.rs` (NEW)
  - `crates/anvil-cli/src/commands/start.rs`
  - `crates/anvil-run/src/detection.rs` (NEW; depends on INTL-001)
- **Validation:**
  - `cargo test -p eddacraft-anvil activation::detect_agents::tests`
  - Integration: fixture environments with each tool installed
- **Status:** Draft
- **Dependencies:** INTL-001 (for the `anvil-run` half)
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
  surface (watch, audit, hooks, `anvil-run`, baseline) honours the same
  list.
- **Expected Outcome:** Single source of truth in `anvil-cli/src/util.rs`
  (already established by WATCHUX-002); all surfaces consume it; the policy
  covers `.claude`, `.opencode`, `.gemini`, `.serena`, `.worktrees`,
  generated dirs, common cache dirs, plus `node_modules`, `target`, `dist`,
  `build`, `__pycache__`, `.venv`. Surface-by-surface conformance tests are
  added.
- **Files:**
  - `crates/anvil-cli/src/util.rs` (extended)
  - `crates/anvil-cli/src/commands/audit.rs`
  - `crates/anvil-cli/src/commands/hook.rs`
  - `crates/anvil-kernel/src/watcher/filter.rs`
  - `crates/anvil-baseline/src/*` (consume shared list)
  - `crates/anvil-run/src/*` (consume shared list)
- **Validation:**
  - `cargo test -p eddacraft-anvil util::tests::ignore_policy_covers_all_surfaces`
  - Per-surface integration tests that assert the policy is honoured
- **Status:** Draft
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
- **Status:** Done
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
- **Status:** Draft
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

- Coordinates with: [`WATCHUX-002`](watch-ux-advisory-rules.aps.md) (shared
  ignore helper), [`MLP-008`](multilayer-protection.aps.md) (hook bootstrap),
  [`INTL-001`](intercept-launcher.aps.md) (`anvil-run` scaffold needed for
  ADOPT-003's launcher half).
- Blocks on: WATCHUX-002 landing (already in flight).
