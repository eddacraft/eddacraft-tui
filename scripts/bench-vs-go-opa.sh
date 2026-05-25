#!/usr/bin/env bash
#
# POLENG-008 parity gate: regorus vs the Go OPA reference.
#
# ADR-040 D-1 adopts regorus on the assumption it is at least as fast as Go OPA
# on Anvil's representative policy mix. This script measures both engines on the
# same fixtures (eval-only scope — prepared query, input bound once) and gates:
# regorus median must be at or above OPA parity on every policy. A failure is an
# ADR-040 D-5 trigger (revisit the engine choice), not a routine error.
#
#   regorus : crates/anvil-policy-engine/examples/parity_harness.rs (eval_rule)
#   Go OPA  : `opa bench` rego_query_eval_ns histogram
#
# Usage: scripts/bench-vs-go-opa.sh [iterations]
# Env:   PARITY_TOLERANCE  ratio slack for measurement noise (default 1.10)
#
# Requires `opa` and `jq` on PATH. If `opa` is absent the script SKIPS with a
# clear message and exit 0, so the CI sidecar can install it out of band
# without turning a missing tool into a hard failure.
set -euo pipefail

ITERS="${1:-5000}"
TOLERANCE="${PARITY_TOLERANCE:-1.10}"
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
FIXTURES="$REPO_ROOT/crates/anvil-policy-engine/benches/fixtures"
HARNESS="$REPO_ROOT/target/release/examples/parity_harness"

# policy-file : rule-path  (rule path doubles as the opa query)
POLICIES=(
	"arch_boundary.rego:data.arch.findings"
	"baseline_filter.rego:data.baseline_filter.findings"
	"repo_scan.rego:data.repo.summary"
)

if ! command -v jq >/dev/null 2>&1; then
	echo "error: jq is required" >&2
	exit 2
fi
if ! command -v opa >/dev/null 2>&1; then
	echo "SKIP: opa (Go OPA reference) not on PATH — install it to run the parity gate."
	echo "      (CI sidecar installs it; see .github/workflows/poleng-parity.yml)"
	exit 0
fi

echo "Building regorus parity harness (release)…" >&2
cargo build --release -p eddacraft-anvil-policy-engine --example parity_harness >&2

opa_ver="$(opa version 2>/dev/null | awk '/^Version:/ {print $2}')"
echo
echo "POLENG-008 parity gate — regorus vs Go OPA ${opa_ver:-unknown}  (iters=$ITERS, tolerance=${TOLERANCE}x)"
printf '%-20s %14s %14s %8s  %s\n' "policy" "regorus p50" "OPA p50" "ratio" "verdict"
printf '%-20s %14s %14s %8s  %s\n' "------" "-----------" "-------" "-----" "-------"

failed=0
for entry in "${POLICIES[@]}"; do
	pol="${entry%%:*}"
	rule="${entry##*:}"

	reg_json="$("$HARNESS" "$FIXTURES/$pol" "$FIXTURES/input.json" "$rule" "$ITERS")"
	reg_p50="$(printf '%s' "$reg_json" | jq '.p50')"

	opa_json="$(opa bench --count 1 --format json -d "$FIXTURES/$pol" -i "$FIXTURES/input.json" "$rule" 2>/dev/null)"
	opa_p50="$(printf '%s' "$opa_json" | jq '.Extra."histogram_timer_rego_query_eval_ns_median"')"

	# ratio = regorus / opa ; PASS when regorus is at/above parity (ratio <= tolerance).
	read -r ratio verdict < <(jq -rn --argjson r "$reg_p50" --argjson o "$opa_p50" --argjson t "$TOLERANCE" \
		'($r / $o) as $ratio | "\(($ratio*100|round)/100) \(if $ratio <= $t then "PASS" else "FAIL" end)"')
	[ "$verdict" = "FAIL" ] && failed=1

	printf '%-20s %12.1fµs %12.1fµs %7sx  %s\n' \
		"${pol%.rego}" \
		"$(jq -rn --argjson v "$reg_p50" '$v/1000')" \
		"$(jq -rn --argjson v "$opa_p50" '$v/1000')" \
		"$ratio" "$verdict"
done

echo
if [ "$failed" -eq 0 ]; then
	echo "GATE: PASS — regorus is at or above Go OPA parity on every measured policy (ADR-040 D-1 holds)."
	exit 0
fi
echo "GATE: FAIL — regorus is slower than Go OPA on one or more policies." >&2
echo "      Per ADR-040 D-5 this triggers an engine-choice revisit; do not silently override." >&2
exit 1
