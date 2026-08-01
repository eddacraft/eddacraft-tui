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
assert_contains 'ref:'
assert_contains "github.event_name == 'workflow_dispatch'"
assert_contains 'shared-key: rust-release-budget'

printf 'resource-budget-workflow.test.sh: ok\n'
