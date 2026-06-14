#!/usr/bin/env bash
# Issue #1897 (TUIR-004 follow-ups): lock the mirror-workflow hardening guards.
#
# Both mirror workflows force-push a `git subtree split` of a canonical
# subdirectory to a PUBLIC sibling repo. The four deferred review findings
# from PR #1894 add defensive guards to those force-push paths. This fixture
# asserts each guard is present so a future refactor cannot silently drop one:
#
#   1. `_mirror_split` pre-delete guard — `git branch -D _mirror_split` before
#      the `git subtree split -b _mirror_split` call (idempotent on runner
#      reuse).
#   2. Dry-run mode — a `workflow_dispatch` boolean `dry_run` input that turns
#      the live `git push --force` into a `--dry-run` no-op preflight.
#   3. Token-rotation runbook cross-link in the workflow header comment.
#   4. Banner double-prepend sentinel — the banner-swap step refuses to run if
#      the canonical README already carries the read-only-mirror banner.
#
# The same guards apply to BOTH mirror workflows (the eddacraft-tui crate
# mirror and the acknowledgements-starter kit mirror, ATTRIB-011), and item 4
# must stay aligned with the drift watchdog that replicates the banner swap.

set -euo pipefail

repo_root=$(git rev-parse --show-toplevel 2>/dev/null || pwd)
workflows_dir="${repo_root}/.github/workflows"

tui="${workflows_dir}/mirror-eddacraft-tui.yml"
ack="${workflows_dir}/mirror-acknowledgements-starter.yml"
drift="${workflows_dir}/mirror-drift-check.yml"

assert_contains() {
  local file="$1"
  local expected="$2"
  if ! grep -Fq -- "${expected}" "${file}"; then
    echo "expected ${file} to contain: ${expected}" >&2
    exit 1
  fi
}

assert_followed_by() {
  # Assert the line matching `first` is immediately followed by a line
  # matching `second` in `file`.
  local file="$1"
  local first="$2"
  local second="$3"
  if ! grep -A1 -F -- "${first}" "${file}" | grep -Fq -- "${second}"; then
    echo "expected '${first}' to be immediately followed by '${second}' in ${file}" >&2
    exit 1
  fi
}

for wf in "${tui}" "${ack}"; do
  [ -f "${wf}" ] || { echo "missing workflow: ${wf}" >&2; exit 1; }

  # Item 1: pre-delete guard immediately before the subtree split.
  assert_contains "${wf}" 'git branch -D _mirror_split'
  assert_followed_by "${wf}" 'git branch -D _mirror_split' 'git subtree split'

  # Item 2: dry_run dispatch input + conditional --dry-run on the force-push.
  assert_contains "${wf}" 'dry_run:'
  assert_contains "${wf}" "type: boolean"
  assert_contains "${wf}" 'inputs.dry_run'
  assert_contains "${wf}" '--dry-run'

  # Item 3: token-rotation runbook cross-link in the header.
  assert_contains "${wf}" 'rotation: see docs/runbooks/'

  # Item 4: banner double-prepend sentinel in the banner-swap path.
  assert_contains "${wf}" 'This repository is a read-only mirror.'
  assert_contains "${wf}" 'double-prepend'
done

# Item 3: each workflow points at its OWN runbook target.
assert_contains "${tui}" 'docs/runbooks/eddacraft-tui-release.md'
assert_contains "${ack}" 'docs/runbooks/acknowledgements-starter-release.md'

# Item 4 alignment: the drift watchdog replicates the banner swap, so its
# sentinel guard must stay in lockstep with the push-side workflow.
assert_contains "${drift}" 'This repository is a read-only mirror.'
assert_contains "${drift}" 'double-prepend'

echo 'mirror-workflow hardening checks passed'
