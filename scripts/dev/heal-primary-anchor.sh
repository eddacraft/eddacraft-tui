#!/usr/bin/env bash
# heal-primary-anchor.sh — resync the default-branch "anchor" worktree after
# `wt` advances refs/heads/<default> out of band.
#
# ROOT CAUSE (see docs/guides/worktree-policy.md § "Default-branch anchor
# auto-heal"): wt (Worktrunk) keeps the local default branch fast-forwarded to
# origin via an in-process git-library ref write. That write moves
# refs/heads/<default> but does NOT update the working tree of the worktree that
# has <default> checked out — the "anchor". The anchor is then stranded one or
# more merges behind HEAD, and `git status` there renders the gap as a phantom
# "revert of merged work".
#
# This heals that. It is a no-op when the anchor is clean, and it only ever
# hard-resets when it can PROVE the anchor holds a pure strand: its tracked
# working state is byte-identical to a committed ancestor of HEAD and there are
# no untracked files. Anything it cannot prove is a strand is preserved with
# `git stash` — never discarded. Safe to run anytime; wired as wt post-switch /
# post-merge / post-remove hooks in .config/wt.toml.
set -euo pipefail

log() { printf 'heal-primary-anchor: %s\n' "$*" >&2; }

# Serialise concurrent heals — many agents run wt in parallel on one machine.
if command -v flock >/dev/null 2>&1; then
  exec 9>"${TMPDIR:-/tmp}/anvil-heal-primary-anchor.lock"
  flock -n 9 || { log "another heal is running; skipping"; exit 0; }
fi

# Resolve the default branch, falling back to main. Kept pipeline-free so a
# missing origin/HEAD ref can't trip `set -e`/`pipefail`.
default_ref="$(git symbolic-ref --quiet --short refs/remotes/origin/HEAD 2>/dev/null || true)"
default_branch="${default_ref##origin/}"
default_branch="${default_branch:-main}"

# Find the worktree that has <default> checked out — the anchor.
anchor="" cur=""
while IFS= read -r line; do
  case "$line" in
    "worktree "*) cur="${line#worktree }" ;;
    "branch refs/heads/${default_branch}") anchor="$cur" ;;
  esac
done < <(git worktree list --porcelain)
[ -n "$anchor" ] || { log "no worktree on '$default_branch'; nothing to heal"; exit 0; }

cd "$anchor" || { log "cannot enter anchor '$anchor'"; exit 0; }

# Clean anchor → nothing to do (the common case; keep this cheap).
if git diff --quiet && git diff --cached --quiet \
   && [ -z "$(git ls-files --others --exclude-standard)" ]; then
  exit 0
fi

head="$(git rev-parse HEAD)"

# Prove a pure strand: snapshot the tracked working state as a commit without
# touching the tree (`git stash create`), then look for a committed ancestor of
# HEAD with the identical tree. A match + no untracked files means a hard reset
# to HEAD discards nothing that is not already committed in that ancestor.
provable_ancestor=""
snap="$(git stash create 2>/dev/null || true)"
if [ -n "$snap" ] && [ -z "$(git ls-files --others --exclude-standard)" ]; then
  snap_tree="$(git rev-parse --quiet --verify "${snap}^{tree}" 2>/dev/null || true)"
  if [ -n "$snap_tree" ]; then
    while IFS= read -r c; do
      if [ "$(git rev-parse "${c}^{tree}")" = "$snap_tree" ]; then
        provable_ancestor="$c"; break
      fi
    done < <(git rev-list --max-count=500 HEAD)
  fi
fi

if [ -n "$provable_ancestor" ]; then
  git reset --hard "$head" >/dev/null
  log "anchor '$anchor' was stranded at ${provable_ancestor:0:9} (phantom revert of merged work); resynced to ${head:0:9}"
else
  git stash push --include-untracked \
    --message "primary-anchor: unexpected changes in the '$default_branch' anchor — NOT a provable wt strand; preserved for review" >/dev/null
  log "anchor '$anchor' held changes that are NOT a provable wt strand; preserved via 'git stash' (review: git -C '$anchor' stash list)"
fi
