#!/usr/bin/env bash
# DEVENV-004 (ADR-057): disk-pressure eviction of relocated Rust target dirs.
#
# DEVENV-002 relocates each worktree's `target/` to $ANVIL_TARGET_BASE/<slug> on
# /home. Those accumulate. This script reclaims the least-recently-used ones when
# the target filesystem crosses a high-water mark — without ever deleting a target
# a build is using, and without ever operating outside $ANVIL_TARGET_BASE.
#
# SAFETY (the PreToolUse hooks are no-ops, so this is self-enforcing):
#   - Every deletion target's realpath must be a direct child of the resolved
#     $ANVIL_TARGET_BASE realpath. Any mismatch => exit non-zero (fail closed).
#   - Skips a dir whose cargo build lock (`.cargo-lock`) is held (build running).
#   - Skips a dir touched within the freshness window.
#   - Dry-run by DEFAULT: prints what it WOULD evict. Pass --apply to delete.
#
# Usage:
#   anvil-target-evict.sh [--apply] [--high-water PCT] [--low-water PCT]
#                         [--freshness-mins N] [--json] [-h|--help]
set -euo pipefail

ANVIL_TARGET_BASE="${ANVIL_TARGET_BASE:-${HOME}/.cache/anvil-targets}"
apply=false
high_water=80
low_water=70
freshness_mins=30
json=false

die() {
  echo "anvil-target-evict: $*" >&2
  exit 1
}

# Print the comment header (lines 2-18) as help — stop before `set -euo pipefail`.
usage() {
  sed -n '2,18p' "$0" | sed 's/^# \{0,1\}//'
  exit "${1:-0}"
}

# Emit array elements as a JSON list body: "a","b" (nothing when empty, so the
# caller's [ ... ] is a valid empty array, not [""]).
json_list() {
  local out="" e
  for e in "$@"; do out+="\"${e}\","; done
  printf '%s' "${out%,}"
}

while (($#)); do
  case "$1" in
    --apply) apply=true; shift ;;
    --high-water) high_water="${2:?--high-water needs a value}"; shift 2 ;;
    --low-water) low_water="${2:?--low-water needs a value}"; shift 2 ;;
    --freshness-mins) freshness_mins="${2:?--freshness-mins needs a value}"; shift 2 ;;
    --json) json=true; shift ;;
    -h | --help) usage 0 ;;
    *) die "unknown argument: $1 (try --help)" ;;
  esac
done

[[ "$high_water" =~ ^[0-9]+$ && "$low_water" =~ ^[0-9]+$ ]] ||
  die "high/low water must be integer percentages"
((low_water < high_water)) || die "--low-water ($low_water) must be < --high-water ($high_water)"

# Resolve the base. If it does not exist yet, there is nothing to evict.
if [[ ! -d "$ANVIL_TARGET_BASE" ]]; then
  $json && echo '{"evicted":[],"skipped":[],"reason":"base-missing"}' ||
    echo "anvil-target-evict: base $ANVIL_TARGET_BASE does not exist; nothing to do"
  exit 0
fi
base_real="$(realpath "$ANVIL_TARGET_BASE")"
# Refuse to ever treat the home dir or a root-ish path as the base.
case "$base_real" in
  "$HOME" | / | "") die "refusing to operate on unsafe base: $base_real" ;;
esac

# Current used% of the filesystem holding the base (integer).
used_pct() {
  df -P "$base_real" | awk 'NR==2 { gsub(/%/,"",$5); print $5 }'
}

cur="$(used_pct)"
if ((cur < high_water)); then
  $json && printf '{"evicted":[],"skipped":[],"used_pct":%s,"reason":"below-high-water"}\n' "$cur" ||
    echo "anvil-target-evict: ${cur}% < high-water ${high_water}%; no pressure, nothing to do"
  exit 0
fi

# A dir is "busy" if its cargo lock is held (non-blocking flock fails) or it was
# touched within the freshness window. cargo holds `.cargo-lock` for the whole
# build/check/test/clippy, so this covers any in-flight cargo invocation
# (DEVENV-003's plugin sentinel is unnecessary).
is_busy() {
  local dir="$1" lock="$1/.cargo-lock"
  if [[ -e "$lock" ]] && ! flock -n "$lock" true 2>/dev/null; then
    return 0 # lock held => busy
  fi
  # Newest mtime under the dir, in epoch seconds.
  local newest
  newest="$(find "$dir" -type f -printf '%T@\n' 2>/dev/null | sort -rn | head -1 | cut -d. -f1)"
  [[ -n "$newest" ]] || return 1
  local age=$(( $(date +%s) - newest ))
  ((age < freshness_mins * 60))
}

# Assert a candidate is a direct child of the base (no symlink escape, no
# mis-set env). Fail closed.
assert_under_base() {
  local cand_real
  cand_real="$(realpath "$1" 2>/dev/null)" || die "cannot resolve $1"
  [[ "$cand_real" == "$base_real/"* ]] || die "REFUSING: $cand_real is not under $base_real"
  [[ "$(dirname "$cand_real")" == "$base_real" ]] || die "REFUSING: $cand_real is not a direct child of $base_real"
}

evicted=()
skipped=()

# LRU: oldest directory mtime first.
while IFS= read -r -d '' dir; do
  ((cur >= low_water)) || break
  if is_busy "$dir"; then
    skipped+=("$dir")
    continue
  fi
  assert_under_base "$dir"
  if $apply; then
    rm -rf -- "$dir"
    evicted+=("$dir")
    cur="$(used_pct)"
  else
    evicted+=("$dir") # would-evict (dry-run)
  fi
done < <(find "$base_real" -mindepth 1 -maxdepth 1 -type d -printf '%T@\t%p\0' |
  sort -zn | cut -z -f2-)

if $json; then
  printf '{"used_pct_start":%s,"applied":%s,"evicted":[' "$(used_pct)" "$apply"
  printf '%s' "$(json_list ${evicted[@]+"${evicted[@]}"})"
  printf '],"skipped":['
  printf '%s' "$(json_list ${skipped[@]+"${skipped[@]}"})"
  printf ']}\n'
else
  mode=$($apply && echo "evicted" || echo "WOULD evict (dry-run; pass --apply)")
  echo "anvil-target-evict: used ${cur}% (high ${high_water}/low ${low_water})"
  for d in "${evicted[@]:-}"; do [[ -n "$d" ]] && echo "  ${mode}: $d"; done
  for d in "${skipped[@]:-}"; do [[ -n "$d" ]] && echo "  skipped (busy): $d"; done
fi

# The trailing loops can leave $? non-zero (empty array → `[[ -n "" ]]` is
# false) even on full success; end deterministically so callers/`set -e` and the
# systemd unit see success.
exit 0
