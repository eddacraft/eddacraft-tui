#!/usr/bin/env bash
# CIB-320: the Nx graph check must fail on an unnamed inferred
# project and pass on a clean tree. A guard that cannot be made to fail
# this way has not been proven to reach the defect.

set -euo pipefail

repo_root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)
script="${repo_root}/scripts/ci/nx-graph-check.sh"
ci_workflow="${repo_root}/.github/workflows/ci.yml"
package_json="${repo_root}/package.json"
nxignore="${repo_root}/.nxignore"
fixture_pkg="${repo_root}/benchmarks/fixtures/devacc/mini-ts-service/package.json"

fail() {
  echo "FAIL: $*" >&2
  exit 1
}

assert_contains() {
  local file="$1"
  local expected="$2"
  if ! grep -Fq -- "${expected}" "${file}"; then
    fail "expected ${file} to contain: ${expected}"
  fi
}

job_block_contains() {
  local job="$1"
  local needle="$2"
  awk -v job="^  ${job}:$" -v needle="${needle}" '
    $0 ~ job { inside = 1; next }
    inside && /^  [a-z]/ { inside = 0 }
    inside && index($0, needle) { found = 1 }
    END { exit (found ? 0 : 1) }
  ' "${ci_workflow}"
}

job_block_excludes() {
  local job="$1"
  local needle="$2"
  if job_block_contains "${job}" "${needle}"; then
    fail "expected ${job} in ${ci_workflow} not to contain: ${needle}"
  fi
}

# --- contract: local script uses a cold cache ---
[[ -f "${script}" ]] || fail "expected ${script} to exist"
[[ -x "${script}" ]] || fail "expected ${script} to be executable"
assert_contains "${script}" 'NX_DAEMON=false'
assert_contains "${script}" 'NX_WORKSPACE_DATA_DIRECTORY'
assert_contains "${script}" 'NX_CACHE_DIRECTORY'
assert_contains "${script}" 'mktemp -d'
assert_contains "${script}" 'nx show projects'
assert_contains "${package_json}" '"nx:graph:check"'
assert_contains "${package_json}" 'scripts/ci/nx-graph-check.sh'

# --- contract: CI job is always-on, named Nx Graph, not path-gated ---
assert_contains "${ci_workflow}" '  nx-graph:'
assert_contains "${ci_workflow}" '    name: Nx Graph'
assert_contains "${ci_workflow}" 'bash scripts/ci/nx-graph-check.sh'
job_block_contains nx-graph 'if: always()' || fail 'expected nx-graph job to use if: always()'
job_block_excludes nx-graph 'detect-changes.outputs'
job_block_excludes nx-graph 'paths:'

# Included in the push merge-gate aggregator.
awk '
  /^  integration-readiness:/ { inside = 1; next }
  inside && /^  [a-z]/ { inside = 0 }
  inside && $0 ~ /- nx-graph$/ { found = 1 }
  END { exit (found ? 0 : 1) }
' "${ci_workflow}" || fail 'expected integration-readiness to depend on nx-graph'
assert_contains "${ci_workflow}" 'NX_GRAPH_RESULT'
assert_contains "${ci_workflow}" 'Nx Graph:${NX_GRAPH_RESULT}'

# --- GREEN: clean tree ---
clean_out=$(bash "${script}" 2>&1) || fail "expected clean-tree nx-graph-check to pass, got:
${clean_out}"

# --- RED: unnamed inferred project must fail with the Nx error text ---
nxignore_backup=$(mktemp)
fixture_backup=$(mktemp)
restore() {
  cp "${nxignore_backup}" "${nxignore}"
  cp "${fixture_backup}" "${fixture_pkg}"
  rm -f "${nxignore_backup}" "${fixture_backup}"
}
cp "${nxignore}" "${nxignore_backup}"
cp "${fixture_pkg}" "${fixture_backup}"
trap restore EXIT

grep -vx 'benchmarks/fixtures' "${nxignore}" > "${nxignore}.tmp"
mv "${nxignore}.tmp" "${nxignore}"
node -e '
  const fs = require("node:fs");
  const path = process.argv[1];
  const pkg = JSON.parse(fs.readFileSync(path, "utf8"));
  delete pkg.name;
  fs.writeFileSync(path, `${JSON.stringify(pkg, null, 2)}\n`);
' "${fixture_pkg}"

set +e
red_out=$(bash "${script}" 2>&1)
red_status=$?
set -e

[[ "${red_status}" -ne 0 ]] || fail "expected nx-graph-check to fail when the unnamed fixture is visible"
printf '%s\n' "${red_out}" | grep -Fq 'projects in the following directories have no name provided' ||
  fail "expected Nx unnamed-project error text, got:
${red_out}"

restore
trap - EXIT

# Restore must return the check to green.
restored_out=$(bash "${script}" 2>&1) || fail "expected nx-graph-check to pass after restore, got:
${restored_out}"

echo 'nx-graph-check contract and RED/GREEN proof passed'
