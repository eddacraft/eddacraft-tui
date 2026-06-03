# Worktree Policy

| Type  | Authority     | Owner   | Status | Freshness                                                                                                  |
| ----- | ------------- | ------- | ------ | ---------------------------------------------------------------------------------------------------------- |
| Guide | Authoritative | OPMODEL | Live   | Last reviewed 2026-05-25 against `docs/guides/branching-strategy.md` and Worktrunk-managed branch workflow |

| Upstream                                                                      | Downstream                                         |
| ----------------------------------------------------------------------------- | -------------------------------------------------- |
| `docs/guides/branching-strategy.md`, `docs/guides/agent-surface-inventory.md` | `AGENTS.md`, `CLAUDE.md`, `wt switch`, `wt remove` |

## Overview

Use Worktrunk (`wt`) to manage Git worktrees as lightweight execution spaces for
active branches. The branch or PR is the unit of work; the worktree is only the
local workspace. Agents should default to a Worktrunk-managed worktree for every
non-trivial task and offer cleanup at the end of the task.

Following the OPMODEL-012 cutover on 2026-05-11, `main` is the only permanent
product anchor. `dev` is retired (see
[branching-strategy](branching-strategy.md#archive--pre-opmodel-012-compatibility-model)).

## Permanent Worktrees

Keep one long-lived product anchor:

1. `main`

Suggested directory: `../anvil.main`.

Do not keep a permanent `dev` worktree. If a local `dev` worktree exists, remove
it:

```bash
git worktree remove ../anvil.dev      # or wherever the dev worktree lives
```

## Disposable Worktrees

Create Worktrunk-managed worktrees for active streams only:

- `feat/*`
- `fix/*`
- `docs/*`
- `chore/*`
- `release/*`
- `hotfix/*`
- short-lived spikes

Worktrunk computes paths from its configured template. The legacy manual
directory pattern is:

- `../wt-<branch-slug>`

Examples:

- `../wt-docsauth`
- `../wt-rcli-038`
- `../wt-release-0.3.0`
- `../wt-hotfix-auth`

## Branch Creation Rules

1. Create normal work branches from a **fresh** `origin/main`.
   `wt switch --create` bases new branches on the _local_ default-branch ref,
   which on a long-lived checkout is routinely behind the remote — so a worktree
   created without fetching first starts behind the integration target and
   conflicts on merge (bit PR #2070). Use the committed wrapper, which fetches
   first (DEVENV-006):

   ```bash
   scripts/dev/wt-new.sh feat/my-branch        # = fetch origin main, then base off it
   # equivalent, by hand:
   git fetch origin main && wt switch --create feat/my-branch --base origin/main
   ```

2. Create `release/*` only when `main` cannot be tagged directly and the branch
   has an explicit expiry.
3. Create `hotfix/*` from `main`, or from the latest good tag only for an
   incident where `main` is unreleasable.
4. Merge completed work into `main`, then offer `wt remove` for the worktree and
   local branch.

## Why Disposable Is the Default

Disposable worktrees reduce drift and maintenance overhead.

Permanent feature worktrees tend to accumulate:

- stale branches
- hidden divergence from the integration target
- rebasing overhead
- unfinished work that feels active but is not moving

Use the branch and PR as the durable record. Remove local worktrees once the
stream is merged, abandoned, superseded, or blocked without near-term action.

## Age Limits

Use these limits as hygiene rules rather than hard technical constraints:

- feature, fix, docs, chore: target under 5 active days
- release worktree: target under 3 days of stabilisation
- spike worktree: target under 2 days before convert-or-close

Any disposable worktree older than 7 days should be reviewed immediately and
either:

- merged
- split into smaller branches
- rebased and continued with intent
- closed and removed

## WIP Limits

1. Keep no more than 4-5 disposable worktrees open at once.
2. If you reach the limit, do not create another until one is merged, paused, or
   removed.
3. If a stream is blocked and you are not returning within 48 hours, remove the
   worktree and keep the branch reference only if needed.

## Cleanup Rules

Remove disposable worktrees when:

1. the branch is merged
2. the branch is abandoned
3. the branch is superseded by a replacement branch
4. the branch is blocked with no near-term next action

Delete merged disposable branches and remove their worktrees on the same day.

Agents should offer `wt remove` at natural finish points:

1. after opening a PR when local iteration is done
2. after a PR merges
3. when a branch is abandoned or superseded
4. when a branch is paused with no near-term next action

Before cleanup, verify the worktree is clean and the branch is either pushed,
merged, or explicitly approved for deletion. Prefer `wt remove`; use raw Git
cleanup commands only when Worktrunk cannot express the required recovery path.

## Review Rhythm

Review open worktrees at least twice a week.

Check for:

1. merged branches that still have a worktree
2. stale branches with no recent progress
3. branches that should be split or rebased
4. streams that should be promoted into the current integration target

## Practical Rule of Thumb

1. Keep `main` as the product anchor.
2. Open Worktrunk worktrees for active streams from `main`.
3. Remove them with `wt remove` as soon as the stream is merged, replaced, or
   paused.
4. If a worktree feels permanent, merge, split, or close the stream.

## Updating an existing clone

If your local clone predates the cutover, `origin/HEAD` may still point to
`dev`. Update it once:

```bash
git fetch origin --prune
git remote set-head origin --auto
git symbolic-ref refs/remotes/origin/HEAD     # must print refs/remotes/origin/main
```

Without this, `gh pr create` may still propose `dev` as the default base.

## Cargo target relocation (DEVENV-002)

Per-worktree Rust `target/` dirs are large (~100 GB each) and the Projects mount
fills fast — a full mount means `ENOSPC` for every worktree at once. To keep
builds off that mount, each worktree relocates its Cargo target onto `/home` via
`CARGO_TARGET_DIR=$HOME/.cache/anvil-targets/<worktree-dir-name>` (per-worktree,
so no shared cargo build lock).

Two committed mechanisms set it:

- **direnv** — the committed `.envrc` exports it. Run `direnv allow` once per
  worktree. Until you do, direnv prints a loud "blocked" nag on every `cd` —
  that nag is the intended guard against silently building onto the full mount.
- **`wt`** — the `.config/wt.toml` `rust` post-start exports the same, so
  `wt`-driven worktrees relocate even without a direnv-hooked shell. A
  non-blocking `wt` pre-commit warning fires if `CARGO_TARGET_DIR` is unset.

If you use **neither** direnv nor `wt` in a shell, cargo builds into the in-tree
`target/` on the full mount — reclaim a stray one with `cargo clean` (automated
eviction lands with DEVENV-004). The relocation is inert on CI runners (no
direnv, no `wt`), so the nx/Azure build cache is unaffected.

## Node version (DEVENV-005)

anvil standardises on **Node 24** — `.nvmrc` = 24, `engines.node` `>=24.0.0`,
and CI's `setup-workspace` default = 24. The native `better-sqlite3` (in
`@eddacraft/anvil-edda-stack`) only has prebuilt binaries for the versions the
project targets; running a different major (e.g. Node 26) triggers the ABI
mismatch that fails `edda-stack` tests.

You don't have to give up other Node versions globally. Use **`fnm`** with
auto-switch so the anvil repo picks up `.nvmrc` (24) while your global default
(e.g. 26) stays for everything else:

```bash
# one-time shell setup (zsh): auto-switch on cd into a dir with .nvmrc
eval "$(fnm env --use-on-cd)"
fnm install 24            # once
# then, in any anvil worktree, fnm selects 24 automatically on cd.
# after first switching a worktree to 24, rebuild the native module:
pnpm rebuild better-sqlite3   # or: pnpm install
```

This composes with the `.envrc` above (fnm owns the Node version; direnv owns
`CARGO_TARGET_DIR` + the pinned-tool PATH).

> **If another manager shadows fnm:** fnm only wins if its shims sit ahead of
> any other `node` on `PATH`. A Homebrew (`brew install node`) or system node
> linked into a higher-priority `bin` will shadow it, so `--use-on-cd` switches
> fnm's dir but `node` still resolves to the other one. Fix with
> `brew unlink node` (not `brew uninstall` — Homebrew formulae like `corepack`
> and `neonctl` depend on node; unlink keeps it in the Cellar for them while
> removing the `PATH` shadow). Confirm with `command -v node` resolving under
> `~/.local/state/fnm_multishells/…`. After switching a worktree to 24, rebuild
> the native module: `pnpm rebuild better-sqlite3`.

## Worktree bootstrap (DEVENV-006)

`wt`-managed worktrees run a `post-start` chain defined in `.config/wt.toml`.
Three behaviours matter for a clean fresh worktree:

- **Branch off a fresh `origin/main`.** See
  [Branch Creation Rules](#branch-creation-rules) — use
  `scripts/dev/wt-new.sh <branch>` (it fetches before
  `wt switch --create … --base origin/main`). `wt` has no pre-create hook, so
  the fetch cannot live in `.config/wt.toml`; it has to happen at create time,
  which is what the wrapper is for.
- **Workspace symlinks are fully reconciled before `typecheck`.** The `copy`
  step seeds `node_modules` wholesale from a sibling worktree, which can be
  internally inconsistent — some per-consumer `workspace:*` symlinks present,
  others missing. A plain `pnpm install` trusts the copied
  `node_modules/.modules.yaml` and does a partial pass that leaves the missing
  links missing (e.g. `apps/anvil-api` without
  `@eddacraft/anvil-observability`), so a first `pnpm typecheck` fails on
  untouched files with `TS2307: Cannot find module`. The `install` post-start
  therefore removes `.modules.yaml` first, forcing pnpm to relink every importer
  from the warm global store in one pass (a few seconds, no re-fetch). Package
  `dist/` itself is carried over by the `copy` step, so this is specifically
  about the symlinks, not build output.
- **Bootstrap failures are loud, not swallowed.** The `rust` and `dist`
  post-start steps no longer send stderr to `/dev/null` or `|| true` their
  failures; a broken bootstrap prints a `WARNING (DEVENV-006)` line with the
  build output above it. The steps stay non-fatal on purpose — a transient break
  should not lock you out of the very worktree you need to fix it in.

## oxfmt resolution in fresh worktrees (CIB-032)

`format` / `format:check` (and post-edit hook) now invoke oxfmt via
`pnpm exec oxfmt` (package.json) or explicit local `.bin` first (hook). This
guarantees the workspace-pinned version once `node_modules` exists, and produces
a clear actionable failure (plus guidance) if a fresh/manual worktree has no
`node_modules` yet.

`wt`-managed worktrees run the `install` post-start which ensures `node_modules`
(and symlinks). Manual `git worktree add` or cleaned trees still require an
explicit `pnpm install` first; the scripts no longer silently fall back to a
stale global `oxfmt` on PATH.

See CIB-032 in `plans/modules/continuous-improvement-backlog.aps.md`.

## Related Docs

- [Branching Strategy](branching-strategy.md)
- [Release Runbook](../runbooks/release-runbook.md)
- [Main-First Cutover Runbook](../runbooks/main-first-cutover.md) (historical
  evidence of the 2026-05-11 cutover)
- [Operating Model Spec](../../plans/specs/2026-05-09-plan-build-release-operating-model.md)
