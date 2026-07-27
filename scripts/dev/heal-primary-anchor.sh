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
# working state is byte-identical to a committed ancestor of HEAD. Anything it
# cannot prove is a strand is preserved with `git stash` — never discarded.
# Safe to run anytime; wired as wt post-switch / post-merge / post-remove hooks
# in .config/wt.toml.
set -euo pipefail

log() { printf 'heal-primary-anchor: %s\n' "$*" >&2; }

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

# Serialise concurrent heals — many agents run wt in parallel on one machine.
#
# The lock lives in the repository's common git dir, NOT under $TMPDIR. Each
# agent process gets its own $TMPDIR, so a $TMPDIR-derived path handed every
# agent a private lock file and serialised nothing: concurrent heals raced, and
# a loser stashed a working tree the winner had already cleaned. That is how
# empty "preserved for review" stashes accumulated.
common_dir="$(git rev-parse --path-format=absolute --git-common-dir 2>/dev/null || true)"
[ -n "$common_dir" ] || common_dir="$(git rev-parse --git-common-dir)"
case "$common_dir" in /*) ;; *) common_dir="${anchor}/${common_dir}" ;; esac
lock_file="${common_dir}/anvil-heal-primary-anchor.lock"

if command -v flock >/dev/null 2>&1; then
  exec 9>"$lock_file"
  flock -n 9 || { log "another heal is running; skipping"; exit 0; }
else
  # Portable fallback: an atomic mkdir lock released on exit. Without it a host
  # lacking flock has no serialisation at all.
  lock_dir="${lock_file}.d"
  if mkdir "$lock_dir" 2>/dev/null; then
    trap 'rmdir "$lock_dir" 2>/dev/null || true' EXIT
  else
    log "another heal is running; skipping"
    exit 0
  fi
fi

# Everything below runs under the lock, so the state observed here is the state
# acted on. Re-read it now rather than trusting anything sampled before the lock
# was held.
tracked_dirty() { ! { git diff --quiet && git diff --cached --quiet; }; }
untracked_files() { git ls-files --others --exclude-standard; }

if ! tracked_dirty && [ -z "$(untracked_files)" ]; then
  exit 0
fi

if ! tracked_dirty; then
  # Untracked files only. That is not a wt strand: the anchor's tracked state
  # already matches HEAD, and `git reset --hard` would not have removed these
  # files anyway, so there is nothing to protect them from. Stashing them here
  # only hid files someone left in the anchor — regenerable state such as
  # anvil/baseline.json — behind a "preserved for review" label nobody read.
  log "anchor '$anchor' holds only untracked files; not a strand, leaving them in place"
  exit 0
fi

head="$(git rev-parse HEAD)"

# Prove a pure strand: snapshot the tracked working state as a commit without
# touching the tree (`git stash create`), then look for a committed ancestor of
# HEAD with the identical tree. A match means a hard reset to HEAD discards
# nothing that is not already committed in that ancestor.
#
# Untracked files deliberately do NOT block this proof: `git reset --hard`
# leaves untracked files alone, so they are never at risk. Requiring their
# absence only forced provable strands down the stash path.
provable_ancestor=""
snap="$(git stash create 2>/dev/null || true)"
if [ -n "$snap" ]; then
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
  exit 0
fi

# Unprovable → preserve. Compare the stash ref before and after so a push that
# saved nothing can be told apart from one that saved real work.
empty_tree="$(git hash-object -t tree /dev/null)"

# True when a stash commit records no working-tree delta and no untracked
# files — it captured nothing and is pure debris.
stash_captured_nothing() {
  local s="$1" stash_tree parent_tree untracked_tree
  stash_tree="$(git rev-parse "${s}^{tree}")"
  parent_tree="$(git rev-parse "${s}^1^{tree}")"
  [ "$stash_tree" = "$parent_tree" ] || return 1
  if git rev-parse --quiet --verify "${s}^3" >/dev/null 2>&1; then
    untracked_tree="$(git rev-parse "${s}^3^{tree}")"
    [ "$untracked_tree" = "$empty_tree" ] || return 1
  fi
  return 0
}

before="$(git rev-parse --quiet --verify refs/stash 2>/dev/null || true)"
git stash push --include-untracked \
  --message "primary-anchor: unexpected changes in the '$default_branch' anchor — NOT a provable wt strand; preserved for review" >/dev/null || true
after="$(git rev-parse --quiet --verify refs/stash 2>/dev/null || true)"

if [ "$after" = "$before" ]; then
  log "anchor '$anchor' had nothing left to preserve; no stash created"
  exit 0
fi

if stash_captured_nothing "$after"; then
  git stash drop >/dev/null 2>&1 || true
  log "anchor '$anchor' produced an empty stash (nothing to preserve); dropped it"
  exit 0
fi

log "anchor '$anchor' held changes that are NOT a provable wt strand; preserved via 'git stash' (review: git -C '$anchor' stash list)"
