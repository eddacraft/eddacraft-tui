#!/usr/bin/env bash
# CICD-011: lock the CI integration of the APS/repo/release drift check.
#
# `ci.yml`'s `aps-drift` job is the warning-mode drift gate. This fixture
# asserts it still:
#   - is `continue-on-error: true` (warning-mode, never blocking)
#   - reads changed files from the same fixture path for both push and PR
#   - hands the PR title + body to drift-check.mjs on `pull_request`
#     events so the `pr-missing-aps-reference` finding can fire
#   - leaves push runs scoped to changed-files only

set -euo pipefail

repo_root=$(git rev-parse --show-toplevel 2>/dev/null || pwd)
ci_workflow="${repo_root}/.github/workflows/ci.yml"
drift_script="${repo_root}/scripts/aps/drift-check.mjs"

assert_contains() {
  local file="$1"
  local expected="$2"
  if ! grep -Fq -- "${expected}" "${file}"; then
    echo "expected ${file} to contain: ${expected}" >&2
    exit 1
  fi
}

assert_block_contains() {
  # `block_marker` is interpreted as an awk regex; `expected` is a fixed
  # substring matched via `index()`.
  local file="$1"
  local block_marker="$2"
  local expected="$3"
  if ! awk -v marker="${block_marker}" -v expected="${expected}" '
    $0 ~ marker { inside = 1; next }
    inside && /^  [a-z]/ { inside = 0 }
    inside && index($0, expected) > 0 { found = 1 }
    END { exit (found ? 0 : 1) }
  ' "${file}"; then
    echo "expected block matching '${block_marker}' in ${file} to contain: ${expected}" >&2
    exit 1
  fi
}

# ── aps-drift job ────────────────────────────────────────────────
assert_contains "${ci_workflow}" '  aps-drift:'
assert_block_contains "${ci_workflow}" "^  aps-drift:" 'continue-on-error: true'
assert_block_contains "${ci_workflow}" "^  aps-drift:" '/tmp/aps-drift-files.txt'

# Push and PR each get their own invocation of drift-check.mjs — the PR
# variant must pass --pr-title and --pr-body-file.
assert_block_contains "${ci_workflow}" "^  aps-drift:" 'Run warning-mode drift checks (push)'
assert_block_contains "${ci_workflow}" "^  aps-drift:" 'Run warning-mode drift checks (PR)'
assert_block_contains "${ci_workflow}" "^  aps-drift:" '--pr-title'
assert_block_contains "${ci_workflow}" "^  aps-drift:" '--pr-body-file'
assert_block_contains "${ci_workflow}" "^  aps-drift:" '/tmp/aps-drift-pr-body.txt'

# Drift script declares the PR-metadata flags + the new finding codes.
assert_contains "${drift_script}" "'--pr-title'"
assert_contains "${drift_script}" "'--pr-body-file'"
assert_contains "${drift_script}" "'pr-missing-aps-reference'"
assert_contains "${drift_script}" "'pr-aps-reference-unknown'"

echo 'drift-check CI integration checks passed'
