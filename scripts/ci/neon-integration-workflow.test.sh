#!/usr/bin/env bash
# CLAWOPEN-011: contract for the credentialed ephemeral-Neon assurance job.

set -euo pipefail

repo_root=$(git rev-parse --show-toplevel 2>/dev/null || pwd)
workflow="${repo_root}/.github/workflows/neon-integration.yml"
readme="${repo_root}/.github/workflows/README.md"
provisioner="${repo_root}/scripts/ci/create-neon-test-branch.mjs"
provisioner_test="${repo_root}/scripts/ci/create-neon-test-branch.test.mjs"

fail() {
  echo "FAIL: $*" >&2
  exit 1
}

assert_contains() {
  local path="$1"
  local expected="$2"
  grep -Fq -- "${expected}" "${path}" || fail "expected ${path} to contain: ${expected}"
}

assert_not_contains() {
  local path="$1"
  local forbidden="$2"
  if grep -Fq -- "${forbidden}" "${path}"; then
    fail "expected ${path} not to contain: ${forbidden}"
  fi
}

assert_not_matches() {
  local path="$1"
  local forbidden="$2"
  if grep -Eq -- "${forbidden}" "${path}"; then
    fail "expected ${path} not to match: ${forbidden}"
  fi
}

[[ -f "${workflow}" ]] || fail "expected workflow to exist: ${workflow}"
[[ -f "${readme}" ]] || fail "expected workflow inventory to exist: ${readme}"
[[ -f "${provisioner}" ]] || fail "expected repo-owned Neon provisioner: ${provisioner}"
[[ -f "${provisioner_test}" ]] || fail "expected provisioner regression test: ${provisioner_test}"

# Trigger and authority boundary: only internal PR heads receive credentials;
# manual dispatch remains available after repository provisioning.
assert_contains "${workflow}" 'pull_request:'
assert_contains "${workflow}" 'branches: [main]'
assert_contains "${workflow}" 'workflow_dispatch: {}'
assert_contains "${workflow}" "- 'apps/anvil-api/**'"
assert_contains "${workflow}" "- '.github/workflows/neon-integration.yml'"
assert_contains "${workflow}" "- 'scripts/ci/neon-integration-workflow.test.sh'"
assert_contains "${workflow}" "- 'scripts/ci/create-neon-test-branch.mjs'"
assert_contains "${workflow}" "- 'scripts/ci/create-neon-test-branch.test.mjs'"
assert_contains "${workflow}" "- 'package.json'"
assert_contains "${workflow}" "- 'pnpm-lock.yaml'"
assert_contains "${workflow}" 'github.event.pull_request.head.repo.full_name'
assert_contains "${workflow}" '== github.repository }}'

assert_contains "${workflow}" 'permissions:'
assert_contains "${workflow}" 'contents: read'
assert_not_contains "${workflow}" 'contents: write'
assert_not_contains "${workflow}" 'actions: write'
assert_not_contains "${workflow}" 'id-token: write'
assert_not_contains "${workflow}" 'pull-requests: write'
assert_contains "${workflow}" 'runs-on: ubuntu-latest'
assert_contains "${workflow}" 'timeout-minutes: 15'
assert_contains "${workflow}" 'concurrency:'
assert_contains "${workflow}" 'cancel-in-progress: true'
assert_contains "${workflow}" 'ref:'
assert_contains "${workflow}" "github.event_name == 'pull_request' && github.event.pull_request.head.sha ||"
assert_contains "${workflow}" 'github.sha }}'
assert_contains "${workflow}" 'persist-credentials: false'

# The repo-owned provisioner verifies the control-plane project before the
# branch POST, masks generated credentials, and only then emits the direct URL.
assert_not_contains "${workflow}" 'neondatabase/create-branch-action'
assert_contains "${workflow}" 'node scripts/ci/create-neon-test-branch.mjs'
assert_contains "${workflow}" 'NEON_API_KEY: ${{ secrets.NEON_API_KEY }}'
assert_contains "${workflow}" 'NEON_PROJECT_ID: ${{ vars.NEON_PROJECT_ID }}'
assert_contains "${workflow}" 'NEON_BRANCH_NAME: ${{ steps.branch-metadata.outputs.branch_name }}'
assert_contains "${workflow}" 'NEON_BRANCH_EXPIRES_AT: ${{ steps.branch-metadata.outputs.expires_at }}'
assert_contains "${workflow}" 'ci-test-clawopen-011-${GITHUB_RUN_ID}-${GITHUB_RUN_ATTEMPT}'
assert_contains "${workflow}" 'ANVIL_API_TEST_DATABASE_URL: ${{ steps.create-branch.outputs.db_url }}'
assert_contains "${workflow}" 'ANVIL_API_TEST_DATABASE_PROJECT_NAME: anvil-api-test'
assert_contains "${workflow}" 'ANVIL_API_TEST_DATABASE_BRANCH_NAME: ${{ steps.branch-metadata.outputs.branch_name }}'
assert_not_contains "${workflow}" 'db_url_pooled'
assert_not_matches "${workflow}" '^[[:space:]]*DATABASE_URL:'
assert_contains "${workflow}" 'pnpm --dir apps/anvil-api test:neon'
assert_contains "${provisioner}" "const EXPECTED_PROJECT_NAME = 'anvil-api-test'"
assert_contains "${provisioner}" "const EXPECTED_DATABASE_NAME = 'anvil_test'"
assert_contains "${provisioner}" "const EXPECTED_ROLE_NAME = 'anvil_test_owner'"
assert_contains "${provisioner}" '::add-mask::'
assert_contains "${provisioner}" 'writeOutput(`db_url=${rawDatabaseUrl}`)'
assert_contains "${provisioner}" 'writeOutput(`branch_id=${branchId}`)'

# Cleanup resolves an exact branch name when a partial create did not return an
# ID. The branch expiry remains the final fallback for runner cancellation.
assert_contains "${workflow}" 'if: ${{ always()'
assert_contains "${workflow}" "steps.create-branch.outputs.branch_name != ''"
assert_contains "${workflow}" 'NEON_BRANCH_ID: ${{ steps.create-branch.outputs.branch_id }}'
assert_contains "${workflow}" 'NEON_BRANCH_NAME: ${{ steps.create-branch.outputs.branch_name }}'
assert_contains "${workflow}" 'NEON_BRANCH_EXPIRES_AT: ${{ steps.create-branch.outputs.expires_at }}'
assert_contains "${workflow}" 'node scripts/ci/create-neon-test-branch.mjs --delete'
assert_contains "${provisioner}" "method: 'DELETE'"
assert_contains "${provisioner}" 'branch?.name === branchName'
assert_not_contains "${workflow}" 'neondatabase/delete-branch-action'
assert_not_contains "${workflow}" 'neonctl'

assert_contains "${readme}" '`neon-integration.yml`'
assert_contains "${readme}" 'CLAWOPEN-011'

echo 'Neon integration workflow contract passed'
