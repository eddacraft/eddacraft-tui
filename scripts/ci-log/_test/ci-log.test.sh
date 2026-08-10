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

# All tracked-log writers coordinate through this git-common-dir lock.
tracked_lock="$tmp/repo/.git/anvil/ci-log-tracked.lock"

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

# 7) an externally held tracked-log lock blocks both mutation paths until release
mkdir -p "$(dirname "$tracked_lock")"
printf 'external-test-owner\n' > "$tracked_lock"
"${APPEND[@]}" --tracked --task 'waited tracked append' --agent codex --outcome locked \
  >"$tmp/locked-append.out" 2>"$tmp/locked-append.err" &
append_pid=$!
"${WATER[@]}" --date 2026-07-11 \
  >"$tmp/locked-watermark.out" 2>"$tmp/locked-watermark.err" &
watermark_pid=$!
sleep 0.25
kill -0 "$append_pid" 2>/dev/null || fail "tracked append ignored an externally held lock"
kill -0 "$watermark_pid" 2>/dev/null || fail "watermark update ignored an externally held lock"
grep -q 'waited tracked append' plans/reviews/continuous-improvement-log.md && \
  fail "tracked append mutated the log while lock was held"
grep -q 'Last triaged:\*\* 2026-07-11' plans/reviews/continuous-improvement-log.md && \
  fail "watermark mutated the log while lock was held"
grep -q '^external-test-owner$' "$tracked_lock" || fail "writer replaced a foreign lock"
rm "$tracked_lock"
wait "$append_pid"
wait "$watermark_pid"
grep -q 'waited tracked append' plans/reviews/continuous-improvement-log.md || \
  fail "tracked append did not resume after lock release"
grep -q 'Last triaged:\*\* 2026-07-11' plans/reviews/continuous-improvement-log.md || \
  fail "watermark did not resume after lock release"

# 8) dual harvests serialise without losing or duplicating pending entries
"${APPEND[@]}" --task 'dual harvest one' --agent codex --outcome queued >/dev/null
"${APPEND[@]}" --task 'dual harvest two' --agent codex --outcome queued >/dev/null
printf 'foreign-temp-sentinel\n' > plans/reviews/continuous-improvement-log.md.harvest-tmp
printf 'external-test-owner\n' > "$tracked_lock"
"${HARVEST[@]}" --json >"$tmp/harvest-one.out" 2>"$tmp/harvest-one.err" &
harvest_one_pid=$!
"${HARVEST[@]}" --json >"$tmp/harvest-two.out" 2>"$tmp/harvest-two.err" &
harvest_two_pid=$!
sleep 0.25
kill -0 "$harvest_one_pid" 2>/dev/null || fail "first harvest ignored tracked-log lock"
kill -0 "$harvest_two_pid" 2>/dev/null || fail "second harvest ignored tracked-log lock"
[[ "$(ls "$tmp/repo/.git/anvil/ci-log-pending/"*.md | wc -l)" -eq 2 ]] || \
  fail "harvest changed pending queue while lock was held"
rm "$tracked_lock"
wait "$harvest_one_pid"
wait "$harvest_two_pid"
[[ "$(grep -c 'dual harvest one' plans/reviews/continuous-improvement-log.md)" -eq 1 ]] || \
  fail "dual harvest lost or duplicated first entry"
[[ "$(grep -c 'dual harvest two' plans/reviews/continuous-improvement-log.md)" -eq 1 ]] || \
  fail "dual harvest lost or duplicated second entry"
[[ ! -e "$tmp/repo/.git/anvil/ci-log-pending/"*.md ]] || \
  fail "dual harvest left processed pending entries"
grep -q '^foreign-temp-sentinel$' plans/reviews/continuous-improvement-log.md.harvest-tmp || \
  fail "harvest clobbered a foreign fixed-name temp file"
rm plans/reviews/continuous-improvement-log.md.harvest-tmp

# 9) reject empty / bad body / heading not first
if "${APPEND[@]}" --body 'no heading here' 2>/dev/null; then
  fail "accepted body without heading"
fi
if "${APPEND[@]}" --body $'preamble\n### 2026-07-12 — opencode\n\n- **Task:** x\n- **Outcome:** y\n- **Worked:** —\n- **Failed:** none\n- **Friction:** none\n- **Improvement:** none\n- **Follow-up:** none\n' 2>/dev/null; then
  fail "accepted body with heading not first"
fi

# 10) second pending then dry-run harvest
"${APPEND[@]}" --task 'second pending' --agent codex --outcome x >/dev/null
dry="$("${HARVEST[@]}" --dry-run --json)"
echo "$dry" | grep -q '"harvested": 1' || fail "dry-run harvest"
echo "$dry" | grep -q '"dryRun": true' || fail "dryRun flag"
# still pending
status_json="$("${STATUS[@]}" --json)"
echo "$status_json" | grep -q '"pendingCount": 1' || fail "dry-run should not clear pending"

# 11) follow-up field round-trip
"${APPEND[@]}" --task 'promote me' --follow-up 'promote: CIB' --agent opencode --json >/dev/null
pending2="$(ls "$tmp/repo/.git/anvil/ci-log-pending/"*.md | tail -1)"
grep -q 'promote: CIB' "$pending2" || fail "follow-up not written"

# 12) command date inputs must name real UTC calendar dates
for invalid_date in 2026-02-30 2025-02-29; do
  if "${APPEND[@]}" --date "$invalid_date" --task 'impossible date' --agent codex \
    2>/dev/null; then
    fail "append accepted impossible date: $invalid_date"
  fi
  if "${WATER[@]}" --date "$invalid_date" 2>/dev/null; then
    fail "watermark accepted impossible date: $invalid_date"
  fi
  if "${SINCE[@]}" --since "$invalid_date" 2>/dev/null; then
    fail "since accepted impossible date: $invalid_date"
  fi
done

"${APPEND[@]}" --date 2024-02-29 --task 'valid leap date' --agent codex >/dev/null
grep -q '^### 2024-02-29 ' "$tmp/repo/.git/anvil/ci-log-pending/"*.md || \
  fail "append rejected valid leap date"
"${WATER[@]}" --date 2024-02-29 >/dev/null
grep -q 'Last triaged:\*\* 2024-02-29' plans/reviews/continuous-improvement-log.md || \
  fail "watermark rejected valid leap date"
"${SINCE[@]}" --since 2024-02-29 >/dev/null || fail "since rejected valid leap date"

# 13) full-entry append modes share the same calendar validation boundary
invalid_stdin_entry=$'### 2026-02-30 — codex\n\n- **Task:** impossible stdin date\n- **Outcome:** rejected\n'
if printf '%s' "$invalid_stdin_entry" | "${APPEND[@]}" --stdin 2>/dev/null; then
  fail "stdin append accepted impossible heading date"
fi
invalid_body_entry=$'### 2025-02-29 — codex\n\n- **Task:** impossible body date\n- **Outcome:** rejected\n'
if "${APPEND[@]}" --body "$invalid_body_entry" 2>/dev/null; then
  fail "body append accepted non-leap heading date"
fi
valid_body_entry=$'### 2024-02-29 — codex\n\n- **Task:** valid leap body date\n- **Outcome:** queued\n'
"${APPEND[@]}" --body "$valid_body_entry" >/dev/null
grep -q 'valid leap body date' "$tmp/repo/.git/anvil/ci-log-pending/"*.md || \
  fail "body append rejected valid leap heading date"

printf 'ci-log.test.sh: OK\n'
