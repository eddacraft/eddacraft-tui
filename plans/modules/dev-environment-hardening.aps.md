<!--
APS Module: Dev Environment Hardening
=====================================
Harden the local development environment for the concurrent-agent box:
build/cache, worktree lifecycle, and toolchain determinism. Implements ADR-057.
-->

# Dev Environment Hardening

| ID     | Owner | Status      | Progress |
| ------ | ----- | ----------- | -------- |
| DEVENV | —     | In Progress | 2/8      |

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

- **Status:** Merged 2026-05-29 via PR #2094
- **Wave:** 1 (harden now)
- **Intent:** Stop ENOSPC by moving live `target/` dirs onto `/home`,
  per-worktree-isolated (lock-free).
- **Expected Outcome:** A committed `.envrc` (direnv) and the `.config/wt.toml`
  `rust` post-start export a per-worktree
  `CARGO_TARGET_DIR=$HOME/.cache/anvil-targets/<worktree-slug>` on `/home`. A
  loud, non-blocking guard (direnv's own "blocked, run `direnv allow`" nag, plus
  a `wt` pre-commit warning when `CARGO_TARGET_DIR` is unset) catches the case
  where a build would hit the full mount. **Design change from the original ADR
  plan:** the committed `.cargo/config.toml` "bypass-proof floor" was dropped —
  cargo's config `target-dir` does NOT expand `$HOME` (verified: it creates a
  literal `$HOME/` dir), a hardcoded `/home/...` path isn't committable, and a
  parent-dir operator config would relocate sibling projects too. So relocation
  is env-driven; the operator accepted the residual porousness (an agent using
  neither direnv nor `wt`) in exchange for a fully-committed, CI-safe change.
- **Validation:** `.envrc` resolves `CARGO_TARGET_DIR` to a per-worktree dir on
  `/home` and creates it; the guard warns when unset and is silent when set;
  everything is inert on CI runners (no direnv / no `wt`) so the nx/Azure build
  cache is unaffected.
- **Dependencies:** None blocking. DEVENV-003 (nx relocation-awareness) is **not**
  required for CI correctness — relocation never happens on CI runners — and is
  deferred; it remains needed for local nx `build`-target caching under
  relocation and for the DEVENV-004 eviction sentinel.
- **Files:** `.envrc` (new), `.config/wt.toml`, `docs/guides/worktree-policy.md`.
- **Confidence:** high

### DEVENV-003: nx-rust relocation-aware build outputs (upstream)

- **Status:** Blocked
- **Wave:** 1 (harden now)
- **Intent:** Make the active nx-rust plugin cache the *relocated* Rust build
  outputs so local `nx build` caching is correct under DEVENV-002 relocation.
- **Expected Outcome:** The active plugin is `@eddacraft/nxrust`, resolved from the
  registry — the in-house plugin (ADR-021) was extracted to the public
  `eddacraft/nxrust` repo, and anvil's old `tools/nx-rust` vendored copy was dead
  code (referenced by nothing) and is **removed in this work**. The
  reloc-awareness — inject `CARGO_TARGET_DIR` as the build target's `target-dir`
  option so the cached `outputs` follow where cargo writes — must land in
  `eddacraft/nxrust`'s in-flight caching work (open PRs #15 cache inputs / #16
  narrow build outputs, CACHE-001/002), after which anvil bumps the
  `@eddacraft/nxrust` dep and verifies. **anvil-001 cannot fix this locally.**
- **Validation:** after the nxrust bump, `nx build <crate>` with `CARGO_TARGET_DIR`
  set caches/restores at the relocated dir (not `{workspaceRoot}/target`).
- **Blocked on:** `eddacraft/nxrust` shipping `CARGO_TARGET_DIR`-aware build
  outputs (coordinate with its CACHE work) + a published release.
- **Note:** the cache-*correctness* gap is already mitigated upstream —
  `@eddacraft/nxrust` lists `CARGO_TARGET_DIR` in its cache-key env allowlist, so a
  relocated build cannot take a stale non-relocated cache hit. The residual gap is
  cache *reuse* only, and benign (agents build via raw cargo; `check`/`test`/
  `clippy` have empty outputs). Low priority until the nxrust CACHE work ships.
- **Coordinates with:** ADR-021, ADR-049; `eddacraft/nxrust` CACHE-001/002.
- **Confidence:** medium — dependent on external release cadence.

### DEVENV-004: Disk-pressure target eviction (race-safe, dry-run-first)

- **Status:** Proposed
- **Wave:** 1 (harden now)
- **Intent:** Keep `/home` bounded by reclaiming idle relocated targets without
  ever deleting a target a build is using, given the PreToolUse safety hooks are
  no-ops.
- **Expected Outcome:** `scripts/cache/anvil-target-evict.sh` + a `systemd --user`
  timer evict LRU-by-mtime above a `/home` high-water mark. A dir is skipped if a
  non-blocking `flock -n` on its `.cargo-lock` fails (cargo holds that lock for the
  duration of any build/check/test/clippy, so no plugin-emitted sentinel is
  needed) or its newest mtime is within a freshness window. The script asserts a
  hard `$ANVIL_TARGET_BASE` prefix and **fails closed**, ships **dry-run/log-only**
  first, and orphaned in-tree reclaim is an opt-in `wt clean-stale-targets` (not a
  blind sweep).
- **Validation:** Dry-run logs over one cycle never select a building dir; with the
  `.cargo-lock` held the dir is skipped; a path outside `$ANVIL_TARGET_BASE` is
  refused with a non-zero exit.
- **Dependencies:** DEVENV-002 (the relocated base path). Not blocked on
  DEVENV-003 — the original `.anvil-building` sentinel is replaced by cargo's own
  `.cargo-lock` flock, which requires no plugin change.
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
