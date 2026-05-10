# Worktree Policy

## Overview

Use worktrees as lightweight execution spaces for active branches. The branch or
PR is the unit of work; the worktree is only the local workspace.

This policy now separates current compatibility execution from the target
`main`-first model. Until `OPMODEL-012` completes the branch cutover, normal
work still branches from `dev` and normal PRs still target `dev`.

## Permanent Worktrees

### Current Compatibility Model

Keep two long-lived anchor worktrees:

1. `main`
2. `dev`

Suggested directories:

- `../anvil.main`
- `../anvil.dev`

`main` is the stable release anchor. `dev` is the active integration anchor.

### Target Model

After `OPMODEL-012`, keep one long-lived product anchor:

1. `main`

`dev` should be removed locally unless it is explicitly retained as a dated
compatibility branch. Normal work should not require a permanent `dev` worktree
after cutover.

## Disposable Worktrees

Create disposable worktrees for active streams only:

- `feat/*`
- `fix/*`
- `docs/*`
- `chore/*`
- `release/*`
- `hotfix/*`
- short-lived spikes

Suggested directory pattern:

- `../wt-<branch-slug>`

Examples:

- `../wt-docsauth`
- `../wt-rcli-038`
- `../wt-release-0.3.0`
- `../wt-hotfix-auth`

## Branch Creation Rules

Current compatibility rules, before `OPMODEL-012`:

1. Create normal work branches from `dev`.
2. Create release branches from `dev`.
3. Create hotfix branches from `main` or the active `release/*` branch.
4. Merge completed work into its target branch, then remove the worktree.

Target rules, after `OPMODEL-012`:

1. Create normal work branches from `main`.
2. Create `release/*` only when `main` cannot be tagged directly and the branch
   has an explicit expiry.
3. Create `hotfix/*` from `main`, or from the latest good tag only for an
   incident where `main` is unreleasable.
4. Merge completed work into its target branch, then remove the worktree.

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

## Review Rhythm

Review open worktrees at least twice a week.

Check for:

1. merged branches that still have a worktree
2. stale branches with no recent progress
3. branches that should be split or rebased
4. streams that should be promoted into the current integration target

## Practical Rule of Thumb

Before `OPMODEL-012`:

1. Keep `main` and `dev` anchors available.
2. Open disposable worktrees for active streams from `dev`.
3. Remove them as soon as the stream is merged, replaced, or paused.

After `OPMODEL-012`:

1. Keep `main` as the product anchor.
2. Open disposable worktrees for active streams from `main`.
3. Remove them as soon as the stream is merged, replaced, or paused.
4. If a worktree feels permanent, merge, split, or close the stream.

## Related Docs

- [Branching Strategy](branching-strategy.md)
- [Release Runbook](release-runbook.md)
- [Operating Model Spec](../../plans/specs/2026-05-09-plan-build-release-operating-model.md)
