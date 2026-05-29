# ADR-057: Local dev-environment hardening (build cache, worktree lifecycle, toolchain determinism)

## Status

Accepted

## Date

2026-05-29

## Context

Anvil is developed on a single shared Linux box where multiple AI agents
(Claude Code + opencode siblings) work concurrently in `git` worktrees driven by
`wt` (Worktrunk). A 2026-05-29 planning council (session `plan-6b3be127`)
catalogued recurring, agent-hour-draining environment failures, several of which
stopped work outright:

- **Build/disk.** Each of ~17 worktrees carries its own Rust `target/` (~100 GB
  each; the main checkout's alone is 97 GB), ~1.7 TB total. These sit on the
  Projects mount (`nvme1n1p1`), which hit **100% → ENOSPC mid-work**. `/home`
  (`nvme0n1p3`) has ~565 GB free. ~100 GB per single target is itself abnormal:
  `Cargo.toml` defines no `[profile.dev]`/`[profile.test]`, so both inherit
  `debug = 2` (full DWARF) — the dominant contributor.
- **Worktree lifecycle.** `wt --create` branches off **stale local `main`**
  (never pulled) so fresh PRs start behind `origin/main` and conflict; fresh
  worktrees miss per-package pnpm workspace symlinks + package `dist/`, so
  `pnpm typecheck` fails on untouched files. `.config/wt.toml`'s `rust`
  post-start runs `cargo build --workspace 2>/dev/null || true` — raw cargo (no
  target relocation) that **silently swallows build failures**.
- **Toolchain determinism.** Rust (`rust-toolchain.toml` 1.95.0) and pnpm
  (corepack `packageManager`) are pinned; **Node is not**: `.nvmrc`=24,
  `engines.node`≥22.13, CI runs 22, the box runs v26 — a four-way drift that
  produces `better-sqlite3` ABI failures (built ABI 137 vs box ABI 147) which
  nx-bail the graph. A stale global `oxfmt` 0.41 shadows the repo-pinned 0.51,
  producing false format failures.
- **Local↔CI parity.** A shared classifier (`scripts/ci/classify-changes.sh`)
  backs both CI and `scripts/validate/local.sh`, but both `validate/local.sh
  --changed` and `scripts/agent/guidance.sh --branch` defaulted their base ref
  to the **retired `origin/dev`** (fixed in PR #2086), and the `wt` pre-commit
  hook runs blunt full `typecheck`/`lint`/`format` rather than the change-scoped
  classifier. E2E is path-gated, so latent breaks (e.g. the
  `@eddacraft/anvil-observability` build-graph gap, PR #2074) only surface
  post-merge.

A decision is needed now because the disk failure recurs and blocks all agents,
and because every fix touches the **shared** environment under concurrently
running agents — the change strategy is as load-bearing as the changes.

The operator chose a **hybrid** approach: land incremental hardening now (wave 1),
and run a bounded spike to evaluate a clean-slate reproducible base before
committing to one (wave 2). This ADR records the wave-1 architecture; the
reproducible-base decision is explicitly deferred to a future ADR (the spike's
go/no-go).

## Decision

Adopt the following wave-1 architecture, tracked by the `DEVENV` APS module
(`plans/modules/dev-environment-hardening.aps.md`). Implementation lands as
separate PRs; this ADR fixes the design.

1. **Base-bloat lever (immediate, repo-root, reversible).** Commit
   `[profile.dev]`/`[profile.test]` with `debug = "line-tables-only"` and an
   explicit `split-debuginfo = "unpacked"` in `Cargo.toml`. Leave
   `incremental = true`. This shrinks each `target/` at the source, applies on
   the next build in every worktree (no lock, no path change), and is reversible
   by deleting the stanzas. To avoid a coordinated cold-rebuild storm, land it in
   a **single combined commit** merged in a low-activity window.

2. **Layered cache relocation off the full mount.**
   - **Override (per-worktree isolation):** a committed `.envrc` and the
     `.config/wt.toml` `rust` post-start export a per-worktree
     `CARGO_TARGET_DIR=$HOME/.cache/anvil-targets/<worktree-slug>` on `/home`.
     Each worktree gets an isolated target with no cargo dir-lock contention.
   - **Floor (bypass guard):** a loud, non-blocking guard — direnv's own
     "blocked, run `direnv allow`" nag, plus a `wt` pre-commit warning when
     `CARGO_TARGET_DIR` is unset.

   > **Amended during DEVENV-002 implementation (PR #2090-series).** The original
   > "bypass-proof floor = committed `.cargo/config.toml` `target-dir`" is **not
   > implementable**: cargo's config `target-dir` does **not** expand `$HOME`
   > (verified — it creates a literal `$HOME/` dir), a hardcoded `/home/...` path
   > is not committable, and a parent-dir operator config would relocate sibling
   > projects too. Relocation is therefore env-driven (direnv/`wt`), with the
   > operator accepting the residual porousness (a shell using neither) in
   > exchange for a fully-committed, CI-safe change. Because nothing relocates on
   > CI runners (no direnv, no `wt`), the nx/Azure cache is unaffected and the
   > DEVENV-002→DEVENV-003 hard dependency is dissolved (DEVENV-003 deferred).

3. **nx-rust executor relocation-awareness (correctness, not optional).** The
   `@eddacraft/nx-rust` `build` target is `cache: true` with
   `outputs: ['{options.target-dir}', '{workspaceRoot}/target']` — nx does not
   read `CARGO_TARGET_DIR`. After relocation, the executors must resolve
   `CARGO_TARGET_DIR` → pass `--target-dir` (so nx's output tracking and the
   Azure remote cache stay correct) and `touch`/remove a `.anvil-building`
   sentinel around the cargo invocation (so eviction can detect nx-driven
   builds). Without this, relocation silently breaks the nx/Azure cache and
   leaves nx builds unguarded against eviction. Touches ADR-021 territory.

4. **Disk-pressure eviction (race-safe, self-enforcing).** A `systemd --user`
   timer runs `scripts/cache/anvil-target-evict.sh`, high-water-mark gated
   (act only above a `/home` threshold), LRU-by-mtime. A dir is skipped if a
   non-blocking `flock -n` on its `.cargo-lock` fails, its newest mtime is within
   a freshness window, or a fresh `.anvil-building` sentinel is present. The
   script asserts a hard `$ANVIL_TARGET_BASE` prefix and **fails closed**;
   reclaim of orphaned in-tree targets is an opt-in `wt clean-stale-targets`, not
   a blind sweep. Ships **dry-run-first** (log-only) until one cycle confirms it
   never selects a building dir. This self-enforcing prefix guard substitutes for
   the PreToolUse Bash safety hooks, which are currently no-ops.

5. **Toolchain alignment now; version manager to the spike.** Reconcile Node to a
   single `better-sqlite3`-compatible version across `.nvmrc` / `engines` / CI /
   the box; fix the global-`oxfmt` shadow so the pinned 0.51 wins. Defer adopting
   `mise`/`.tool-versions` to the spike. Batch the nx cache-key Node-version
   hardening (a `runtime` input so a Node change busts the key) **into this
   alignment commit**, since aligning the toolchain busts `sharedGlobals` anyway
   — one cache-bust, not two.

6. **Worktree lifecycle via `.config/wt.toml`.** Branch new worktrees off
   `origin/main` (fetch-before-create, not stale local main); warm the
   dist-producing workspace packages in post-start so `dist/`/symlinks exist
   before typecheck; and stop swallowing the post-start build's failures
   (`2>/dev/null || true`) so a broken bootstrap is visible, not silent.

7. **Parity via the shared classifier.** Wire `.config/wt.toml`'s pre-commit hook
   to the change-scoped `scripts/validate/local.sh` (the CI-classifier-shared
   tool) instead of blunt full runs, and extend `classify-changes.sh` so
   E2E-impacting paths require the E2E surface — closing the path-gate that hid
   the observability break in both CI and local at once.

### Wave 2 (deferred to the spike)

`DEVENV-008` evaluates a clean-slate reproducible base and **exits with a
go/no-go ADR**. Explicitly in scope for the spike (not wave 1): `mise`/devcontainer/
Nix as the determinism substrate; **nx-cache-as-local-dedup** (route local builds
through `nx build` + make the nx cache shared or remote-readable — a candidate
that may beat `sccache`); `sccache`; `incremental = false` / `split-debuginfo =
"packed"` footprint experiments; `CARGO_HOME` consolidation; and a `mold`/`lld`
linker swap.

## Rationale

The disk failure has two independent axes — **location** (where the live
`target/` sits) and **reuse** (avoiding rebuilds). Wave 1 fixes location
(relocation) and base size (profile trim); reuse (nx-cache/sccache) is deferred
because it is a larger, measurable trade-off the spike should own.

The council originally sought a committed config floor to make relocation
bypass-proof, but implementation proved that impossible (cargo config does not
expand `$HOME` — see the Decision amendment). The accepted resolution is
env-driven relocation (direnv/`wt`): always per-worktree-isolated, never a
shared cargo dir-lock, fully committed, and inert on CI runners. The residual
porousness — a shell using neither direnv nor `wt` builds onto the full mount —
is bounded by a loud guard (direnv's allow-nag + a `wt` pre-commit warning), a
trade-off the operator accepted in exchange for correctness and CI-safety.

### Alternatives Considered

| Option | Pros | Cons |
|--------|------|------|
| **Env-only per-worktree relocation + loud guard (chosen)** | Always isolated (no shared cargo dir-lock); fully committed; inert on CI (nx/Azure cache unaffected); honours "per-worktree" | Porous — a shell using neither direnv nor `wt` builds onto the full mount; bounded by direnv's allow-nag + a `wt` pre-commit warning |
| Committed `.cargo/config.toml` bypass-proof floor (originally planned) | Would catch every bypass regardless of shell | **Not implementable** — cargo config doesn't expand `$HOME` (creates a literal `$HOME/` dir); a hardcoded `/home/...` path isn't committable; a parent-dir operator config would relocate sibling projects too |
| Single shared `CARGO_TARGET_DIR` | Maximal dedup, simplest config | Cargo dir-lock serialises all concurrent agent builds; cross-branch fingerprint thrash; needs eviction |
| `sccache` as the wave-1 core | Compile reuse without lock contention | Extra moving parts; operator-skeptical; doesn't relocate the live target (disk axis unsolved) — better evaluated in the spike |
| Adopt `mise`/Nix now | One determinism substrate immediately | Changes every agent's env at once on a shared box mid-flight; the reproducible base is exactly what the spike must evaluate, not pre-empt |

## Consequences

- **Positive:** ENOSPC stops recurring; each `target/` shrinks at the base; fresh
  worktrees stop failing typecheck on untouched files; local validation matches
  CI's classifier; latent E2E breaks surface pre-merge; the nx/Azure build cache
  is unaffected because relocation is inert on CI runners.
- **Negative:** Relocation is env-driven (direnv/`wt`), so a shell using neither
  builds onto the full mount until the guard warns — residual porousness, since
  the committed bypass-proof floor proved unimplementable; the profile trim
  degrades local-variable debugging on dev builds
  (`gdb`/`lldb` locals — override with `RUSTFLAGS=-Cdebuginfo=2` when needed; does
  not affect `panic = "unwind"`/`catch_unwind` per ADR-051).
- **Risks:** Landing shared-state changes under concurrent agents; a profile or
  toolchain change busting caches in a thundering herd; an eviction misfire on a
  shared box; the nx-executor change interacting with ADR-021's plugin.
- **Mitigations:** Combined single-commit profile change merged in a low-activity
  window; one batched cache-bust for the toolchain alignment; eviction ships
  dry-run-first with a fail-closed prefix assertion; relocation is reversible
  (documented revert order: eviction → relocation → profile); the
  reproducible-base decision is deferred behind a bounded spike with a go/no-go
  ADR.

## References

- Related ADRs: ADR-021 (in-house nx-rust plugin — executor change touches it),
  ADR-049 (cross-language `^build` contract), ADR-051 (panic=unwind; profile-trim
  risk note), ADR-018 (proprietary boundary — dev-env tooling is internal)
- APS modules: DEVENV-001..008 (`plans/modules/dev-environment-hardening.aps.md`)
- Planning council: session `plan-6b3be127` (interrogation + architect/adversarial
  negotiation)
- Related PRs: #2086 (origin/dev base-ref fix, already landed), #2074
  (observability E2E build-graph fix), #2069 (clawpatch periodic-scan filing)
