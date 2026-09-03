#!/usr/bin/env bash
# shellcheck disable=SC2016 # Literal GitHub expressions and shell snippets are fixture data.
# CIB-394: Council evidence must belong to the current protected-path PR head.

set -euo pipefail

repo_root=$(git rev-parse --show-toplevel 2>/dev/null || pwd)
workflow="${repo_root}/.github/workflows/council-gate.yml"

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

assert_not_contains() {
  local forbidden="$1"
  if grep -Fq -- "${forbidden}" "${workflow}"; then
    echo "expected ${workflow} not to contain: ${forbidden}" >&2
    exit 1
  fi
}

# A required PR gate must not expose a no-PR success run.
assert_contains 'pull_request:'
assert_contains 'BASE_SHA: ${{ github.event.pull_request.base.sha }}'
assert_contains 'HEAD_SHA: ${{ github.event.pull_request.head.sha }}'
assert_not_contains 'workflow_dispatch:'
assert_not_contains 'No pull_request context; nothing to gate.'
assert_not_contains 'if [[ -z "${BASE_SHA:-}" || -z "${HEAD_SHA:-}" ]]'

# Forks cannot dismiss labels with the read-only token, so only the actual
# council:reviewed label event for the current head can count as evidence.
assert_contains 'EVENT_ACTION: ${{ github.event.action }}'
assert_contains 'EVENT_LABEL: ${{ github.event.label.name }}'
assert_contains 'RUN_ATTEMPT: ${{ github.run_attempt }}'
assert_contains 'LIVE_HEAD_SHA=$(gh pr view "$PR_NUMBER" --json headRefOid --jq .headRefOid)'
assert_contains '[[ "$LIVE_HEAD_SHA" == "$HEAD_SHA" ]]'
assert_contains '[[ "$EVENT_ACTION" == "labeled" ]]'
assert_contains '[[ "$EVENT_LABEL" == "$GATE_LABEL" ]]'
assert_contains '[[ "$RUN_ATTEMPT" == "1" ]]'

# Exercise the exact fork acceptance predicate as a truth table. This keeps the
# policy legible while the string assertions above bind the model's inputs and
# comparisons to the workflow implementation.
fork_evidence_is_current() {
  local action="$1" label="$2" event_head="$3" live_head="$4" attempt="$5"
  [[ "$live_head" == "$event_head" ]] &&
    [[ "$action" == "labeled" ]] &&
    [[ "$label" == "council:reviewed" ]] &&
    [[ "$attempt" == "1" ]]
}

assert_accepts() {
  if ! fork_evidence_is_current "$@"; then
    echo "expected fork evidence to be accepted: $*" >&2
    exit 1
  fi
}

assert_rejects() {
  if fork_evidence_is_current "$@"; then
    echo "expected fork evidence to be rejected: $*" >&2
    exit 1
  fi
}

assert_accepts labeled council:reviewed head-2 head-2 1
assert_rejects synchronize council:reviewed head-2 head-2 1
assert_rejects reopened council:reviewed head-2 head-2 1
assert_rejects labeled unrelated head-2 head-2 1
assert_rejects labeled council:reviewed head-1 head-2 1
assert_rejects labeled council:reviewed head-2 head-2 2

printf 'council-gate-workflow.test.sh: ok\n'
