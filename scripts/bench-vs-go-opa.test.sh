#!/usr/bin/env bash
# Fixture test for scripts/bench-vs-go-opa.sh (CIB-019).
#
# Verifies that a failed `opa bench` surfaces Go OPA's stderr (instead of a bare
# "no positive measurement"), and that the happy path still reports GATE: PASS.
# Stubs `opa` (on PATH) and the regorus harness (via BENCH_HARNESS), so it needs
# neither a real `opa` nor a release build.
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
SCRIPT="$ROOT/scripts/bench-vs-go-opa.sh"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

fail() {
	printf 'bench-vs-go-opa.test.sh: FAIL: %s\n' "$1" >&2
	printf -- '--- stdout ---\n%s\n--- stderr ---\n%s\n' "$(cat "$tmp/out")" "$(cat "$tmp/err")" >&2
	exit 1
}

# Stub regorus harness: ignore args, emit a valid p50.
cat >"$tmp/harness" <<'EOF'
#!/usr/bin/env bash
echo '{"p50": 1000}'
EOF
chmod +x "$tmp/harness"

# Stub opa: `version` always works; `bench` is ok or errors per FAKE_OPA_MODE.
mkdir -p "$tmp/bin"
cat >"$tmp/bin/opa" <<'EOF'
#!/usr/bin/env bash
case "$1" in
	version) echo "Version: 0.0.0-fake" ;;
	bench)
		if [ "${FAKE_OPA_MODE:-ok}" = "err" ]; then
			echo "OPA-STUB-BOOM: rego_parse_error on stub policy" >&2
			exit 1
		fi
		echo '{"Extra": {"histogram_timer_rego_query_eval_ns_median": 1000}}'
		;;
	*) echo "fake opa: unhandled $*" >&2; exit 2 ;;
esac
EOF
chmod +x "$tmp/bin/opa"

run() { # <FAKE_OPA_MODE>
	FAKE_OPA_MODE="$1" BENCH_HARNESS="$tmp/harness" PATH="$tmp/bin:$PATH" \
		bash "$SCRIPT" 1 >"$tmp/out" 2>"$tmp/err"
}

# 1. Happy path: valid `opa bench` → GATE: PASS (exit 0).
set +e
run ok
ok_code=$?
set -e
[ "$ok_code" -eq 0 ] || fail "happy path should exit 0 (GATE PASS), got $ok_code"
grep -q 'GATE: PASS' "$tmp/out" || fail "expected 'GATE: PASS' on stdout"

# 2. opa-error path: `opa bench` fails → exit 2 AND OPA's stderr is surfaced.
set +e
run err
err_code=$?
set -e
[ "$err_code" -eq 2 ] || fail "opa-error path should exit 2 (cannot conclude), got $err_code"
grep -q 'no positive measurement' "$tmp/err" || fail "expected the gate-cannot-conclude message"
grep -q 'OPA-STUB-BOOM' "$tmp/err" || fail "OPA's stderr was not surfaced (CIB-019 regression)"

printf 'bench-vs-go-opa.test.sh: ok\n'
