#!/usr/bin/env bash
# shellcheck disable=SC2016 # Literal GitHub expressions and shell snippets are fixture data.
# CIB-394: Council evidence must belong to the current protected-path PR head.

set -euo pipefail

repo_root=$(git rev-parse --show-toplevel 2>/dev/null || pwd)
workflow="${repo_root}/.github/workflows/council-gate.yml"
ci_workflow="${repo_root}/.github/workflows/ci.yml"
local_validator="${repo_root}/scripts/validate/local.sh"

if [ ! -f "${workflow}" ]; then
  echo "expected ${workflow} to exist" >&2
  exit 1
fi

for runner in "$ci_workflow" "$local_validator"; do
  if ! grep -Fq 'pnpm test:ci-council-gate-workflow' "$runner"; then
    echo "expected ${runner} to run the Council gate fixture" >&2
    exit 1
  fi
done

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
assert_contains 'types: [opened, synchronize, reopened, edited, labeled, unlabeled]'
assert_contains 'BASE_SHA: ${{ github.event.pull_request.base.sha }}'
assert_contains 'HEAD_SHA: ${{ github.event.pull_request.head.sha }}'
assert_not_contains 'workflow_dispatch:'
assert_not_contains 'No pull_request context; nothing to gate.'
assert_not_contains 'if [[ -z "${BASE_SHA:-}" || -z "${HEAD_SHA:-}" ]]'
assert_not_contains 'mapfile -t changed < <(git diff'
assert_contains 'CHANGED_FILES=$(git diff --name-only "${BASE_SHA}...${HEAD_SHA}")'

changed_files_query_failure_is_fatal() (
  git() { return 43; }
  set -euo pipefail
  changed_files=$(git diff --name-only base...head)
  [[ -z "$changed_files" ]]
)

set +e
changed_files_query_failure_is_fatal
diff_status=$?
set -e
if [[ "$diff_status" != "43" ]]; then
  echo "expected git diff failure status 43, got ${diff_status}" >&2
  exit 1
fi

# Label API failures must not be interpreted as label absence inside an `if`
# pipeline. Bind the workflow to fail-fast retrieval before membership tests.
assert_not_contains 'if gh pr view "$PR_NUMBER" --json labels'
assert_contains 'CURRENT_LABELS=$(gh pr view "$PR_NUMBER" --json labels --jq '\''.labels[].name'\'')'
assert_contains 'UPDATED_LABELS=$(gh pr view "$PR_NUMBER" --json labels --jq '\''.labels[].name'\'')'

label_query_failure_is_fatal() (
  gh() { return 42; }
  set -euo pipefail
  labels=$(gh pr view 123 --json labels --jq '.labels[].name')
  grep -Fxq council:reviewed <<<"$labels"
)

set +e
label_query_failure_is_fatal
query_status=$?
set -e
if [[ "$query_status" != "42" ]]; then
  echo "expected label query failure status 42, got ${query_status}" >&2
  exit 1
fi

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
assert_contains 'dismiss-stale-review:'
assert_contains 'pull-requests: write'
assert_contains 'GH_REPO: ${{ github.repository }}'
assert_contains "github.event.action == 'edited' && github.event.changes.base != null"
assert_contains 'council-gate:'
assert_contains 'pull-requests: read'

dismiss_job=$(sed -n '/^  dismiss-stale-review:/,/^  council-gate:/p' "$workflow")
if grep -Fq 'actions/checkout' <<<"$dismiss_job"; then
  echo 'dismiss-stale-review must not checkout or execute PR-controlled files' >&2
  exit 1
fi

non_git_dir=$(mktemp -d)
trap 'rm -rf "$non_git_dir"' EXIT
gh() {
  [[ "${GH_REPO:-}" == 'eddacraft/anvil' ]] || return 44
  [[ "$PWD" == "$non_git_dir" ]] || return 45
  printf 'council:reviewed\n'
}
(
  cd "$non_git_dir"
  GH_REPO=eddacraft/anvil gh pr view 123 --json labels --jq '.labels[].name'
) >/dev/null

gate_job=$(sed -n '/^  council-gate:/,$p' "$workflow")
if grep -Fq 'pull-requests: write' <<<"$gate_job"; then
  echo 'council-gate must remain read-only' >&2
  exit 1
fi

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
assert_rejects edited council:reviewed head-2 head-2 1

printf 'council-gate-workflow.test.sh: ok\n'
