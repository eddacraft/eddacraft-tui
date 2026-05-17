#!/usr/bin/env bash
# Editor coexistence harness — ADOPT-006.
#
# Runs each targets/<name>.sh runner against the matching fixture with
# `anvil watch --source` live on the same tree. Emits a JSON verdict on
# stdout describing pass / skip / fail per target. Exits non-zero when:
#   - any target failed, OR
#   - the count of present targets is below the threshold pinned in
#     required-targets.txt.
#
# Diagnostic logs go to stderr; do not parse them.
#
# Environment:
#   ANVIL_BIN     Path to an `anvil` binary. Required.
#   SETTLE_MS     Initial-scan settle window in ms (default: 1500).

set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SETTLE_MS="${SETTLE_MS:-1500}"
SCHEMA_VERSION=1

# Portable millisecond epoch. `date +%s%3N` is GNU-only; macOS / BSD
# truncate the format and emit literal `%3N`. Try GNU date, then gdate
# (Homebrew coreutils), then python3 as the universal fallback.
_epoch_ms() {
  local ms
  ms="$(date +%s%3N 2>/dev/null)"
  if [[ -n "${ms}" && "${ms}" != *N ]]; then
    printf '%s' "${ms}"
    return
  fi
  if command -v gdate >/dev/null 2>&1; then
    gdate +%s%3N
    return
  fi
  python3 -c 'import time; print(time.time_ns() // 1_000_000)'
}

if [[ -z "${ANVIL_BIN:-}" ]]; then
  echo "harness: ANVIL_BIN must be set to the anvil binary under test" >&2
  exit 2
fi
if [[ ! -x "${ANVIL_BIN}" ]]; then
  echo "harness: ANVIL_BIN=${ANVIL_BIN} is not executable" >&2
  exit 2
fi

# Global cleanup state so the EXIT/INT/TERM trap can reap leaks even when a
# per-target run aborts unexpectedly (set -e propagation, SIGINT from CI,
# etc).
declare -a _active_scratch=()
_active_anvil_pid=""

_cleanup() {
  if [[ -n "${_active_anvil_pid}" ]]; then
    kill -TERM "${_active_anvil_pid}" 2>/dev/null || true
    wait "${_active_anvil_pid}" 2>/dev/null || true
    _active_anvil_pid=""
  fi
  local dir
  for dir in "${_active_scratch[@]}"; do
    [[ -n "${dir}" && -d "${dir}" ]] && rm -rf "${dir}"
  done
  _active_scratch=()
}
trap _cleanup EXIT
trap 'rc=$?; _cleanup; exit "${rc}"' INT TERM

# Parse required-targets.txt — pinned floor.
threshold=0
declare -a required=()
while IFS= read -r line; do
  line="${line%%#*}"
  line="${line## }"
  line="${line%% }"
  [[ -z "${line}" ]] && continue
  if [[ "${line}" =~ ^threshold[[:space:]]*=[[:space:]]*([0-9]+)$ ]]; then
    threshold="${BASH_REMATCH[1]}"
    continue
  fi
  required+=("${line}")
done < "${HERE}/required-targets.txt"

declare -a results=()
present_count=0
failed=0

# Emit a JSON-safe string for the verdict's `notes` field. Escapes
# backslashes and double-quotes so an unusual scratch path or runner
# message can't break the composed JSON.
json_escape() {
  local s="${1-}"
  s="${s//\\/\\\\}"
  s="${s//\"/\\\"}"
  printf '%s' "${s}"
}

emit_result() {
  local name="$1" status="$2" duration="$3" events="$4" exit_code="$5" notes
  notes="$(json_escape "${6-}")"
  results+=("{\"name\":\"${name}\",\"status\":\"${status}\",\"duration_ms\":${duration},\"anvil_events\":${events},\"language_server_exit\":${exit_code},\"notes\":\"${notes}\"}")
}

run_target() {
  local name="$1"
  local runner="${HERE}/targets/${name}.sh"
  if [[ ! -x "${runner}" ]]; then
    # A required-targets.txt entry without a matching runner is a harness
    # integrity bug, not a runner skip. Record fail so CI surfaces it.
    emit_result "${name}" fail 0 0 -1 "no runner script at ${runner} (harness integrity)"
    failed=$((failed + 1))
    return
  fi

  local scratch
  scratch="$(mktemp -d)"
  _active_scratch+=("${scratch}")
  local anvil_log="${scratch}/anvil.log"
  local target_log="${scratch}/target.log"

  local fixture
  if ! fixture="$("${runner}" --print-fixture 2>/dev/null)"; then
    emit_result "${name}" skip 0 0 -1 "runner did not declare fixture"
    rm -rf "${scratch}"
    return
  fi
  fixture="${HERE}/${fixture}"
  if [[ ! -d "${fixture}" ]]; then
    emit_result "${name}" fail 0 0 -1 "fixture missing: ${fixture}"
    failed=$((failed + 1))
    rm -rf "${scratch}"
    return
  fi

  # Copy fixture to scratch so concurrent harness runs do not race, then
  # initialise a git repo so anvil treats it as a workspace. Failures here
  # are per-target — they must not abort the whole harness via set -e.
  if ! cp -a "${fixture}/." "${scratch}/repo/" 2>"${target_log}"; then
    emit_result "${name}" fail 0 0 -1 "fixture copy failed; see ${target_log}"
    failed=$((failed + 1))
    rm -rf "${scratch}"
    return
  fi
  if ! (
    cd "${scratch}/repo" \
      && git init -q \
      && git add -A \
      && git -c user.email=h@h -c user.name=h commit -q -m baseline
  ) 2>>"${target_log}"; then
    emit_result "${name}" fail 0 0 -1 "git init failed; see ${target_log}"
    failed=$((failed + 1))
    rm -rf "${scratch}"
    return
  fi

  local target_start target_end duration_ms target_exit
  target_start="$(_epoch_ms)"

  # Start anvil watch in background against the scratch repo.
  (
    cd "${scratch}/repo"
    ANVIL_DEV=1 "${ANVIL_BIN}" watch --source >"${anvil_log}" 2>&1
  ) &
  _active_anvil_pid=$!

  # Settle window for initial scan.
  sleep "$(awk "BEGIN { printf \"%.3f\", ${SETTLE_MS}/1000 }")"

  if ! kill -0 "${_active_anvil_pid}" 2>/dev/null; then
    emit_result "${name}" fail 0 0 -1 "anvil watch exited during settle; see ${anvil_log}"
    failed=$((failed + 1))
    _active_anvil_pid=""
    rm -rf "${scratch}"
    return
  fi

  present_count=$((present_count + 1))

  set +e
  ( cd "${scratch}/repo" && "${runner}" --run-against "${scratch}/repo" ) \
    >"${target_log}" 2>&1
  target_exit=$?
  set -e

  kill -TERM "${_active_anvil_pid}" 2>/dev/null || true
  wait "${_active_anvil_pid}" 2>/dev/null || true
  _active_anvil_pid=""

  target_end="$(_epoch_ms)"
  duration_ms=$((target_end - target_start))

  # Lock-contention / panic detection across both logs is the load-bearing
  # check. anvil watch's default output is human-readable prose, so we
  # rely on the language-server / formatter runner exiting cleanly and on
  # the absence of OS-level conflict markers — not on parsing watch event
  # counts, which would couple the harness to a moving format. When a
  # structured `anvil watch --json` mode lands, switch `anvil_events` to
  # a real count and tighten the contract.
  local status="pass" notes="" events=0
  if [[ "${target_exit}" -eq 200 ]]; then
    status="skip"
    notes="target unavailable on PATH"
    present_count=$((present_count - 1))
  elif [[ "${target_exit}" -ne 0 ]]; then
    status="fail"
    local tail_excerpt
    tail_excerpt="$(tail -c 600 "${target_log}" 2>/dev/null | tr '\n\r\t' '   ')"
    notes="target runner exited ${target_exit}; tail: ${tail_excerpt}"
    failed=$((failed + 1))
  elif grep -qiE 'EBUSY|EAGAIN|file lock|resource temporarily unavailable|panicked' \
       "${anvil_log}" "${target_log}"; then
    status="fail"
    notes="lock-contention or panic detected in logs"
    failed=$((failed + 1))
  fi

  # Stream the per-target logs on stderr for CI visibility — they don't fit
  # in the verdict JSON and the scratch dir is wiped immediately after.
  if [[ "${status}" == "fail" ]]; then
    {
      echo "=== ${name} anvil watch log ==="
      cat "${anvil_log}" 2>/dev/null || true
      echo "=== ${name} target log ==="
      cat "${target_log}" 2>/dev/null || true
      echo "=== end ${name} ==="
    } >&2
  fi

  emit_result "${name}" "${status}" "${duration_ms}" "${events}" "${target_exit}" "${notes}"

  rm -rf "${scratch}"
  # Remove this scratch entry from the global list (best effort — the
  # trap handles any survivors).
  local i
  for i in "${!_active_scratch[@]}"; do
    if [[ "${_active_scratch[$i]}" == "${scratch}" ]]; then
      unset '_active_scratch[i]'
    fi
  done
}

for name in "${required[@]}"; do
  run_target "${name}"
done

# Compose verdict.
joined="$(IFS=,; echo "${results[*]}")"
threshold_met=$([[ "${present_count}" -ge "${threshold}" ]] && echo true || echo false)

cat <<EOF
{
  "schema_version": ${SCHEMA_VERSION},
  "targets": [${joined}],
  "required_targets_present": ${present_count},
  "required_targets_threshold": ${threshold},
  "threshold_met": ${threshold_met}
}
EOF

if [[ "${failed}" -gt 0 ]]; then
  echo "harness: ${failed} target(s) failed" >&2
  exit 1
fi
if [[ "${threshold_met}" != "true" ]]; then
  echo "harness: only ${present_count}/${threshold} required targets present" >&2
  exit 1
fi
