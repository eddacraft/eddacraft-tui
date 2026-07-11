#!/usr/bin/env bash
# Fixture tests for scripts/ci-log/* (CIB-191).
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
NODE=(node)

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

fail() {
  printf 'ci-log.test.sh: FAIL: %s\n' "$1" >&2
  exit 1
}

# Isolated fake repo with its own .git so pending lives under tmp.
mkdir -p "$tmp/repo/plans/reviews"
cd "$tmp/repo"
git init -q
git config user.email test@example.com
git config user.name test
# Seed tracked log (minimal header + one old entry)
cat > plans/reviews/continuous-improvement-log.md <<'LOG'
# Continuous Improvement Log

This file captures lightweight session learning from agents.

> **Concurrent writes:** this file is `merge=union`.

> **Last triaged:** 2026-07-01

## Template

## Entries

### 2026-07-01 — opencode

- **Task:** seed
- **Outcome:** ok
- **Worked:** —
- **Failed:** none
- **Friction:** none
- **Improvement:** none
- **Follow-up:** none

LOG
git add plans/reviews/continuous-improvement-log.md
git commit -q -m 'seed'

export GIT_DIR="$tmp/repo/.git"
export GIT_WORK_TREE="$tmp/repo"

# Use absolute paths to scripts from real repo
APPEND=(node "$ROOT/scripts/ci-log/append.mjs")
HARVEST=(node "$ROOT/scripts/ci-log/harvest.mjs")
STATUS=(node "$ROOT/scripts/ci-log/status.mjs")
SINCE=(node "$ROOT/scripts/ci-log/since.mjs")
WATER=(node "$ROOT/scripts/ci-log/set-watermark.mjs")

# 1) append to pending (default)
out="$("${APPEND[@]}" --task 'pending path test' --agent opencode --outcome 'queued' --json)"
echo "$out" | grep -q '"destination": "pending"' || fail "append pending destination"
pending_path="$(printf '%s' "$out" | node -e "let s='';process.stdin.on('data',d=>s+=d);process.stdin.on('end',()=>console.log(JSON.parse(s).path))")"
[[ -f "$pending_path" ]] || fail "pending file missing: $pending_path"
# pending under git common dir, not work tree
case "$pending_path" in
  *'/.git/anvil/ci-log-pending/'*) ;;
  *) fail "pending not under .git/anvil/ci-log-pending: $pending_path" ;;
esac

# tracked log unchanged
grep -q 'pending path test' plans/reviews/continuous-improvement-log.md && fail "pending leaked into tracked" || true

# 2) status sees pending
status_json="$("${STATUS[@]}" --json)"
echo "$status_json" | grep -q '"pendingCount": 1' || fail "status pendingCount"

# 3) harvest moves into tracked
harvest_json="$("${HARVEST[@]}" --json)"
echo "$harvest_json" | grep -q '"harvested": 1' || fail "harvest count"
grep -q 'pending path test' plans/reviews/continuous-improvement-log.md || fail "harvest did not append"
[[ ! -f "$pending_path" ]] || fail "pending file not removed after harvest"
status_json="$("${STATUS[@]}" --json)"
echo "$status_json" | grep -q '"pendingCount": 0' || fail "status after harvest"

# 4) tracked append path
"${APPEND[@]}" --tracked --task 'direct tracked' --agent claude --outcome 'in log' >/dev/null
grep -q 'direct tracked' plans/reviews/continuous-improvement-log.md || fail "tracked append"

# 5) since watermark
since_out="$("${SINCE[@]}" --watermark --headings)"
echo "$since_out" | grep -q 'pending path test\|direct tracked\|2026-07-' || fail "since watermark empty unexpectedly: $since_out"
# entries on/after 2026-07-01 include seed; after 2026-07-12 should only be new if dates match
# set watermark to today and ensure since watermark can be empty for older-only content... skip flaky date cases

# 6) set watermark
"${WATER[@]}" --date 2026-07-10 >/dev/null
grep -q 'Last triaged:\*\* 2026-07-10' plans/reviews/continuous-improvement-log.md || fail "watermark not set"

# 7) reject empty / bad body
if "${APPEND[@]}" --body 'no heading here' 2>/dev/null; then
  fail "accepted body without heading"
fi

# 8) second pending then dry-run harvest
"${APPEND[@]}" --task 'second pending' --agent codex --outcome x >/dev/null
dry="$("${HARVEST[@]}" --dry-run --json)"
echo "$dry" | grep -q '"harvested": 1' || fail "dry-run harvest"
echo "$dry" | grep -q '"dryRun": true' || fail "dryRun flag"
# still pending
status_json="$("${STATUS[@]}" --json)"
echo "$status_json" | grep -q '"pendingCount": 1' || fail "dry-run should not clear pending"

# 9) follow-up field round-trip
"${APPEND[@]}" --task 'promote me' --follow-up 'promote: CIB' --agent opencode --json >/dev/null
pending2="$(ls "$tmp/repo/.git/anvil/ci-log-pending/"*.md | tail -1)"
grep -q 'promote: CIB' "$pending2" || fail "follow-up not written"

printf 'ci-log.test.sh: OK\n'
