#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
HARNESS="$ROOT/scripts/release/_test/harness.sh"
FIXTURE="$ROOT/scripts/release/_test/fixtures/contract-command.sh"
BUMP_HOMEBREW="$ROOT/scripts/release/bump-homebrew.sh"

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

# Every release command accepts the same SemVer-safe candidate-tag grammar.
# Reject malformed dot-separated prerelease identifiers before other inputs
# are validated, preventing a direct invocation from reaching Cargo or npm
# with a version those tools reject.
for release_script in preflight.sh prepare.sh promote.sh tag.sh monitor.sh verify.sh closeout.sh; do
  malformed_output="$(bash "$ROOT/scripts/release/$release_script" --json --version v1.2.3-beta..1 2>/dev/null || true)"
  assert_contains "$malformed_output" '"code":"invalid-input"'
done

malformed_homebrew_output="$(bash "$BUMP_HOMEBREW" --release-tag v1.2.3-beta..1 --formula-source "$tmp/missing.rb" --out "$tmp/anvil.rb" 2>&1 || true)"
assert_contains "$malformed_homebrew_output" '--release-tag must look like vX.Y.Z[-suffix]'

bash "$HARNESS" run-contract \
  --name success-envelope \
  --expected-exit 0 \
  --expected-command assess \
  -- bash "$FIXTURE" success

bash "$HARNESS" run-contract \
  --name metadata-comment \
  --expected-exit 0 \
  --expected-command prepare \
  -- bash "$FIXTURE" metadata-comment

bash "$HARNESS" run-contract \
  --name remote-tag-recovery \
  --expected-exit 1 \
  --expected-command tag \
  -- bash "$FIXTURE" remote-tag-recovery

bash "$HARNESS" run-contract \
  --name release-record-mismatch \
  --expected-exit 1 \
  --expected-command verify \
  -- bash "$FIXTURE" release-record-mismatch

bash "$HARNESS" run-contract \
  --name cargo-dist-failure \
  --expected-exit 1 \
  --expected-command monitor \
  -- bash "$FIXTURE" cargo-dist-failure

if bash "$HARNESS" run-contract \
  --name non-json-stdout \
  --expected-exit 0 \
  --expected-command assess \
  -- bash "$FIXTURE" non-json >"$tmp/non-json.out" 2>&1; then
  echo "expected non-JSON stdout to fail contract validation" >&2
  exit 1
fi
assert_contains "$(<"$tmp/non-json.out")" "stdout is not valid JSON"

if bash "$HARNESS" run-contract \
  --name failed-gate-mismatch \
  --expected-exit 2 \
  --expected-command preflight \
  -- bash "$FIXTURE" failed-gate-mismatch >"$tmp/missing-failure.out" 2>&1; then
  echo "expected preflight exit mismatch to fail contract validation" >&2
  exit 1
fi
assert_contains "$(<"$tmp/missing-failure.out")" "preflight exit 2 does not match failedGateCount"

if bash "$HARNESS" run-contract \
  --name invalid-failure-code \
  --expected-exit 1 \
  --expected-command preflight \
  -- bash "$FIXTURE" invalid-failure-code >"$tmp/invalid-code.out" 2>&1; then
  echo "expected out-of-schema failure code to fail contract validation" >&2
  exit 1
fi
assert_contains "$(<"$tmp/invalid-code.out")" "invalid code tool-unavailable"

bash "$HARNESS" run-kill9-rerun \
  --name killable-idempotency \
  --state-file "$tmp/killable.state" \
  -- bash "$FIXTURE" killable "$tmp/killable.state"

echo "contract.test.sh: ok"
