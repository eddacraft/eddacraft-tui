#!/usr/bin/env bash
# Run every kit self-test.
#
# This script is the single source of the kit's test list. CI invokes it
# rather than restating the list, so a test added here is a test CI runs —
# there is no second place to forget.
#
# It also makes the suite runnable by anyone who adopted the kit from the
# public mirror, where the upstream CI workflow does not travel. See
# `kit-tests.yml.snippet` beside the kit for a workflow you can copy into
# your own `.github/workflows/`, including the external tool versions the
# driver tests expect.
#
# Usage:
#   tests/run-all.sh                 # run everything; skips are tolerated
#   tests/run-all.sh --require-all   # a skipped test is a failure (CI)
#   tests/run-all.sh --list          # print the test list, run nothing
#
# Why --require-all: several driver tests exit 0 with a `skip:` line when
# their external tool or network is unavailable, so a provisioning
# regression would otherwise show up as a green run with three ecosystems
# silently unexercised. CI passes --require-all so "green" means "every
# test actually ran".
#
# Exit codes:
#   0  every test passed (and, under --require-all, none skipped)
#   1  a test failed, or skipped while --require-all was set
#   2  CLI argument error

set -euo pipefail

TESTS_DIR="$(cd "$(dirname "$0")" && pwd)"

# Order is deliberate: dispatcher contract first (cheapest, no external
# tools), then per-ecosystem drivers, then the repo-level drift check.
TESTS=(
  dispatcher-schema-validation.sh
  dispatcher-two-block.sh
  strict-license-field.sh
  licences-drift.sh
  version-changelog-consistency.sh
  node-driver-preflight.sh
  node-driver-render.sh
  node-driver-strict.sh
  go-driver-preflight.sh
  go-driver-render.sh
  go-driver-strict.sh
  python-driver-preflight.sh
  python-driver-render.sh
  python-driver-strict.sh
  bundled-binaries-preflight.sh
  bundled-binaries-render.sh
)

require_all=false
list_only=false

while [ $# -gt 0 ]; do
  case "$1" in
    --require-all) require_all=true; shift ;;
    --list)        list_only=true; shift ;;
    -h|--help)     sed -n '2,27p' "$0"; exit 0 ;;
    *)             echo "unknown argument: $1" >&2; exit 2 ;;
  esac
done

if [ "$list_only" = true ]; then
  printf '%s\n' "${TESTS[@]}"
  exit 0
fi

# The array above is ordered deliberately, so it is written by hand — which
# would make "add a test to tests/ and CI runs it" a promise the runner
# cannot keep. Enforce it instead: any test file on disk that is missing
# from TESTS is an error, not a silent omission.
missing=""
for candidate in "$TESTS_DIR"/*.sh; do
  base="$(basename "$candidate")"
  [ "$base" = "run-all.sh" ] && continue
  case " ${TESTS[*]} " in
    *" $base "*) ;;
    *) missing="$missing $base" ;;
  esac
done
if [ -n "$missing" ]; then
  echo "error: test file(s) present in $TESTS_DIR but absent from this runner's list:" >&2
  for m in $missing; do echo "  - $m" >&2; done
  echo "  add them to TESTS in $(basename "$0") so CI actually runs them." >&2
  exit 1
fi

passed=0
skipped=0
failed=0
skipped_names=""
failed_names=""

for test in "${TESTS[@]}"; do
  script="$TESTS_DIR/$test"
  if [ ! -f "$script" ]; then
    echo "FAIL $test — not found at $script" >&2
    failed=$((failed + 1))
    failed_names="$failed_names $test"
    continue
  fi

  output_file="$(mktemp)"
  status=0
  bash "$script" >"$output_file" 2>&1 || status=$?

  if [ "$status" -ne 0 ]; then
    echo "FAIL $test (exit $status)"
    sed 's/^/    /' "$output_file"
    failed=$((failed + 1))
    failed_names="$failed_names $test"
  elif grep -q '^skip:' "$output_file"; then
    # A test that exits 0 having announced `skip:` did not exercise its
    # subject. Report it as its own outcome so it can never be mistaken
    # for coverage.
    echo "SKIP $test — $(grep -m1 '^skip:' "$output_file")"
    skipped=$((skipped + 1))
    skipped_names="$skipped_names $test"
  else
    echo "PASS $test"
    passed=$((passed + 1))
  fi
  rm -f "$output_file"
done

echo
echo "kit self-tests: $passed passed, $skipped skipped, $failed failed (of ${#TESTS[@]})"

if [ "$failed" -gt 0 ]; then
  echo "failed:$failed_names" >&2
  exit 1
fi

if [ "$skipped" -gt 0 ]; then
  echo "skipped:$skipped_names" >&2
  if [ "$require_all" = true ]; then
    echo "" >&2
    echo "error: --require-all was set, so a skipped test is a failure." >&2
    echo "  a skip means the test's external tool or network was unavailable and its" >&2
    echo "  subject went unexercised — provision the tool rather than accepting the skip." >&2
    exit 1
  fi
fi

exit 0
