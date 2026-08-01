#!/usr/bin/env bash
# Contract tests for the manual release-readiness workflow skeleton.

set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
workflow="$ROOT/.github/workflows/release-readiness.yml"

assert_file_exists() {
  local path="$1"
  if [[ ! -f "$path" ]]; then
    echo "FAIL: expected file to exist: $path" >&2
    exit 1
  fi
}

assert_contains() {
  local haystack="$1"
  local needle="$2"
  local message="$3"
  if [[ "$haystack" != *"$needle"* ]]; then
    echo "FAIL: $message" >&2
    echo "missing: $needle" >&2
    exit 1
  fi
}

assert_not_contains() {
  local haystack="$1"
  local needle="$2"
  local message="$3"
  if [[ "$haystack" == *"$needle"* ]]; then
    echo "FAIL: $message" >&2
    echo "unexpected: $needle" >&2
    exit 1
  fi
}

assert_file_exists "$workflow"
content="$(<"$workflow")"

assert_contains "$content" 'workflow_dispatch:' 'workflow must be manually triggered'
assert_contains "$content" 'sourceSha:' 'workflow must accept exact source SHA input'
assert_contains "$content" 'mode:' 'workflow must accept readiness mode input'
assert_contains "$content" 'channel:' 'workflow must accept release channel input'
assert_contains "$content" 'expectedReachableFrom:' 'workflow must accept reachability input'
assert_contains "$content" 'baseBoundary:' 'workflow must accept base boundary input'
assert_contains "$content" 'requestedVersion:' 'workflow must accept requested version input'
assert_contains "$content" 'trackingIssue:' 'workflow must accept tracking issue input'
assert_contains "$content" 'apsItems:' 'workflow must accept APS item allowlist input'
assert_contains "$content" 'retentionDays:' 'workflow must accept retention input'

assert_contains "$content" 'permissions:' 'workflow must declare top-level permissions'
assert_contains "$content" 'contents: read' 'workflow must only need read access to contents'
assert_not_contains "$content" 'contents: write' 'workflow must not have publishing permissions'
assert_not_contains "$content" 'id-token: write' 'workflow must not request OIDC credentials'
assert_not_contains "$content" 'packages: write' 'workflow must not request package publishing permissions'
# Publication-token preflight (post-#3309) needs ANVIL_RELEASES_TOKEN only —
# still no registry/OIDC/publish secrets on the readiness surface.
assert_contains "$content" 'secrets.ANVIL_RELEASES_TOKEN' 'readiness mode must validate the publication token'
if grep -E 'secrets\.[A-Za-z0-9_]+' <<<"$content" | grep -vq 'ANVIL_RELEASES_TOKEN'; then
  echo "FAIL: readiness workflow must not consume secrets other than ANVIL_RELEASES_TOKEN" >&2
  grep -E 'secrets\.[A-Za-z0-9_]+' <<<"$content" >&2 || true
  exit 1
fi

assert_contains "$content" 'ref: ${{ inputs.sourceSha }}' 'workflow must checkout the requested SHA directly'
assert_contains "$content" 'checked_out_sha="$(git rev-parse HEAD)"' 'workflow must read the validated SHA from git'
assert_contains "$content" 'checked out SHA does not match sourceSha' 'workflow must fail on SHA mismatch'
assert_contains "$content" 'git merge-base --is-ancestor' 'workflow must check expected reachability'
assert_contains "$content" 'trackingIssue must be an integer greater than zero when provided' 'workflow must reject malformed tracking issues'

assert_contains "$content" 'release-candidate-metadata.json' 'workflow must emit candidate metadata JSON'
assert_contains "$content" 'artifact-name=release-candidate-metadata-' 'metadata artefact name must be built without YAML folding'
assert_contains "$content" 'lifecycleState' 'metadata must identify candidate lifecycle state'
assert_contains "$content" 'workflowRunUrl' 'metadata must link back to the workflow run'

assert_contains "$content" 'mode must be readiness or candidate-artifacts' 'workflow must reject invalid modes'
assert_contains "$content" 'expectedReachableFrom must be main' 'workflow must reject invalid reachability targets'
assert_not_contains "$content" 'migration-dev' 'migration-dev probe retired with #1419'
assert_contains "$content" 'retention-days: ${{ needs.validate.outputs.retention-days }}' 'metadata retention must use bounded validation output'

# Pre-release packaging + budget gates (moved off routine PR surface).
assert_contains "$content" 'name: cargo-dist plan' 'readiness must run cargo-dist plan on the candidate SHA'
assert_contains "$content" 'dist plan --output-format=json' 'readiness dist plan must emit JSON manifest'
assert_contains "$content" 'uses: ./.github/workflows/resource-budget.yml' 'readiness must call resource-budget reusable workflow'
assert_contains "$content" 'ref: ${{ needs.validate.outputs.checked-out-sha }}' 'resource-budget must pin the validated SHA'
assert_contains "$content" "needs.dist-plan.result == 'success'" 'metadata must require dist-plan success'
assert_contains "$content" "needs.resource-budget.result == 'success'" 'metadata must require resource-budget success'

echo 'release-readiness workflow contract passed'
