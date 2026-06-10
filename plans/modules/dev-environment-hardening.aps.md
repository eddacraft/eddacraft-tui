<!--
APS Module: Dev Environment Hardening
=====================================
Harden the local development environment for the concurrent-agent box:
build/cache, worktree lifecycle, and toolchain determinism. Implements ADR-057.
-->

# Dev Environment Hardening

| ID     | Owner | Status      | Progress |
| ------ | ----- | ----------- | -------- |
| DEVENV | —     | In Progress | 6/8      |

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

- **Status:** Merged 2026-05-29 via PR #2101
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
  first (`--apply` to delete), and orphaned in-tree reclaim stays a deliberate
  manual `cargo clean` per worktree (documented in the runbook — not a blind sweep).
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

- **Status:** Merged 2026-05-29 via PR #2104
- **Wave:** 1 (harden now)
- **Intent:** Remove the four-way Node drift (and its `better-sqlite3` ABI
  failures) and the stale-global-`oxfmt` false failures, with a single cache-bust.
- **Expected Outcome:** Standardise on **Node 24** (it's `.nvmrc`'s value, current
  LTS, already in the nightly matrix, and has `better-sqlite3` 12.10.0 prebuilds):
  `engines.node` → `>=24.0.0`, CI's `setup-workspace` default `22` → `24`, the
  `ci-nightly` matrix → `24.x`. The box keeps multiple Node majors via **`fnm`**
  (`--use-on-cd`): it auto-selects `.nvmrc`'s 24 inside anvil while a global 26
  stays for other work (documented in `worktree-policy.md`) — so no manual
  downgrade, and `mise`/devcontainer stays in the DEVENV-008 spike. The stale
  global `oxfmt` is defeated by prepending `node_modules/.bin` in `.envrc`
  (CIB-032). The nx cache key gains a `{ "runtime": "node --version" }` input in
  `sharedGlobals` so a Node change busts the JS-task cache (closes the cross-Node
  cache-poisoning gap surfaced by the 2026-05-29 clawpatch scan / ADV-6). Specialised
  workflows pinned to explicit Node 22 (`napi`, `security`, `infra`,
  `release-harness`) are left as-is — N-API is ABI-stable and they aren't the
  `better-sqlite3` path; they can migrate later.
- **Validation:** CI re-runs green on Node 24; a fresh `pnpm install` + edda-stack
  test on Node 24 builds `better-sqlite3` without ABI error; a Node change produces
  an nx cache miss.
- **Files:** `package.json` (`engines`), `.github/actions/setup-workspace/action.yml`,
  `.github/workflows/ci-nightly.yml`, `nx.json` (cache-key input), `.envrc`
  (pinned-bin PATH), `docs/guides/worktree-policy.md` (fnm + box guidance).
  `.nvmrc` already = 24.
- **Confidence:** medium — the CI-wide Node-24 bump re-proves all jobs on 24.

### DEVENV-006: Fix worktree creation + bootstrap in `.config/wt.toml`

- **Status:** Merged 2026-05-29 via PR #2113
- **Wave:** 1 (harden now)
- **Intent:** Make a fresh worktree start from `origin/main`, build cleanly, and
  fail loudly when it doesn't.
- **Expected Outcome:** New worktrees branch off `origin/main` (fetch-before-create,
  not stale local main, via the committed `scripts/dev/wt-new.sh` wrapper since
  `wt` has no pre-create hook); the post-start `install` fully reconciles pnpm
  workspace symlinks before `typecheck`; and the `rust` post-start stops swallowing
  failures (`2>/dev/null || true` removed) so a broken bootstrap is visible.
  Implementation note: the planned cause ("`dist/` missing, warm it") was wrong —
  on a `wt` worktree `dist/` is carried over by `copy-ignored`; the real fault is
  that `copy-ignored` seeds an inconsistent `node_modules` and a single
  `pnpm install` trusts the copied `.modules.yaml` and skips re-linking the missing
  per-consumer `workspace:*` symlinks (e.g. `anvil-api` → `@eddacraft/anvil-observability`),
  so the first `typecheck` fails `TS2307`. Fixed by removing `.modules.yaml` before
  install to force a one-pass relink from the warm global store.
- **Validation:** A freshly created worktree's first `pnpm typecheck` passes on
  untouched files; the branch's merge-base is `origin/main`'s tip; a deliberately
  broken build surfaces in post-start output instead of being swallowed.
- **Files:** `.config/wt.toml`, `scripts/dev/wt-new.sh` (new fetch-then-create
  wrapper), `docs/guides/worktree-policy.md` (branch-creation rules + bootstrap
  section).
- **Confidence:** resolved — `wt` accepts a remote-tracking ref as `--base`, so the
  thin `wt-new.sh` wrapper (fetch → `wt switch --create --base origin/main`) is the
  branch-base fix; no `wt` config knob exists for it.

### DEVENV-007: Wire change-scoped parity into the wt pre-commit + classifier

- **Status:** Merged 2026-06-10 via PR #2516
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

- **Status:** Ready
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
