#!/usr/bin/env bash
# CIB-044: release-sign-artefacts signs CLI releases only.
#
# Library releases such as eddacraft-tui-v* are non-prerelease GitHub Releases
# on anvil-001, but they do not carry installer/provenance assets. The signing
# job must therefore require the CLI tag convention (starts with `v`) instead of
# running on every non-prerelease release. Prerelease CLI tags such as v0.x-beta
# still sign because they share the CLI `v` prefix.

set -euo pipefail

repo_root=$(git rev-parse --show-toplevel 2>/dev/null || pwd)
workflow="${repo_root}/.github/workflows/release-sign-artefacts.yml"
release_workflow="${repo_root}/.github/workflows/release.yml"

for required_workflow in "${workflow}" "${release_workflow}"; do
  if [ ! -f "${required_workflow}" ]; then
    echo "expected ${required_workflow} to exist" >&2
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

assert_step_order() {
  local first="$1"
  local second="$2"
  local first_line second_line
  first_line=$(grep -nF -- "- name: ${first}" "${workflow}" | head -n 1 | cut -d: -f1)
  second_line=$(grep -nF -- "- name: ${second}" "${workflow}" | head -n 1 | cut -d: -f1)
  [ -n "${first_line}" ] && [ -n "${second_line}" ] && [ "${first_line}" -lt "${second_line}" ] || {
    echo "expected workflow step '${first}' before '${second}'" >&2
    exit 1
  }
}

assert_release_contains() {
  local expected="$1"
  if ! grep -Fq -- "${expected}" "${release_workflow}"; then
    echo "expected ${release_workflow} to contain: ${expected}" >&2
    exit 1
  fi
}

assert_release_step_blocking() {
  local step_name="$1"
  local block
  block=$(awk -v target="      - name: ${step_name}" '
    $0 == target { capture = 1 }
    capture && $0 != target && $0 ~ /^      - name:/ { exit }
    capture { print }
  ' "${release_workflow}")
  [ -n "$block" ] || {
    echo "expected ${release_workflow} to contain step: ${step_name}" >&2
    exit 1
  }
  if grep -Fq -- "continue-on-error: true" <<< "$block"; then
    echo "expected ${step_name} to remain blocking" >&2
    exit 1
  fi
}

assert_contains "github.event_name == 'workflow_dispatch' &&"
assert_contains "startsWith(github.event.inputs.tag, 'v')"
assert_contains "github.event.workflow_run.conclusion =="
assert_contains "github.event.workflow_run.event =="
assert_contains "actions: read"
assert_contains "startsWith(github.event.workflow_run.head_branch, 'v')"
assert_contains "TAG: \${{ github.event.inputs.tag || github.event.workflow_run.head_branch }}"
assert_contains "RELEASE_RUN_ID: \${{ github.event.inputs.run_id || github.event.workflow_run.id }}"
assert_contains 'gh api "repos/${GITHUB_REPOSITORY}/commits/${TAG}" --jq '\''.sha'\'''
assert_contains 'actions/runs/${RELEASE_RUN_ID}'
assert_contains 'if [ "$tag_sha" != "$commit_sha" ]'
assert_contains "scripts/release/validate-signing-inputs.sh"
assert_contains "eddacraft-anvil-installer.sh.minisig"
assert_contains "eddacraft-anvil-installer.ps1.minisig"
assert_contains 'anvil-${TAG}-provenance.json.minisig'
assert_contains "steps.release.outputs.commit_sha"
assert_contains "Validate decoded minisign secret-key structure"
assert_contains "scripts/release/normalise-minisign-secret-key.sh"
assert_contains "ANVIL_MINISIGN_PRIVATE_KEY is not valid base64"
assert_contains "-W \\"
assert_not_contains 'echo "" | rsign sign'
assert_step_order "Download release assets" "Validate decoded minisign secret-key structure"
assert_contains "Mirror .minisig files to the public release"
assert_contains "--repo eddacraft/anvil"
assert_not_contains 'if [ "${size}" -lt 200 ]'
assert_not_contains '^untrusted\ comment:.*secret\ key$'
assert_not_contains "--json targetCommitish"
assert_not_contains "!github.event.release.prerelease &&"

assert_release_contains 'bash scripts/release/publish-public-contents.sh'
assert_release_contains '--path ACKNOWLEDGEMENTS.md'
assert_release_contains '--path "$DEST"'
assert_release_step_blocking "Publish ACKNOWLEDGEMENTS.md to eddacraft/anvil"
assert_release_step_blocking "Publish release evidence to eddacraft/anvil"
assert_release_contains '::error title=Missing ACKNOWLEDGEMENTS.md'
assert_release_contains '::error title=Missing release evidence'

# CI de-bloat: cargo-dist publish is tag-only; PR dry-run lives in release-readiness.
if grep -E '^[[:space:]]*pull_request:' "${release_workflow}" >/dev/null; then
  echo "expected ${release_workflow} not to declare a pull_request trigger" >&2
  exit 1
fi
assert_release_contains "push:"
assert_release_contains "tags:"

printf 'release-sign-artefacts-workflow.test.sh: ok\n'
