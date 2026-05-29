<!--
APS Module: Dev Environment Hardening
=====================================
Harden the local development environment for the concurrent-agent box:
build/cache, worktree lifecycle, and toolchain determinism. Implements ADR-057.
-->

# Dev Environment Hardening

| ID     | Owner | Status      | Progress |
| ------ | ----- | ----------- | -------- |
| DEVENV | —     | In Progress | 1/8      |

## Purpose

Make the local dev environment deterministic and isolated per worker so the
recurring, agent-hour-draining failures catalogued by the 2026-05-29 planning
council (session `plan-6b3be127`, [ADR-057](../decisions/057-dev-environment-hardening.md))
stop recurring: disk ENOSPC from duplicated Rust `target/` dirs, fresh-worktree
bootstrap breakage, four-way Node version drift, and local↔CI parity gaps.

The approach is **hybrid**: land incremental hardening now (Wave 1, DEVENV-001..007),
then run a bounded spike (Wave 2, DEVENV-008) that exits with a go/no-go ADR on a
clean-slate reproducible base. The `origin/dev` base-ref fix that the council also
surfaced already landed independently via PR #2086 and is not re-counted here.

## In Scope

- Rust build-cache location, base size, and eviction on the shared box
- `git` worktree creation and bootstrap reliability via `.config/wt.toml`
- Toolchain version determinism (Node alignment now; version manager evaluated)
- Local↔CI validation parity through the shared change classifier
- nx-rust executor correctness under target relocation

## Out of Scope

- The reproducible-base substrate choice itself (mise/devcontainer/Nix) — that is
  DEVENV-008's spike output, captured in a future ADR
- Product behaviour, CI gate semantics, or release mechanics
- `sccache` / nx-cache-as-dedup adoption — deferred to the spike

## Work Items

### DEVENV-001: Trim dev/test debug info to shrink each target at the base

- **Status:** Merged 2026-05-29 via PR #2090
- **Wave:** 1 (harden now)
- **Intent:** Cut the ~100 GB-per-`target/` base bloat at its dominant source
  (full DWARF) without changing build location or behaviour.
- **Expected Outcome:** `Cargo.toml` gains `[profile.dev]` and `[profile.test]`
  with `debug = "line-tables-only"` and explicit `split-debuginfo = "unpacked"`;
  `incremental` left at default `true`. Backtraces still resolve to file:line;
  `panic = "unwind"`/`catch_unwind` (ADR-051) unaffected. Lands as a single
  combined commit merged in a low-activity window to avoid a cold-rebuild storm.
- **Validation:** `cargo build --workspace` succeeds; a debug `target/` is
  materially smaller than a full-DWARF baseline; `cargo test --workspace` green.
- **Files:** `Cargo.toml`.
- **Confidence:** high

### DEVENV-002: Layered Rust target relocation off the Projects mount

- **Status:** Proposed
- **Wave:** 1 (harden now)
- **Intent:** Stop ENOSPC by moving live `target/` dirs onto `/home`, bypass-proof
  yet per-worktree-isolated on the normal path.
- **Expected Outcome:** A committed `.cargo/config.toml` sets
  `[build] target-dir = "$HOME/.cache/anvil-targets/shared"` as the bypass-proof
  floor (honoured regardless of shell). A committed `.envrc` and the
  `.config/wt.toml` `rust` post-start export a per-worktree
  `CARGO_TARGET_DIR=$HOME/.cache/anvil-targets/<worktree-slug>` that overrides the
  floor when present, giving lock-free isolation. CI is unaffected (`$HOME`-relative).
- **Validation:** In a worktree with `direnv`/`wt` active, `cargo` writes to the
  per-slug dir; in a raw shell with neither, it writes to the shared floor — never
  into the in-tree `target/` on the Projects mount.
- **Dependencies:** DEVENV-003 (nx must be relocation-aware before relocation is
  advertised, or the nx/Azure cache breaks).
- **Files:** `.cargo/config.toml` (new), `.envrc` (new), `.config/wt.toml`.
- **Confidence:** high

### DEVENV-003: Make nx-rust executors relocation-aware + sentinel-emitting

- **Status:** Proposed
- **Wave:** 1 (harden now)
- **Intent:** Keep the nx build cache (local + Azure remote) correct under target
  relocation, and let eviction detect nx-driven builds.
- **Expected Outcome:** The `@eddacraft/nx-rust` build/test/check/clippy executors
  resolve `CARGO_TARGET_DIR` from the env and pass it as `--target-dir` (so nx's
  declared `outputs` match where cargo actually writes), and `touch` a
  `.anvil-building` sentinel in the target dir before spawning cargo and remove it
  in a `finally`-equivalent after. The `build` target's cache-hit restore lands at
  the relocated path, not `{workspaceRoot}/target`.
- **Validation:** `nx run <crate>:build` with `CARGO_TARGET_DIR` set produces a
  cache entry keyed to and restored at the relocated dir; a sentinel exists during
  the build and is gone after.
- **Coordinates with:** ADR-021 (in-house nx-rust plugin) — this changes its
  executors; ADR-049 (`^build` contract).
- **Files:** `tools/nx-rust/src/**` (executors + `utils/target-configs.ts`).
- **Confidence:** medium — executor change interacts with nx output tracking and
  the Powerpack Azure cache; needs careful cache-correctness tests.

### DEVENV-004: Disk-pressure target eviction (race-safe, dry-run-first)

- **Status:** Proposed
- **Wave:** 1 (harden now)
- **Intent:** Keep `/home` bounded by reclaiming idle relocated targets without
  ever deleting a target a build is using, given the PreToolUse safety hooks are
  no-ops.
- **Expected Outcome:** `scripts/cache/anvil-target-evict.sh` + a `systemd --user`
  timer evict LRU-by-mtime above a `/home` high-water mark. A dir is skipped if a
  non-blocking `flock -n` on its `.cargo-lock` fails, its newest mtime is within a
  freshness window, or a fresh `.anvil-building` sentinel is present. The script
  asserts a hard `$ANVIL_TARGET_BASE` prefix and **fails closed**, ships
  **dry-run/log-only** first, and orphaned in-tree reclaim is an opt-in
  `wt clean-stale-targets` (not a blind sweep).
- **Validation:** Dry-run logs over one cycle never select a building dir; with a
  sentinel/lock present the dir is skipped; a path outside `$ANVIL_TARGET_BASE` is
  refused with a non-zero exit.
- **Dependencies:** DEVENV-003 (the `.anvil-building` sentinel) and DEVENV-002
  (the relocated base path).
- **Files:** `scripts/cache/anvil-target-evict.sh` (new), `systemd --user` unit
  files (operator-installed, documented in the runbook), `.config/wt.toml`.
- **Confidence:** medium

### DEVENV-005: Align Node version + fix the oxfmt shadow + nx cache key

- **Status:** Proposed
- **Wave:** 1 (harden now)
- **Intent:** Remove the four-way Node drift (and its `better-sqlite3` ABI
  failures) and the stale-global-`oxfmt` false failures, with a single cache-bust.
- **Expected Outcome:** `.nvmrc`, `engines.node`, the CI Node matrix, and the box
  agree on one `better-sqlite3`-compatible Node version; the repo-pinned `oxfmt`
  0.51 wins over any stale global. The nx cache key gains a `runtime` input on the
  Node version (a Node change busts the key), batched into this same commit so the
  toolchain-alignment cache-bust happens once. Adopting `mise`/`.tool-versions` is
  explicitly **not** in this item (DEVENV-008).
- **Validation:** A fresh `pnpm install` + `pnpm test` on a clean worktree builds
  `better-sqlite3`/edda-stack without ABI error; `pnpm format:check` uses 0.51;
  changing the Node version produces an nx cache miss on a native-dep target.
- **Files:** `.nvmrc`, `package.json` (`engines`), `.github/workflows/*` (Node
  matrix + `setup-workspace`), `nx.json` (cache key input), a documented PATH/shim
  fix for `oxfmt`.
- **Confidence:** medium — picking the single Node version requires confirming
  `better-sqlite3` prebuild availability.

### DEVENV-006: Fix worktree creation + bootstrap in `.config/wt.toml`

- **Status:** Proposed
- **Wave:** 1 (harden now)
- **Intent:** Make a fresh worktree start from `origin/main`, build cleanly, and
  fail loudly when it doesn't.
- **Expected Outcome:** New worktrees branch off `origin/main` (fetch-before-create,
  not stale local main); post-start warms the dist-producing workspace packages so
  `dist/` and pnpm workspace symlinks exist before `typecheck`; and the `rust`
  post-start stops swallowing failures (`2>/dev/null || true` removed/narrowed) so
  a broken bootstrap is visible.
- **Validation:** A freshly created worktree's first `pnpm typecheck` passes on
  untouched files; the branch's merge-base is `origin/main`'s tip; a deliberately
  broken build surfaces in post-start output instead of being swallowed.
- **Files:** `.config/wt.toml`; worktree-creation guidance in the dev-workflow
  docs.
- **Confidence:** medium — `wt`'s control over the branch base point needs
  confirming (config vs a thin create wrapper).

### DEVENV-007: Wire change-scoped parity into the wt pre-commit + classifier

- **Status:** Proposed
- **Wave:** 1 (harden now)
- **Intent:** Make the local pre-commit gate match CI's classifier and close the
  path-gate that hid the observability E2E break.
- **Expected Outcome:** `.config/wt.toml`'s pre-commit hook runs the change-scoped
  `scripts/validate/local.sh` (sharing `scripts/ci/classify-changes.sh`) instead of
  blunt full `typecheck`/`lint`/`format`; and `classify-changes.sh` is extended so
  E2E-impacting paths require the E2E surface, closing the gap in CI and local at
  once.
- **Validation:** Editing an E2E-impacting path makes `validate/local.sh` select
  the E2E surface; the wt pre-commit runs the scoped validation; CI and local agree
  on the required surfaces for a given diff.
- **Dependencies:** Builds on the `origin/dev`→`origin/main` base-ref fix (PR #2086).
- **Files:** `.config/wt.toml`, `scripts/ci/classify-changes.sh`,
  `scripts/validate/local.sh`.
- **Confidence:** medium

### DEVENV-008: Spike — reproducible dev-environment base (go/no-go ADR)

- **Status:** Proposed
- **Wave:** 2 (spike / evaluate)
- **Intent:** Decide whether to re-platform the dev environment onto a reproducible
  base, with a hard exit so it cannot become open-ended research.
- **Expected Outcome:** A spike that evaluates `mise`/`.tool-versions` vs
  devcontainer vs Nix as the determinism substrate, and **nx-cache-as-local-dedup**
  (route local builds through `nx build` + a shared or remote-readable nx cache —
  a candidate that may beat `sccache`) vs `sccache` vs the wave-1 relocation, and
  **exits by merging a go/no-go ADR** ("adopt X with this migration plan" or
  "defer, because"). Also parks `incremental=false` / `split-debuginfo="packed"`
  footprint experiments, `CARGO_HOME` consolidation, and a `mold`/`lld` linker swap
  as spike inputs.
- **Validation:** The go/no-go ADR is merged and linked from this item; if "go",
  it carries a migration plan; if "defer", it records why.
- **Dependencies:** Informed by DEVENV-001..007 outcomes (treated as spike inputs,
  not pre-empted by them).
- **Files:** `plans/decisions/` (the go/no-go ADR), a spike report under
  `plans/specs/` or `plans/audits/`.
- **Confidence:** medium — the substrate trade-offs are real and the win depends on
  measured rebuild/disk numbers from the wave-1 changes.
