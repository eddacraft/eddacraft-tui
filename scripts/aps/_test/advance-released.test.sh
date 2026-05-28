#!/usr/bin/env bash
# Fixture tests for scripts/aps/advance-released.mjs (#1715).
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
ADV=(node "$ROOT/scripts/aps/advance-released.mjs")

TAG="v9.9.9-test"
SHA="abcdef1234567890deadbeef"      # 8-char prefix: abcdef12
DATE="2026-05-29"
RELEASED="Released/Shipped via ${TAG} (abcdef12 · ${DATE})"

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

fail() {
  printf 'advance-released.test.sh: FAIL: %s\n' "$1" >&2
  exit 1
}

# Recreate a clean fixture tree. Module `widget` uses `###` item headings;
# module `mega` nests `#### ID:` items under `###` group headings (MLP2 shape).
setup() {
  rm -rf "$tmp/plans"
  mkdir -p "$tmp/plans/modules" "$tmp/plans/releases" "$tmp/plans/archive/modules"

  # Archived module: a historical record may reference items whose module has
  # since been archived. Already-released items there must SKIP (not MISS); a
  # Merged item there is an anomaly that must NOT rewrite the frozen archive.
  cat >"$tmp/plans/archive/modules/legacy.aps.md" <<'EOF'
# Legacy

| ID  | Owner | Status   | Progress |
| --- | ----- | -------- | -------- |
| LEG | —     | Complete | 2/2      |

## Tasks

### LEG-001: shipped long ago

- **Status:** Released/Shipped via v0.0.1-old (11111111 · 2026-01-01)

### LEG-002: anomalous merged-in-archive

- **Status:** Merged 2026-01-01 via PR #9
EOF

  cat >"$tmp/plans/modules/widget.aps.md" <<'EOF'
# Widget

| ID  | Owner | Status      | Progress |
| --- | ----- | ----------- | -------- |
| WID | —     | In Progress | 2/3      |

## Tasks

### WID-001: first

- **Status:** Merged 2026-01-01 via PR #1 — landed the thing.

### WID-002: second

- **Status:** Released/Shipped via v0.0.1-old (11111111 · 2026-01-01)

### WID-003: third

- **Status:** In Progress
EOF

  cat >"$tmp/plans/modules/mega.aps.md" <<'EOF'
# Mega

| ID  | Owner | Status      | Progress |
| --- | ----- | ----------- | -------- |
| MEG | —     | In Progress | 1/1      |

## Tasks

### A. First group

#### MEG-001: nested item

- **Status:** Merged 2026-02-02 via PR #2 — nested under a group heading.
EOF
}

# Write a release record with the given tab-separated `id` list as aps.items[].
write_record() {
  local path="$1"; shift
  {
    echo "# Release"
    echo ""
    echo '```json'
    printf '{ "lifecycleState": "published", "aps": { "items": ['
    local first=1
    for id in "$@"; do
      [ $first -eq 1 ] && first=0 || printf ','
      printf '{"id":"%s","module":"%s"}' "$id" "${id%%-*}"
    done
    printf '] } }\n'
    echo '```'
  } >"$path"
}

run() {
  "${ADV[@]}" --root="$tmp" --release-record="$tmp/plans/releases/r.md" \
    --tag="$TAG" --sha="$SHA" --date="$DATE" "$@"
}

# ── Scenario 1: happy path — advances ### and #### Merged items, skips an
#    already-released item; exit 0. ───────────────────────────────────────────
setup
write_record "$tmp/plans/releases/r.md" WID-001 WID-002 MEG-001
run >/dev/null 2>&1 || fail "happy path should exit 0"
grep -qF -- "- **Status:** ${RELEASED}" "$tmp/plans/modules/widget.aps.md" \
  || fail "WID-001 (###) not advanced"
grep -qF -- "- **Status:** ${RELEASED}" "$tmp/plans/modules/mega.aps.md" \
  || fail "MEG-001 (####) not advanced"
grep -qF -- "Released/Shipped via v0.0.1-old" "$tmp/plans/modules/widget.aps.md" \
  || fail "WID-002 already-released line must be preserved (not retagged)"
grep -qF -- "- **Status:** In Progress" "$tmp/plans/modules/widget.aps.md" \
  || fail "WID-003 (not in record) must be untouched"

# ── Scenario 2: idempotent re-run — all skipped, no change, exit 0. ──────────
before="$(cat "$tmp/plans/modules/widget.aps.md" "$tmp/plans/modules/mega.aps.md")"
run >/dev/null 2>&1 || fail "idempotent re-run should exit 0"
after="$(cat "$tmp/plans/modules/widget.aps.md" "$tmp/plans/modules/mega.aps.md")"
[ "$before" = "$after" ] || fail "idempotent re-run must not modify files"

# Assert `run` (plus any extra args) exits non-zero and its combined output
# contains a string — single invocation, no `run | grep` pipefail trap.
expect_fail_with() {
  local needle="$1"; shift
  local out rc=0
  out="$(run "$@" 2>&1)" || rc=$?
  [ "$rc" -ne 0 ] || fail "expected non-zero exit (looking for: $needle)"
  printf '%s\n' "$out" | grep -qF -- "$needle" || fail "expected output to contain: $needle"
}

# ── Scenario 3: missing heading — item not in any module → MISS → exit 1. ────
setup
write_record "$tmp/plans/releases/r.md" GHOST-001
expect_fail_with "MISS: GHOST-001"

# ── Scenario 4: stale items list — item present but not Merged → MISS → 1. ───
setup
write_record "$tmp/plans/releases/r.md" WID-003
expect_fail_with "MISS: WID-003"
grep -qF -- "- **Status:** In Progress" "$tmp/plans/modules/widget.aps.md" \
  || fail "non-Merged item must not be rewritten"

# ── Scenario 5: --dry-run — reports but writes nothing; exit 0. ──────────────
setup
write_record "$tmp/plans/releases/r.md" WID-001 MEG-001
run --dry-run >/dev/null 2>&1 || fail "dry-run should exit 0"
grep -qF -- "- **Status:** Merged 2026-01-01" "$tmp/plans/modules/widget.aps.md" \
  || fail "dry-run must not rewrite WID-001"
grep -qF -- "- **Status:** Merged 2026-02-02" "$tmp/plans/modules/mega.aps.md" \
  || fail "dry-run must not rewrite MEG-001"

# ── Scenario 6: archived module — already-released item SKIPs (not MISS);
#    a Merged item in archive is an anomaly → MISS, archive left untouched. ───
setup
write_record "$tmp/plans/releases/r.md" LEG-001
out="$(run 2>&1)" || fail "archived already-released item should SKIP (exit 0)"
printf '%s\n' "$out" | grep -qF -- "SKIP LEG-001" || fail "archived released item should report SKIP"

setup
write_record "$tmp/plans/releases/r.md" LEG-002
expect_fail_with "MISS: LEG-002"
grep -qF -- "- **Status:** Merged 2026-01-01 via PR #9" "$tmp/plans/archive/modules/legacy.aps.md" \
  || fail "frozen archive file must not be rewritten"

# ── Scenario 7: all-or-nothing — one MISS blocks the run, so an advanceable
#    item in the SAME run is NOT written (Council AR-001). ────────────────────
setup
write_record "$tmp/plans/releases/r.md" WID-001 GHOST-001
expect_fail_with "NOT WRITTEN"
grep -qF -- "- **Status:** Merged 2026-01-01 via PR #1" "$tmp/plans/modules/widget.aps.md" \
  || fail "all-or-nothing: WID-001 must NOT be written when a sibling item MISSes"

# ── Scenario 8: a relative --release-record resolves against --root, not the
#    caller's CWD (Copilot review on #2055). Invoked from an unrelated CWD with
#    a path that only exists under root, so a CWD-relative resolve would MISS. ─
setup
write_record "$tmp/plans/releases/r.md" WID-001
( cd "$tmp/plans" && "${ADV[@]}" --root="$tmp" \
    --release-record="plans/releases/r.md" \
    --tag="$TAG" --sha="$SHA" --date="$DATE" ) >/dev/null 2>&1 \
  || fail "relative --release-record under --root should resolve and exit 0"
grep -qF -- "- **Status:** ${RELEASED}" "$tmp/plans/modules/widget.aps.md" \
  || fail "relative --release-record must resolve under --root, not the caller CWD"

printf 'advance-released.test.sh: ok\n'
