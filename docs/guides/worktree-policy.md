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

1. Create normal work branches from `main`.
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

## Related Docs

- [Branching Strategy](branching-strategy.md)
- [Release Runbook](../runbooks/release-runbook.md)
- [Main-First Cutover Runbook](../runbooks/main-first-cutover.md) (historical
  evidence of the 2026-05-11 cutover)
- [Operating Model Spec](../../plans/specs/2026-05-09-plan-build-release-operating-model.md)
