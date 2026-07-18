#!/usr/bin/env bash
# DEVENV-006 (ADR-057): create a Worktrunk worktree branched off a FRESH
# origin/main, not the (possibly stale) local main tip.
#
# `wt switch --create` bases new branches on the *local* default-branch ref,
# which is whatever the last `git fetch`/pull left behind. On a long-lived
# checkout that is routinely behind origin/main, so fresh worktrees start behind
# the integration target and conflict on merge (bit PR #2070). `wt` has no
# pre-create hook and no auto-fetch/default-base config, so the only reliable
# fix is to fetch first and pass the fetched ref as the base — which is exactly
# what this wrapper does. (`wt` accepts a remote-tracking ref as --base; it is
# delegated to `git worktree add`.)
#
# Usage:
#   scripts/dev/wt-new.sh <branch-name> [extra `wt switch` args...]
#
# Examples:
#   scripts/dev/wt-new.sh feat/widget-123
#   scripts/dev/wt-new.sh fix/login -x claude        # launch claude in the worktree
#
# Equivalent one-liner if you'd rather not use the wrapper:
#   git fetch origin main && wt switch --create <branch> --base origin/main
set -euo pipefail

if [ "$#" -lt 1 ]; then
  echo "usage: scripts/dev/wt-new.sh <branch-name> [extra 'wt switch' args...]" >&2
  exit 2
fi

branch="$1"
shift

# Fetch the integration target so origin/main reflects the remote tip, then
# branch off it. We deliberately do NOT touch the local `main` ref (it may be
# checked out in a shared worktree that siblings are parked in).
#
# Use an explicit refspec that updates refs/remotes/origin/main. A bare
# `git fetch origin main` lands in FETCH_HEAD and does not reliably refresh the
# remote-tracking ref that --base consumes.
git fetch --quiet origin refs/heads/main:refs/remotes/origin/main
exec wt switch --create "$branch" --base origin/main "$@"
