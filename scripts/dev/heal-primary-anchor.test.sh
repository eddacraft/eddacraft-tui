#!/usr/bin/env bash
# Tests for heal-primary-anchor.sh — covers the three contracts:
#   1. a pure wt strand is healed (tree resynced to HEAD, clean)
#   2. genuine uncommitted work is PRESERVED via stash, never reset away
#   3. a clean anchor is a no-op (no stash, no change)
set -euo pipefail

repo_root=$(git rev-parse --show-toplevel 2>/dev/null || pwd)
heal="${repo_root}/scripts/dev/heal-primary-anchor.sh"
[ -x "$heal" ] || { echo "FAIL: $heal not executable"; exit 1; }

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
fail=0
check() { # name  got  want
  if [ "$2" = "$3" ]; then echo "ok: $1"; else echo "FAIL: $1 (want '$3' got '$2')"; fail=1; fi
}

# Count stashes without tripping `set -e`: `grep -c` exits 1 on zero matches,
# which would fail the surrounding command substitution.
stash_count() { git -C "$1" stash list | wc -l | tr -d ' '; }

new_repo() { # n  -> echoes repo path with two commits (c1: v1, c2: v2)
  local repo="$tmp/repo$1"
  git init -q -b main "$repo"
  git -C "$repo" config user.email t@e.invalid
  git -C "$repo" config user.name 'Test User'
  printf 'v1\n' >"$repo/f.txt"; git -C "$repo" add f.txt; git -C "$repo" commit -q -m c1
  printf 'v2\n' >"$repo/f.txt"; git -C "$repo" add f.txt; git -C "$repo" commit -q -m c2
  echo "$repo"
}

# ── Case 1: pure strand (tree at c1, HEAD ref advanced to c2 out of band) ──────
repo=$(new_repo 1)
c1=$(git -C "$repo" rev-parse HEAD~1)
c2=$(git -C "$repo" rev-parse HEAD)
git -C "$repo" reset -q --hard "$c1"                # tree + HEAD at c1
git -C "$repo" update-ref refs/heads/main "$c2"     # advance ref only → strand
[ -n "$(git -C "$repo" status --porcelain)" ] || { echo "FAIL: case1 setup not dirty"; fail=1; }
( cd "$repo" && bash "$heal" ) >/dev/null 2>&1 || true
check "strand: healed to clean"      "$(git -C "$repo" status --porcelain)"        ""
check "strand: HEAD unchanged (c2)"  "$(git -C "$repo" rev-parse HEAD)"            "$c2"
check "strand: tree resynced to v2"  "$(cat "$repo/f.txt")"                        "v2"
check "strand: no stash created"     "$(stash_count "$repo")"    "0"

# ── Case 2: genuine uncommitted work must be preserved, not reset ──────────────
repo=$(new_repo 2)
printf 'REAL WORK\n' >"$repo/f.txt"                 # real edit atop HEAD, not a strand
printf 'new\n' >"$repo/untracked.txt"               # + an untracked file
( cd "$repo" && bash "$heal" ) >/dev/null 2>&1 || true
check "real work: tree cleaned"      "$(git -C "$repo" status --porcelain)"        ""
check "real work: stashed once"      "$(stash_count "$repo")"    "1"
git -C "$repo" stash pop -q
check "real work: tracked recovered" "$(cat "$repo/f.txt")"                        "REAL WORK"
check "real work: untracked recovered" "$(cat "$repo/untracked.txt")"             "new"

# ── Case 3: clean anchor is a no-op ───────────────────────────────────────────
repo=$(new_repo 3)
( cd "$repo" && bash "$heal" ) >/dev/null 2>&1 || true
check "clean: still clean"           "$(git -C "$repo" status --porcelain)"        ""
check "clean: no stash created"      "$(stash_count "$repo")"    "0"

# ── Case 4: untracked-only anchor is not a strand ─────────────────────────────
# Regression: this used to stash the untracked file under a "preserved for
# review" label. Nothing here is at risk — `git reset --hard` never touches
# untracked files — so the heal must leave them alone and create no stash.
repo=$(new_repo 4)
printf 'generated\n' >"$repo/baseline.json"          # regenerable state, untracked
( cd "$repo" && bash "$heal" ) >/dev/null 2>&1 || true
check "untracked-only: no stash created"   "$(stash_count "$repo")"           "0"
check "untracked-only: file left in place" "$(cat "$repo/baseline.json")"     "generated"

# ── Case 5: a strand is still healed when untracked files sit alongside it ────
# Regression: untracked files used to block the strand proof outright, forcing
# a provable strand down the stash path. `git reset --hard` leaves untracked
# files alone, so the strand must heal AND the untracked file must survive.
repo=$(new_repo 5)
c1=$(git -C "$repo" rev-parse HEAD~1)
c2=$(git -C "$repo" rev-parse HEAD)
git -C "$repo" reset -q --hard "$c1"
git -C "$repo" update-ref refs/heads/main "$c2"
printf 'scratch\n' >"$repo/scratch.txt"              # untracked, alongside the strand
( cd "$repo" && bash "$heal" ) >/dev/null 2>&1 || true
check "strand+untracked: tree resynced"     "$(cat "$repo/f.txt")"            "v2"
check "strand+untracked: no stash created"  "$(stash_count "$repo")"          "0"
check "strand+untracked: untracked kept"    "$(cat "$repo/scratch.txt")"      "scratch"

# ── Case 6: the lock is repo-scoped, not $TMPDIR-scoped ───────────────────────
# Regression: the lock path was "${TMPDIR:-/tmp}/anvil-heal-primary-anchor.lock".
# Every agent has its own $TMPDIR, so each took a private lock and concurrent
# heals raced — the losers stashed a tree the winner had already cleaned.
repo=$(new_repo 6)
fake_tmp="$tmp/faketmp"; mkdir -p "$fake_tmp"
printf 'REAL WORK\n' >"$repo/f.txt"
( cd "$repo" && TMPDIR="$fake_tmp" bash "$heal" ) >/dev/null 2>&1 || true
check "lock: none created under \$TMPDIR" \
  "$(find "$fake_tmp" -name 'anvil-heal-primary-anchor.lock*' | wc -l | tr -d ' ')" "0"
check "lock: created in the git dir" \
  "$(find "$repo/.git" -maxdepth 1 -name 'anvil-heal-primary-anchor.lock*' | wc -l | tr -d ' ')" "1"

# ── Case 7: a second heal cannot pile on while one holds the lock ─────────────
# The winner's exclusion must make the loser a clean no-op, not a stasher.
# Skipped rather than silently vacuous where flock is absent — case 8 covers
# the mkdir fallback that such a host would actually use.
if command -v flock >/dev/null 2>&1; then
  repo=$(new_repo 7)
  printf 'REAL WORK\n' >"$repo/f.txt"
  (
    cd "$repo"
    exec 8>"$repo/.git/anvil-heal-primary-anchor.lock"
    flock -n 8 || exit 0                              # hold the lock, then heal
    bash "$heal" >/dev/null 2>&1 || true
  ) || true
  check "lock held: loser created no stash" "$(stash_count "$repo")" "0"
  check "lock held: work untouched"         "$(cat "$repo/f.txt")"   "REAL WORK"
else
  echo "skip: lock held (flock unavailable on this host)"
fi

# ── Case 8: the mkdir fallback lock excludes a second heal too ────────────────
# HEAL_FORCE_MKDIR_LOCK exercises the no-flock branch on any host. Pre-creating
# the lock directory simulates a heal already in flight.
repo=$(new_repo 8)
printf 'REAL WORK\n' >"$repo/f.txt"
mkdir "$repo/.git/anvil-heal-primary-anchor.lock.d"
( cd "$repo" && HEAL_FORCE_MKDIR_LOCK=1 bash "$heal" ) >/dev/null 2>&1 || true
check "mkdir lock held: no stash created" "$(stash_count "$repo")" "0"
check "mkdir lock held: work untouched"   "$(cat "$repo/f.txt")"   "REAL WORK"
rmdir "$repo/.git/anvil-heal-primary-anchor.lock.d"
# ...and releases the lock so the next heal proceeds normally.
( cd "$repo" && HEAL_FORCE_MKDIR_LOCK=1 bash "$heal" ) >/dev/null 2>&1 || true
check "mkdir lock free: work preserved"   "$(stash_count "$repo")" "1"
check "mkdir lock: released on exit" \
  "$([ -d "$repo/.git/anvil-heal-primary-anchor.lock.d" ] && echo present || echo gone)" "gone"

# ── Case 9: staged-only work must never be reset away ─────────────────────────
# `git stash create` commits the working tree, so staging a change and then
# restoring the file leaves a snapshot whose tree matches HEAD while the index
# does not. That mimics a healed anchor exactly; the strand proof must reject it
# on the index, or `git reset --hard` destroys the staged work outright.
repo=$(new_repo 9)
printf 'STAGED WORK\n' >"$repo/f.txt"
git -C "$repo" add f.txt
printf 'v2\n' >"$repo/f.txt"                          # working tree back to HEAD
( cd "$repo" && bash "$heal" ) >/dev/null 2>&1 || true
check "staged-only: not reset away"  "$(git -C "$repo" stash list | wc -l | tr -d ' ')" "1"
# `--index` restores the staged state too; a plain pop would bring the content
# back only as an unstaged edit.
git -C "$repo" stash pop --index -q >/dev/null 2>&1 || true
check "staged-only: staged content recovered" \
  "$(git -C "$repo" show :f.txt 2>/dev/null)" "STAGED WORK"

if [ "$fail" = 0 ]; then echo "ALL PASS"; else echo "FAILURES"; exit 1; fi
