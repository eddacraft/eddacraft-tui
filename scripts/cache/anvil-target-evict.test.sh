#!/usr/bin/env bash
# Tests for anvil-target-evict.sh (DEVENV-004). Uses a temp base so no real
# target dirs are touched. Forces the high-water gate via --high-water 1
# (real used% is always >= 1) / --low-water 0 (never satisfied => considers
# every non-busy dir), or --high-water 100 to assert the below-threshold no-op.
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
script="$here/anvil-target-evict.sh"
fails=0
# @anvil-ignore SURFSH-002 -- test harness evaluates assertion snippets
check() { if eval "$2"; then echo "ok: $1"; else echo "FAIL: $1"; fails=$((fails + 1)); fi; }

base="$(mktemp -d)"
trap 'rm -rf "$base"' EXIT
export ANVIL_TARGET_BASE="$base"

mkslug() { # name, age-seconds-ago
  local d="$base/$1"
  mkdir -p "$d"
  : >"$d/artifact"
  touch -d "@$(($(date +%s) - ${2:-0}))" "$d/artifact" "$d"
}

# --- T1: dry-run (default) never deletes ---
mkslug old1 99999
"$script" --high-water 1 --low-water 0 >/dev/null 2>&1
check "dry-run leaves dirs intact" '[[ -d "$base/old1" ]]'

# --- T2: --apply evicts a stale, non-busy dir ---
"$script" --apply --high-water 1 --low-water 0 >/dev/null 2>&1
check "--apply evicts a stale dir" '[[ ! -d "$base/old1" ]]'

# --- T3: a held .cargo-lock makes a dir busy (skipped) ---
mkslug busy1 99999
exec 9>"$base/busy1/.cargo-lock"
flock -n 9 # hold the lock for the duration of this test
"$script" --apply --high-water 1 --low-water 0 >/dev/null 2>&1
check "lock-held dir is skipped" '[[ -d "$base/busy1" ]]'
exec 9>&-

# --- T4 + T5: a freshly-touched dir is skipped; a stale one is evicted ---
mkslug fresh1 0
mkslug stale1 99999
"$script" --apply --high-water 1 --low-water 0 --freshness-mins 30 >/dev/null 2>&1
check "T4 fresh dir skipped" '[[ -d "$base/fresh1" ]]'
check "T5 stale dir evicted" '[[ ! -d "$base/stale1" ]]'

# --- T6: below high-water => no-op ---
mkslug keep1 99999
"$script" --apply --high-water 100 --low-water 0 >/dev/null 2>&1
check "T6 below high-water is a no-op" '[[ -d "$base/keep1" ]]'

# --- T7: unsafe base ($HOME) fails closed ---
check "T7 refuses unsafe base (\$HOME)" '! ANVIL_TARGET_BASE="$HOME" "$script" --apply --high-water 1 --low-water 0 >/dev/null 2>&1'

# --- T8: low >= high is rejected ---
check "T8 rejects low-water >= high-water" '! "$script" --high-water 50 --low-water 50 >/dev/null 2>&1'

if ((fails > 0)); then
  echo "anvil-target-evict.test: $fails failure(s)" >&2
  exit 1
fi
echo "anvil-target-evict.test: all passed"
