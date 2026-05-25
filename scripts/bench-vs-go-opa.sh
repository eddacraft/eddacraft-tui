#!/usr/bin/env bash
#
# POLENG-008 parity gate: regorus vs the Go OPA reference.
#
# ADR-040 D-1 adopts regorus on the assumption it is at least as fast as Go OPA
# on Anvil's representative policy mix. This script measures both engines on the
# same fixtures and gates: regorus median must be at or above OPA parity on
# every policy. A failure is an ADR-040 D-5 trigger (revisit the engine choice),
# not a routine error.
#
# Scope match (so the comparison is fair): both sides measure *eval only* on a
# prepared query with the input bound once.
#   regorus : examples/parity_harness.rs loops Engine::eval_rule (compile/
#             prepare happens once; the cached `prepared` flag skips re-prep).
#   Go OPA  : `opa bench` prepares the query once and reports the per-iteration
#             rego_query_eval_ns histogram (the eval-phase timer, not parse or
#             compile).
# Neither side measures cold-start compile — this validates ADR-040's *engine
# eval* parity claim, not per-invocation startup. The facade's realistic
# per-eval cost (with serde + input-set) is tracked separately by `cargo bench`
# (benches/parity.rs). The fixtures are standard Rego (no anvil.* builtins) so
# both engines can run them; builtin-bridge overhead is regorus-internal and
# out of scope for a cross-engine comparison.
#
# Usage: scripts/bench-vs-go-opa.sh [iterations]
# Env:   PARITY_TOLERANCE  ratio slack for measurement noise (default 1.10,
#                          i.e. regorus may be at most 10% slower and still PASS)
#
# Requires `opa` and `jq` on PATH. Locally, a missing `opa` SKIPs (exit 0); in
# CI ($CI set) a missing `opa` is a hard error so the gate can never silently
# no-op.
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
	if [ -n "${CI:-}" ]; then
		echo "error: opa not on PATH in CI — the parity gate cannot run (check the setup-opa step)." >&2
		exit 1
	fi
	echo "SKIP: opa (Go OPA reference) not on PATH — install it to run the parity gate locally."
	echo "      (CI installs it via open-policy-agent/setup-opa; see .github/workflows/poleng-parity.yml)"
	exit 0
fi

echo "Building regorus parity harness (release)…" >&2
cargo build --release -p eddacraft-anvil-policy-engine --example parity_harness >&2
if [ ! -x "$HARNESS" ]; then
	echo "error: harness binary not found at $HARNESS after build" >&2
	exit 2
fi

opa_ver="$(opa version 2>/dev/null | awk '/^Version:/ {print $2}')"
echo
echo "POLENG-008 parity gate — regorus vs Go OPA ${opa_ver:-unknown}  (iters=$ITERS, tolerance=${TOLERANCE}x)"
printf '%-20s %14s %14s %8s  %s\n' "policy" "regorus p50" "OPA p50" "ratio" "verdict"
printf '%-20s %14s %14s %8s  %s\n' "------" "-----------" "-------" "-----" "-------"

# Extract a strictly-positive number from JSON, or fail the whole gate. A
# missing/null/non-numeric reading must NEVER be treated as a pass.
require_pos_num() { # <json> <jq-filter> <what>
	local val
	if ! val="$(printf '%s' "$1" | jq -e "$2 | numbers | select(. > 0)" 2>/dev/null)"; then
		echo "error: $3 produced no positive measurement (gate cannot conclude)." >&2
		exit 2
	fi
	printf '%s' "$val"
}

failed=0
for entry in "${POLICIES[@]}"; do
	pol="${entry%%:*}"
	rule="${entry##*:}"

	reg_json="$("$HARNESS" "$FIXTURES/$pol" "$FIXTURES/input.json" "$rule" "$ITERS")"
	reg_p50="$(require_pos_num "$reg_json" '.p50' "regorus ($pol)")"

	opa_json="$(opa bench --count 1 --format json -d "$FIXTURES/$pol" -i "$FIXTURES/input.json" "$rule" 2>/dev/null || true)"
	opa_p50="$(require_pos_num "$opa_json" '.Extra."histogram_timer_rego_query_eval_ns_median"' "opa ($pol)")"

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
