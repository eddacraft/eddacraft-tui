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

assert_contains "github.event_name == 'workflow_dispatch' ||"
assert_contains "startsWith(github.event.release.tag_name, 'v')"
assert_contains "steps.release.outputs.commit_sha"
assert_contains "targetCommitish"
assert_not_contains "!github.event.release.prerelease &&"

printf 'release-sign-artefacts-workflow.test.sh: ok\n'