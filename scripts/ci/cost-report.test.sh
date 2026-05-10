#!/usr/bin/env bash
# Fixture tests for CI cost-report input handling.

set -euo pipefail

script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
reporter="${script_dir}/cost-report.sh"

if ! command -v jq >/dev/null 2>&1; then
  echo "jq is required" >&2
  exit 2
fi

tmp_dir=$(mktemp -d)
cleanup() {
  rm -rf "${tmp_dir}"
}
trap cleanup EXIT

input="${tmp_dir}/runs.json"
now='2026-05-10T00:00:00Z'
later='2026-05-10T00:10:00Z'

jq -n --arg now "${now}" --arg later "${later}" '[
  {workflowName:"one", event:"push", status:"completed", conclusion:"success", createdAt:$now, updatedAt:$later, headBranch:"dev", databaseId:1},
  {workflowName:"two", event:"push", status:"completed", conclusion:"success", createdAt:$now, updatedAt:$later, headBranch:"dev", databaseId:2},
  {workflowName:"three", event:"push", status:"completed", conclusion:"success", createdAt:$now, updatedAt:$later, headBranch:"dev", databaseId:3}
]' >"${input}"

summary=$(bash "${reporter}" --input "${input}" --limit 2 --json)
jq -e '.sampledRuns == 2' >/dev/null <<<"${summary}"
jq -e '.workflowTotals | length == 2' >/dev/null <<<"${summary}"
jq -e '.workflowTotals | map(.workflow) | index("three") | not' >/dev/null <<<"${summary}"

echo 'cost-report fixtures passed'
