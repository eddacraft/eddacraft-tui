#!/usr/bin/env bash
# RTAI-003 CI baseline-comparison gate for `midedit_roundtrip`.
#
# Parses the percentile sampler block emitted by
# `cargo bench -p eddacraft-anvil-intercept --bench midedit_roundtrip
# --features bench-internals` and compares each per-case p95 against the
# baseline JSON committed at
# `crates/anvil-intercept/benches/baselines/midedit_roundtrip.json`.
#
# Policy (per ADR-031 § Regression policy and the bench file's calibration
# header):
#
#   * Hard fail on any p95 exceeding the ADR-031 SLO for its boundary
#     (validation.service ≤ 50 ms, validation.roundtrip ≤ 80 ms for the
#     interactive-buffer budget class).
#   * Soft warn on baseline drift outside ±drift_pct (p95 only — p50/p99 are
#     reported for context but not gated, matching ADR-031 which makes
#     p95 the pass/fail SLO). When a baseline p95 is 0 (sub-resolution
#     case) the drift gate switches to an absolute floor of
#     `tolerance.zero_baseline_floor_ms` so silent regressions past the
#     floor still surface.
#   * Surface a warning when the baseline JSON contains entries with no
#     matching bench output (orphaned baseline rows after a case is
#     removed from the bench).
#   * Always print the ADR-031 dimension line preceding each FAIL/WARN
#     row so an on-call reader can attribute a regression without
#     digging through the workflow log.
#
# Usage:
#   scripts/check-midedit-baseline.sh <bench-output-log> <baseline.json>
#
# Exit codes:
#   0  all p95s within tolerance and SLO
#   1  at least one p95 exceeded the ADR-031 SLO (hard fail)
#   2  malformed input (bench log missing the percentile sampler block,
#      baseline JSON missing a referenced (boundary, case) pair, etc.)

set -euo pipefail

if [[ $# -ne 2 ]]; then
  echo "usage: $0 <bench-output-log> <baseline.json>" >&2
  exit 2
fi

bench_log="$1"
baseline="$2"

if [[ ! -f "$bench_log" ]]; then
  echo "error: bench log not found: $bench_log" >&2
  exit 2
fi
if [[ ! -f "$baseline" ]]; then
  echo "error: baseline JSON not found: $baseline" >&2
  exit 2
fi
if ! command -v jq >/dev/null 2>&1; then
  echo "error: jq is required" >&2
  exit 2
fi

drift_pct=$(jq -r '.tolerance.drift_pct' "$baseline")
if [[ -z "$drift_pct" || "$drift_pct" == "null" ]]; then
  echo "error: baseline missing .tolerance.drift_pct" >&2
  exit 2
fi

# Absolute floor used in place of a percentage drift when baseline_p95 is 0
# (or sub-millisecond). Without this, a baseline_p95 of 0 ms makes any
# observed p95 a no-op for the drift gate even though the SLO check still
# runs — but the SLO gap (50 ms / 80 ms) is wide enough that a real
# regression on the binary short-circuit case (expected sub-microsecond)
# would slip through silently. Defaulting to 1 ms keeps that gate alive.
zero_baseline_floor_ms=$(jq -r '.tolerance.zero_baseline_floor_ms // 1.0' "$baseline")

# Extract the percentile sampler block. Its bracketing markers are emitted by
# `bench_percentile_sampler` in midedit_roundtrip.rs; if either marker is
# missing the bench did not run to completion and we should fail loud rather
# than silently passing.
sampler=$(awk '
  /^--- ADR-031 mid-edit warm percentile sampler ---/ { capture=1; next }
  /^--- end ADR-031 sampler ---/ { capture=0; next }
  capture { print }
' "$bench_log")

if [[ -z "$sampler" ]]; then
  echo "error: no ADR-031 percentile sampler block found in $bench_log" >&2
  echo "       (bench probably did not run to completion)" >&2
  exit 2
fi

declare -i hard_fail=0
declare -i soft_warn=0
declare -i rows=0
current_dim=""

echo "midedit baseline gate (drift tolerance: ±${drift_pct}%)"
echo "baseline: $baseline"
echo

# Pair each `dimensions:` line with the `<boundary> <case>: ...` row that
# follows it. ADR-031 requires both for each measurement; the percentile
# sampler emits them as adjacent lines.
while IFS= read -r line; do
  if [[ "$line" == dimensions:* ]]; then
    current_dim="$line"
    continue
  fi
  if [[ "$line" =~ ^(validation\.[a-z]+)[[:space:]]+([A-Za-z0-9_]+):[[:space:]]+samples=[0-9]+[[:space:]]+p50=([0-9.]+)ms[[:space:]]+p95=([0-9.]+)ms[[:space:]]+p99=([0-9.]+)ms ]]; then
    rows+=1
    boundary="${BASH_REMATCH[1]}"
    case_label="${BASH_REMATCH[2]}"
    p50_now="${BASH_REMATCH[3]}"
    p95_now="${BASH_REMATCH[4]}"
    p99_now="${BASH_REMATCH[5]}"

    base_p95=$(jq -r --arg b "$boundary" --arg c "$case_label" \
      '.cases[$b][$c].p95_ms // empty' "$baseline")
    slo_p95=$(jq -r --arg b "$boundary" '.slos[$b].p95_ms // empty' "$baseline")

    if [[ -z "$base_p95" ]]; then
      echo "FAIL  $boundary $case_label: no baseline entry" >&2
      echo "       $current_dim" >&2
      echo "       hint: a new bench case has been added without a baseline." >&2
      echo "       Re-record the baseline (see midedit_roundtrip.rs § Re-baselining)" >&2
      echo "       and commit the updated baselines/midedit_roundtrip.json." >&2
      hard_fail+=1
      continue
    fi
    if [[ -z "$slo_p95" ]]; then
      echo "FAIL  $boundary: no SLO entry in baseline" >&2
      hard_fail+=1
      continue
    fi

    # Drift = (now - baseline) / baseline * 100. The drift gate is symmetric
    # (|drift| > drift_pct triggers a warn) so a sudden p95 *improvement* also
    # surfaces — usually a sign that the bench environment changed or the
    # baseline is stale and should be re-recorded. When baseline_p95 is 0
    # (sub-resolution case, e.g. binary_short_circuit on validation.service)
    # we fall back to a fixed absolute floor so a regression past the floor
    # still trips the drift warn even though no percentage is meaningful.
    read -r drift over_drift drift_mode < <(awk \
        -v now="$p95_now" -v base="$base_p95" \
        -v floor="$zero_baseline_floor_ms" -v t="$drift_pct" '
      BEGIN {
        n = now + 0
        b = base + 0
        if (b == 0) {
          # Absolute mode: warn iff observed exceeds floor.
          drift = 0.0
          warn = (n > floor + 0) ? 1 : 0
          mode = "abs"
        } else {
          drift = (n - b) / b * 100
          abs_drift = (drift < 0) ? -drift : drift
          warn = (abs_drift > t + 0) ? 1 : 0
          mode = "pct"
        }
        printf "%.1f %d %s\n", drift, warn, mode
      }')

    over_slo=$(awk -v now="$p95_now" -v slo="$slo_p95" \
      'BEGIN { print (now+0 > slo+0) ? "1" : "0" }')

    status="OK"
    if (( over_slo )); then
      status="FAIL"
      hard_fail+=1
    elif (( over_drift )); then
      status="WARN"
      soft_warn+=1
    fi

    if [[ "$drift_mode" == "abs" ]]; then
      drift_display="abs(floor=${zero_baseline_floor_ms}ms)"
    else
      drift_display="${drift}%"
    fi

    printf '%-4s %s %s: p50=%sms p95=%sms p99=%sms  baseline_p95=%sms drift=%s  slo_p95=%sms\n' \
      "$status" "$boundary" "$case_label" "$p50_now" "$p95_now" "$p99_now" \
      "$base_p95" "$drift_display" "$slo_p95"
    if [[ "$status" != "OK" ]]; then
      printf '     %s\n' "$current_dim"
    fi
  fi
done <<< "$sampler"

# Coverage check: warn if the baseline JSON contains entries that the bench
# did not produce. Catches the case where a bench case is removed but the
# baseline isn't pruned — without this the orphan rows are silently ignored.
orphans=$(jq -r '
  .cases | to_entries[] as $b
  | $b.value | keys[] as $c
  | "\($b.key) \($c)"
' "$baseline" | while read -r boundary case_label; do
  if ! grep -qF "${boundary} ${case_label}:" <<< "$sampler"; then
    echo "$boundary $case_label"
  fi
done)
if [[ -n "$orphans" ]]; then
  echo
  echo "warning: baseline contains entries with no matching bench output:" >&2
  echo "$orphans" | sed 's/^/  - /' >&2
  echo "       (prune them from baselines/midedit_roundtrip.json or restore the bench case)" >&2
fi

if [[ $rows -eq 0 ]]; then
  echo "error: percentile sampler block contained no measurement rows" >&2
  exit 2
fi

echo
echo "summary: $rows rows, $hard_fail hard-fail (SLO breach), $soft_warn soft-warn (drift > ±${drift_pct}% or above ${zero_baseline_floor_ms}ms floor)"

if (( hard_fail > 0 )); then
  exit 1
fi
exit 0
