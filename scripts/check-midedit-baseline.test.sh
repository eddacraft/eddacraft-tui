#!/usr/bin/env bash
# Fixture test for scripts/check-midedit-baseline.sh (RTAI-003 / ADR-031 gate).
#
# Verifies CI hard-fail aligns with ADR-031's primary boundary:
#   * validation.roundtrip over SLO → exit 1
#   * validation.service over SLO with roundtrip under SLO → exit 0 (WARN)
#   * drift-only / clean paths → exit 0
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
SCRIPT="$ROOT/scripts/check-midedit-baseline.sh"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

fail() {
	printf 'check-midedit-baseline.test.sh: FAIL: %s\n' "$1" >&2
	if [[ -f "$tmp/out" || -f "$tmp/err" ]]; then
		printf -- '--- stdout ---\n%s\n--- stderr ---\n%s\n' \
			"$(cat "$tmp/out" 2>/dev/null || true)" \
			"$(cat "$tmp/err" 2>/dev/null || true)" >&2
	fi
	exit 1
}

# Minimal baseline: two boundaries, one case each, ADR-031 interactive-buffer SLOs.
cat >"$tmp/baseline.json" <<'JSON'
{
  "schema_version": 1,
  "tolerance": { "drift_pct": 15, "zero_baseline_floor_ms": 1.0 },
  "slos": {
    "validation.service": { "p95_ms": 50.0 },
    "validation.roundtrip": { "p95_ms": 80.0 }
  },
  "cases": {
    "validation.service": {
      "near_cap_1MiB_minus_1KiB": { "p50_ms": 23.0, "p95_ms": 24.0, "p99_ms": 25.0 }
    },
    "validation.roundtrip": {
      "near_cap_1MiB_minus_1KiB": { "p50_ms": 31.0, "p95_ms": 36.0, "p99_ms": 44.0 }
    }
  }
}
JSON

write_log() {
	# $1 service p95, $2 roundtrip p95
	cat >"$tmp/bench.log" <<LOG
--- ADR-031 mid-edit warm percentile sampler ---
dimensions: mode=midEdit boundary=validation.service case=near_cap
validation.service near_cap_1MiB_minus_1KiB: samples=10 p50=23.0ms p95=${1}ms p99=26.0ms
dimensions: mode=midEdit boundary=validation.roundtrip case=near_cap
validation.roundtrip near_cap_1MiB_minus_1KiB: samples=10 p50=31.0ms p95=${2}ms p99=45.0ms
--- end ADR-031 sampler ---
LOG
}

run_gate() {
	set +e
	bash "$SCRIPT" "$tmp/bench.log" "$tmp/baseline.json" >"$tmp/out" 2>"$tmp/err"
	code=$?
	set -e
}

# 1. Clean: both under SLO and near baseline → exit 0, no WARN/FAIL.
write_log 24.0 36.0
run_gate
[[ "$code" -eq 0 ]] || fail "clean path should exit 0, got $code"
grep -q 'FAIL' "$tmp/out" && fail "clean path should not print FAIL"
grep -qE '^WARN' "$tmp/out" && fail "clean path should not print WARN"

# 2. Service over SLO (~55 vs 50), roundtrip still under 80 → exit 0 + WARN attribution.
write_log 55.0 70.0
run_gate
[[ "$code" -eq 0 ]] || fail "service-over-SLO-only should exit 0 (soft-warn), got $code"
grep -qE '^WARN validation\.service' "$tmp/out" || fail "expected WARN on validation.service"
grep -q 'attribution only' "$tmp/out" || fail "expected attribution-only note for service SLO"
grep -qE '^FAIL validation\.roundtrip' "$tmp/out" && fail "roundtrip under SLO must not FAIL"

# 3. Roundtrip over SLO → exit 1 hard-fail regardless of service.
write_log 40.0 90.0
run_gate
[[ "$code" -eq 1 ]] || fail "roundtrip over SLO should exit 1, got $code"
grep -qE '^FAIL validation\.roundtrip' "$tmp/out" || fail "expected FAIL on validation.roundtrip"

# 4. Both over SLO → exit 1 (roundtrip hard-fail); service still WARN not FAIL.
write_log 60.0 95.0
run_gate
[[ "$code" -eq 1 ]] || fail "both over SLO should exit 1, got $code"
grep -qE '^FAIL validation\.roundtrip' "$tmp/out" || fail "expected FAIL on validation.roundtrip when both breach"
grep -qE '^WARN validation\.service' "$tmp/out" || fail "service SLO breach must remain WARN when both breach"
grep -qE '^FAIL validation\.service' "$tmp/out" && fail "validation.service must never hard-FAIL on SLO"

# 5. Drift-only soft-warn (service 28 vs baseline 24 = +16.7% > 15%) → exit 0.
write_log 28.0 36.0
run_gate
[[ "$code" -eq 0 ]] || fail "drift-only should exit 0, got $code"
grep -qE '^WARN validation\.service' "$tmp/out" || fail "expected drift WARN on validation.service"


# 6. Unknown future boundary over SLO → hard-fail (soft-warn is explicit for
#    validation.service only; do not silently downgrade new boundaries).
cat >"$tmp/baseline.json" <<'JSON'
{
  "schema_version": 1,
  "tolerance": { "drift_pct": 15, "zero_baseline_floor_ms": 1.0 },
  "slos": {
    "validation.service": { "p95_ms": 50.0 },
    "validation.roundtrip": { "p95_ms": 80.0 },
    "validation.future": { "p95_ms": 40.0 }
  },
  "cases": {
    "validation.service": {
      "near_cap_1MiB_minus_1KiB": { "p50_ms": 23.0, "p95_ms": 24.0, "p99_ms": 25.0 }
    },
    "validation.roundtrip": {
      "near_cap_1MiB_minus_1KiB": { "p50_ms": 31.0, "p95_ms": 36.0, "p99_ms": 44.0 }
    },
    "validation.future": {
      "near_cap_1MiB_minus_1KiB": { "p50_ms": 10.0, "p95_ms": 12.0, "p99_ms": 14.0 }
    }
  }
}
JSON
cat >"$tmp/bench.log" <<'LOG'
--- ADR-031 mid-edit warm percentile sampler ---
dimensions: mode=midEdit boundary=validation.service case=near_cap
validation.service near_cap_1MiB_minus_1KiB: samples=10 p50=23.0ms p95=24.0ms p99=26.0ms
dimensions: mode=midEdit boundary=validation.roundtrip case=near_cap
validation.roundtrip near_cap_1MiB_minus_1KiB: samples=10 p50=31.0ms p95=36.0ms p99=45.0ms
dimensions: mode=midEdit boundary=validation.future case=near_cap
validation.future near_cap_1MiB_minus_1KiB: samples=10 p50=10.0ms p95=55.0ms p99=60.0ms
--- end ADR-031 sampler ---
LOG
run_gate
[[ "$code" -eq 1 ]] || fail "unknown-boundary-over-SLO should exit 1, got $code"
grep -qE '^FAIL validation\.future' "$tmp/out" || fail "expected FAIL on validation.future"

# 7. Invalid baseline JSON is an input error, not jq's implementation-specific
#    parse exit code.
write_log 24.0 36.0
printf '{ invalid json\n' >"$tmp/baseline.json"
run_gate
[[ "$code" -eq 2 ]] || fail "malformed baseline JSON should exit 2, got $code"
grep -q 'error: malformed baseline JSON' "$tmp/err" || fail "expected malformed JSON input error"

# 8. The primary roundtrip boundary must declare an SLO.
cat >"$tmp/baseline.json" <<'JSON'
{
  "schema_version": 1,
  "tolerance": { "drift_pct": 15 },
  "slos": {
    "validation.service": { "p95_ms": 50.0 }
  },
  "cases": {
    "validation.roundtrip": {
      "near_cap_1MiB_minus_1KiB": { "p95_ms": 36.0 }
    }
  }
}
JSON
run_gate
[[ "$code" -eq 2 ]] || fail "missing required roundtrip SLO should exit 2, got $code"
grep -q 'error: baseline missing required validation.roundtrip p95 SLO' "$tmp/err" || fail "expected required roundtrip SLO input error"

# 9. The primary roundtrip boundary must define at least one required case.
cat >"$tmp/baseline.json" <<'JSON'
{
  "schema_version": 1,
  "tolerance": { "drift_pct": 15 },
  "slos": {
    "validation.roundtrip": { "p95_ms": 80.0 }
  },
  "cases": {
    "validation.roundtrip": {}
  }
}
JSON
run_gate
[[ "$code" -eq 2 ]] || fail "missing required roundtrip cases should exit 2, got $code"
grep -q 'error: baseline missing required validation.roundtrip cases' "$tmp/err" || fail "expected required roundtrip cases input error"

# 10. A sampler row with a malformed p95 is an input error, not an omitted
#     optional measurement.
cat >"$tmp/baseline.json" <<'JSON'
{
  "schema_version": 1,
  "tolerance": { "drift_pct": 15 },
  "slos": {
    "validation.service": { "p95_ms": 50.0 },
    "validation.roundtrip": { "p95_ms": 80.0 }
  },
  "cases": {
    "validation.service": {
      "near_cap_1MiB_minus_1KiB": { "p95_ms": 24.0 }
    },
    "validation.roundtrip": {
      "near_cap_1MiB_minus_1KiB": { "p95_ms": 36.0 }
    }
  }
}
JSON
cat >"$tmp/bench.log" <<'LOG'
--- ADR-031 mid-edit warm percentile sampler ---
dimensions: mode=midEdit boundary=validation.service case=near_cap
validation.service near_cap_1MiB_minus_1KiB: samples=10 p50=23.0ms p95=24.0ms p99=26.0ms
dimensions: mode=midEdit boundary=validation.roundtrip case=near_cap
validation.roundtrip near_cap_1MiB_minus_1KiB: samples=10 p50=31.0ms p95=not-a-number p99=45.0ms
--- end ADR-031 sampler ---
LOG
run_gate
[[ "$code" -eq 2 ]] || fail "malformed percentile row should exit 2, got $code"
grep -q 'error: malformed percentile row' "$tmp/err" || fail "expected malformed percentile input error"

# 11. Roundtrip cases in the baseline contract are mandatory gate input. A
#     service-only sampler must fail closed.
cat >"$tmp/bench.log" <<'LOG'
--- ADR-031 mid-edit warm percentile sampler ---
dimensions: mode=midEdit boundary=validation.service case=near_cap
validation.service near_cap_1MiB_minus_1KiB: samples=10 p50=23.0ms p95=24.0ms p99=26.0ms
--- end ADR-031 sampler ---
LOG
run_gate
[[ "$code" -eq 2 ]] || fail "service-only sampler should exit 2, got $code"
grep -q 'error: missing required benchmark row: validation.roundtrip near_cap_1MiB_minus_1KiB' "$tmp/err" || fail "expected missing required roundtrip row input error"

# 12. Service rows remain advisory. A roundtrip-complete sampler may omit the
#     baseline's validation.service row without making the gate fail.
cat >"$tmp/bench.log" <<'LOG'
--- ADR-031 mid-edit warm percentile sampler ---
dimensions: mode=midEdit boundary=validation.roundtrip case=near_cap
validation.roundtrip near_cap_1MiB_minus_1KiB: samples=10 p50=31.0ms p95=36.0ms p99=45.0ms
--- end ADR-031 sampler ---
LOG
run_gate
[[ "$code" -eq 0 ]] || fail "missing advisory service row should exit 0, got $code"
grep -q 'validation.service near_cap_1MiB_minus_1KiB' "$tmp/err" || fail "expected advisory missing-service warning"

# 13. Percentiles are strict decimals, not arbitrary runs of dots.
cat >"$tmp/bench.log" <<'LOG'
--- ADR-031 mid-edit warm percentile sampler ---
dimensions: mode=midEdit boundary=validation.roundtrip case=near_cap
validation.roundtrip near_cap_1MiB_minus_1KiB: samples=10 p50=31.0ms p95=3..6ms p99=45.0ms
--- end ADR-031 sampler ---
LOG
run_gate
[[ "$code" -eq 2 ]] || fail "malformed decimal should exit 2, got $code"
grep -q 'error: malformed percentile row' "$tmp/err" || fail "expected malformed-decimal input error"

# 14. A valid prefix with trailing content is not a complete percentile row.
cat >"$tmp/bench.log" <<'LOG'
--- ADR-031 mid-edit warm percentile sampler ---
dimensions: mode=midEdit boundary=validation.roundtrip case=near_cap
validation.roundtrip near_cap_1MiB_minus_1KiB: samples=10 p50=31.0ms p95=36.0ms p99=45.0ms trailing
--- end ADR-031 sampler ---
LOG
run_gate
[[ "$code" -eq 2 ]] || fail "percentile row with trailing content should exit 2, got $code"
grep -q 'error: malformed percentile row' "$tmp/err" || fail "expected trailing-content input error"

# 15. Raw note text that mentions a required key cannot satisfy coverage.
cat >"$tmp/bench.log" <<'LOG'
--- ADR-031 mid-edit warm percentile sampler ---
dimensions: mode=midEdit boundary=validation.service case=near_cap
validation.service near_cap_1MiB_minus_1KiB: samples=10 p50=23.0ms p95=24.0ms p99=26.0ms
note: validation.roundtrip near_cap_1MiB_minus_1KiB: intentionally not a measurement row
--- end ADR-031 sampler ---
LOG
run_gate
[[ "$code" -eq 2 ]] || fail "spoofed raw text should not satisfy required coverage, got $code"
grep -q 'error: missing required benchmark row: validation.roundtrip near_cap_1MiB_minus_1KiB' "$tmp/err" || fail "expected spoof-resistant missing-row error"

# 16. A syntactically valid percentile row must contain at least one sample.
cat >"$tmp/bench.log" <<'LOG'
--- ADR-031 mid-edit warm percentile sampler ---
dimensions: mode=midEdit boundary=validation.service case=near_cap
validation.service near_cap_1MiB_minus_1KiB: samples=10 p50=23.0ms p95=24.0ms p99=26.0ms
dimensions: mode=midEdit boundary=validation.roundtrip case=near_cap
validation.roundtrip near_cap_1MiB_minus_1KiB: samples=0 p50=31.0ms p95=36.0ms p99=45.0ms
--- end ADR-031 sampler ---
LOG
run_gate
[[ "$code" -eq 2 ]] || fail "zero-sample percentile row should exit 2, got $code"
grep -q 'error: percentile row samples must be at least 1' "$tmp/err" || fail "expected zero-sample input error"

# 17. Raw note text cannot hide an optional/advisory orphaned baseline row.
cat >"$tmp/bench.log" <<'LOG'
--- ADR-031 mid-edit warm percentile sampler ---
dimensions: mode=midEdit boundary=validation.roundtrip case=near_cap
validation.roundtrip near_cap_1MiB_minus_1KiB: samples=10 p50=31.0ms p95=36.0ms p99=45.0ms
note: validation.service near_cap_1MiB_minus_1KiB: intentionally omitted measurement
--- end ADR-031 sampler ---
LOG
run_gate
[[ "$code" -eq 0 ]] || fail "missing advisory row with spoofing note should exit 0, got $code"
grep -q 'warning: baseline contains entries with no matching bench output' "$tmp/err" || fail "expected advisory orphan warning"
grep -q 'validation.service near_cap_1MiB_minus_1KiB' "$tmp/err" || fail "expected missing advisory row in orphan warning"

printf 'check-midedit-baseline.test.sh: ok\n'
