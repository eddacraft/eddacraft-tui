#!/usr/bin/env bash
# Resource-budget workflow tier contract: not on routine PRs; nightly + push +
# readiness (workflow_call) + dispatch.

set -euo pipefail

repo_root=$(git rev-parse --show-toplevel 2>/dev/null || pwd)
workflow="${repo_root}/.github/workflows/resource-budget.yml"

if [ ! -f "${workflow}" ]; then
  echo "expected ${workflow} to exist" >&2
  exit 1
fi

assert_contains() {
  local expected="$1"
  if ! grep -Fq -- "${expected}" "${workflow}"; then
    echo "expected ${workflow} to contain: ${expected}" >&2
    exit 1
  fi
}

if grep -E '^[[:space:]]*pull_request:' "${workflow}" >/dev/null; then
  echo "expected ${workflow} not to declare a pull_request trigger (PR cost control)" >&2
  exit 1
fi

assert_contains 'schedule:'
assert_contains "cron: '15 2 * * *'"
assert_contains 'workflow_dispatch: {}'
assert_contains 'workflow_call:'
# Require the reusable-workflow *input* named `ref` under workflow_call.inputs,
# not a generic checkout `with.ref` key elsewhere in the file.
if ! awk '
  /^[[:space:]]*workflow_call:[[:space:]]*$/ { in_call = 1; next }
  in_call && /^[[:space:]]*inputs:[[:space:]]*$/ { in_inputs = 1; next }
  in_call && /^[a-zA-Z]/ { in_call = 0; in_inputs = 0 }
  in_inputs && /^[[:space:]]+ref:[[:space:]]*$/ { found = 1; exit }
  END { exit found ? 0 : 1 }
' "${workflow}"; then
  echo "expected ${workflow} workflow_call.inputs.ref to be defined for release-readiness" >&2
  exit 1
fi
assert_contains 'description: Exact git ref or SHA to check out (release-readiness)'
assert_contains "github.event_name == 'workflow_dispatch'"
assert_contains 'shared-key: rust-release-budget'

printf 'resource-budget-workflow.test.sh: ok\n'
