#!/usr/bin/env bash
# CIB-338: pin the fail-open contract on the required `Test` check.
#
# `rust-tests.yml` runs on EVERY pull request and gates its heavy steps
# behind a `dorny/paths-filter` step so docs/config-only PRs get a green
# no-op. On 2026-08-14 the filter itself died on the GitHub `listFiles`
# API (run 31786493631, PR #3888): every downstream step was skipped and
# the REQUIRED check went red on a docs-only PR with zero tests executed.
#
# The fix routes a non-success filter outcome to "run the full work"
# (fail open) — a path-detection outage may cost redundant CI minutes,
# never a false failure. These greps pin both halves of that contract so
# a future edit cannot silently restore fail-closed:
#   1. the filter step carries `continue-on-error: true` (a filter
#      failure must not abort the job before the gate step decides), and
#   2. the gate step consumes `steps.changes.outcome` and treats
#      != 'success' as "run" — the step-level analogue of ci.yml's
#      `needs.detect-changes.result != 'success'` consumers.

set -euo pipefail

repo_root=$(git rev-parse --show-toplevel 2>/dev/null || pwd)
workflow="${repo_root}/.github/workflows/rust-tests.yml"

if [ ! -f "${workflow}" ]; then
  echo "expected ${workflow} to exist" >&2
  exit 1
fi

# 1. The paths-filter step must fail open at the step level. Scope the
# grep to the lines following the `uses:` so the assertion cannot be
# satisfied by `continue-on-error` on an unrelated step (the report-only
# policy eval-regression step also carries one).
if ! grep -A4 -F 'uses: dorny/paths-filter' "${workflow}" | grep -Fq 'continue-on-error: true'; then
  echo "FAIL: the dorny/paths-filter step in rust-tests.yml no longer carries" >&2
  echo "      'continue-on-error: true' — a filter outage would abort the job" >&2
  echo "      and red the required Test check with every step skipped (CIB-338)" >&2
  exit 1
fi
echo "ok: rust-tests.yml paths-filter step carries continue-on-error (CIB-338)"

# 2. The gate step must treat a non-success filter outcome as "run the
# full work". Grep for the outcome check AS A DISJUNCT of the gate's
# routing condition (leading `|| [`) — a bare `steps.changes.outcome`
# grep would also match the advisory ::warning guard, which routes
# nothing, so it could pass with the fail-open routing deleted. This is
# the condition that turns a detection outage into redundant CI minutes
# instead of a false failure.
if ! grep -Fq "|| [ \"\${{ steps.changes.outcome }}\" != 'success' ] ||" "${workflow}"; then
  echo "FAIL: rust-tests.yml gate step no longer fails open on" >&2
  echo "      'steps.changes.outcome != success' — a paths-filter outage" >&2
  echo "      would skip the Rust test gate instead of running it (CIB-338)" >&2
  exit 1
fi
echo "ok: rust-tests.yml gate fails open when the paths-filter outcome is not success (CIB-338)"

echo 'rust-tests fail-open contract checks passed'
