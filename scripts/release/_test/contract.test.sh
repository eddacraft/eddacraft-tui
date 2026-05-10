#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
HARNESS="$ROOT/scripts/release/_test/harness.sh"
FIXTURE="$ROOT/scripts/release/_test/fixtures/contract-command.sh"

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

assert_contains() {
  local haystack="$1"
  local needle="$2"
  if [[ "$haystack" != *"$needle"* ]]; then
    echo "expected output to contain: $needle" >&2
    echo "actual output:" >&2
    echo "$haystack" >&2
    exit 1
  fi
}

bash "$HARNESS" run-contract \
  --name success-envelope \
  --expected-exit 0 \
  --expected-command assess \
  -- bash "$FIXTURE" success

if bash "$HARNESS" run-contract \
  --name non-json-stdout \
  --expected-exit 0 \
  --expected-command assess \
  -- bash "$FIXTURE" non-json >/"$tmp/non-json.out" 2>&1; then
  echo "expected non-JSON stdout to fail contract validation" >&2
  exit 1
fi
assert_contains "$(<"$tmp/non-json.out")" "stdout is not valid JSON"

if bash "$HARNESS" run-contract \
  --name missing-failure-object \
  --expected-exit 2 \
  --expected-command preflight \
  -- bash "$FIXTURE" missing-failure >/"$tmp/missing-failure.out" 2>&1; then
  echo "expected non-zero command without failures[] to fail contract validation" >&2
  exit 1
fi
assert_contains "$(<"$tmp/missing-failure.out")" "non-zero exits must include failures[]"

bash "$HARNESS" run-kill9-rerun \
  --name killable-idempotency \
  --state-file "$tmp/killable.state" \
  -- bash "$FIXTURE" killable "$tmp/killable.state"

echo "contract.test.sh: ok"
