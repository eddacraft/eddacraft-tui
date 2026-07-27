#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
SCRIPT="$ROOT/scripts/release/validate-publication-token.sh"

fail() {
  echo "validate-publication-token.test: $*" >&2
  exit 1
}

assert_eq() {
  [ "$1" = "$2" ] || fail "expected '$2', got '$1' ($3)"
}

assert_contains() {
  case "$1" in
    *"$2"*) ;;
    *) fail "expected output to contain '$2'; got: $1" ;;
  esac
}

# --- decision logic, no network -------------------------------------------
# The script guards its entry point so the tests can source it and exercise
# the expiry classification directly.
# shellcheck source=/dev/null
source "$SCRIPT"

NOW="2026-07-27T00:00:00Z"

assert_eq "$(classify_expiry "2026-08-30T00:00:00Z" "$NOW" 14)" "ok 34" "comfortably valid"
assert_eq "$(classify_expiry "2026-08-10T00:00:00Z" "$NOW" 14)" "ok 14" "exactly at the margin is not expiring"
assert_eq "$(classify_expiry "2026-08-09T00:00:00Z" "$NOW" 14)" "expiring 13" "one day inside the margin"
assert_eq "$(classify_expiry "2026-07-27T12:00:00Z" "$NOW" 14)" "expiring 0" "expires today"
assert_eq "$(classify_expiry "2026-07-20T00:00:00Z" "$NOW" 14)" "expired -7" "already expired"

# A credential that cannot expire advertises no header. That is the good case
# and must not block a cut.
assert_eq "$(classify_expiry "" "$NOW" 14)" "unknown" "absent header"
assert_eq "$(classify_expiry "not-a-date" "$NOW" 14)" "unknown" "unparseable header"

# A zero margin disables the early warning without disabling expiry detection.
assert_eq "$(classify_expiry "2026-07-27T12:00:00Z" "$NOW" 0)" "ok 0" "zero margin accepts today"
assert_eq "$(classify_expiry "2026-07-26T00:00:00Z" "$NOW" 0)" "expired -1" "zero margin still catches expired"

# --- absent credential ----------------------------------------------------
# The failure this whole script exists to prevent: an unset secret silently
# falling back to GITHUB_TOKEN and dying at the cross-repo publish step.
set +e
out="$(ANVIL_RELEASES_TOKEN='' bash "$SCRIPT" 2>&1)"
rc=$?
set -e
assert_eq "$rc" "2" "absent credential must exit 2"
assert_contains "$out" "empty or unset"
assert_contains "$out" "GITHUB_TOKEN"

# --- usage errors ---------------------------------------------------------
set +e
out="$(ANVIL_RELEASES_TOKEN=x bash "$SCRIPT" --min-days notanumber 2>&1)"
rc=$?
set -e
assert_eq "$rc" "1" "bad --min-days must exit 1 (usage), not 2 (credential)"
assert_contains "$out" "non-negative integer"

set +e
out="$(ANVIL_RELEASES_TOKEN=x bash "$SCRIPT" --nonsense 2>&1)"
rc=$?
set -e
assert_eq "$rc" "1" "unknown argument must exit 1"

out="$(bash "$SCRIPT" --help 2>&1)"
assert_contains "$out" "Usage: validate-publication-token.sh"

echo "validate-publication-token.test.sh: ok"
