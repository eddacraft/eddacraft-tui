#!/usr/bin/env bash
# Validate fast PR workflow invariants that are easy to regress in YAML edits.

set -euo pipefail

repo_root=$(git rev-parse --show-toplevel 2>/dev/null || pwd)
ci_workflow="${repo_root}/.github/workflows/ci.yml"
detect_action="${repo_root}/.github/actions/detect-changes/action.yml"

assert_contains() {
  local file="$1"
  local expected="$2"
  if ! grep -Fq -- "${expected}" "${file}"; then
    echo "expected ${file} to contain: ${expected}" >&2
    exit 1
  fi
}

assert_not_contains() {
  local file="$1"
  local unexpected="$2"
  if grep -Fq -- "${unexpected}" "${file}"; then
    echo "expected ${file} not to contain: ${unexpected}" >&2
    exit 1
  fi
}

assert_contains "${detect_action}" 'classification=$(bash scripts/ci/classify-changes.sh --context "${CLASSIFIER_CONTEXT}" --paths-file "${paths_file}")'
assert_contains "${detect_action}" 'required-checks:'
assert_contains "${detect_action}" 'unit-tests-required:'
assert_contains "${detect_action}" 'echo "unit-tests-required=true" >> "$GITHUB_OUTPUT"'
assert_contains "${detect_action}" 'PR fallback returned no changed files — treating all outputs as changed.'
assert_contains "${detect_action}" 'workflow-lint-required:'
assert_contains "${detect_action}" 'shell-syntax-required:'
assert_contains "${detect_action}" 'dependency-audit-required:'
assert_contains "${detect_action}" 'opa-required:'
assert_contains "${detect_action}" 'regal-required:'
assert_contains "${detect_action}" 'infra-static-check-required:'

assert_contains "${ci_workflow}" "needs.detect-changes.outputs.lint-required == 'true'"
assert_contains "${ci_workflow}" "needs.detect-changes.outputs.typecheck-required == 'true'"
assert_contains "${ci_workflow}" "needs.detect-changes.outputs.unit-tests-required == 'true'"
assert_contains "${ci_workflow}" "needs.detect-changes.outputs.workflow-lint-required == 'true'"
assert_contains "${ci_workflow}" "needs.detect-changes.outputs.shell-syntax-required == 'true'"
assert_contains "${ci_workflow}" "needs.detect-changes.outputs.dependency-audit-required == 'true'"
assert_contains "${ci_workflow}" "needs.detect-changes.outputs.opa-required == 'true'"
assert_contains "${ci_workflow}" "needs.detect-changes.outputs.regal-required == 'true'"
assert_contains "${ci_workflow}" "needs.detect-changes.outputs.infra-static-check-required == 'true'"
assert_contains "${ci_workflow}" 'const yaml = require("yaml")'
assert_contains "${ci_workflow}" "git ls-files 'scripts/*.sh' 'scripts/**/*.sh'"
assert_contains "${ci_workflow}" 'aquasecurity/trivy-action@ed142fd0673e97e23eac54620cfb913e5ce36c25'
assert_contains "${ci_workflow}" 'ANVIL_RELEASE_STEP_TIMEOUT=120 bash scripts/release.sh'
assert_contains "${ci_workflow}" 'pnpm --filter @eddacraft/anvil-checks-native build:debug'
assert_contains "${ci_workflow}" "needs.detect-changes.result != 'success'"
assert_contains "${ci_workflow}" "needs.detect-changes.outputs.lint-required != 'true'"
assert_contains "${ci_workflow}" 'if: github.event_name != '\''pull_request'\'' && always()'
assert_contains "${ci_workflow}" 'pnpm exec nx affected -t test --exclude=@eddacraft/anvil-e2e --exclude=@eddacraft/anvil-checks-native "${RUST_EXCLUDES[@]}"'
assert_contains "${ci_workflow}" 'pnpm exec nx run-many -t test --exclude=@eddacraft/anvil-e2e --exclude=@eddacraft/anvil-checks-native "${RUST_EXCLUDES[@]}" \'
assert_contains "${ci_workflow}" '--run --coverage --coverage.reporter=json-summary --coverage.reporter=text'

echo 'fast PR validation workflow checks passed'
