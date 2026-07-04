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

if [ "$fail" = 0 ]; then echo "ALL PASS"; else echo "FAILURES"; exit 1; fi
