#!/usr/bin/env bash
# CIB-396 / issue #4345: lock secret-calibration.yml off LINUX_RUNNER for fork PRs.
# Issue #4391: bind that lock to jobs.calibrate.runs-on, not a whole-file grep.
#
# DeepSec 20260902184224-6753b67df9c072ab found the advisory calibration job
# scheduled on `vars.LINUX_RUNNER` with no fork ternary, so an external PR
# touching secret-calibration paths could run PR-controlled `cargo test` on
# the org Linux runner. rust-tests.yml and codeql.yml already force forks
# onto ubuntu-latest. CIB-396/#4363 put the same ternary on
# jobs.calibrate.runs-on. A file-wide substring check still passed if that
# expression was parked in a comment, unrelated field, or dead job while
# calibrate kept `runs-on: ${{ vars.LINUX_RUNNER }}`. This fixture parses
# the workflow and requires the calibrate job's runs-on value to be exactly
# the fork-safe expression.

set -euo pipefail

repo_root=$(git rev-parse --show-toplevel 2>/dev/null || pwd)
workflow="${repo_root}/.github/workflows/secret-calibration.yml"

if [ ! -f "${workflow}" ]; then
  echo "expected ${workflow} to exist" >&2
  exit 1
fi

tmp_dir=$(mktemp -d)
cleanup() {
  rm -rf "${tmp_dir}"
}
trap cleanup EXIT

# Node + the repo's `yaml` package (same toolchain as fast-pr-validation and
# security-summary-gate) so the test needs no PyYAML and cannot be satisfied
# by a parked substring.
checker="${tmp_dir}/calibrate-runs-on.cjs"
cat >"${checker}" <<'NODE'
const fs = require('node:fs');
const yaml = require('yaml');

const file = process.argv[2];
// YAML folding collapses the workflow's wrapped scalar to this single line.
const expected =
  "${{ github.event.pull_request.head.repo.fork && 'ubuntu-latest' || vars.LINUX_RUNNER || 'ubuntu-latest' }}";
const norm = (s) => String(s).replace(/\s+/g, ' ').trim();
const fail = (m) => {
  console.error(`${file}: ${m}`);
  process.exit(1);
};

const doc = yaml.parse(fs.readFileSync(file, 'utf8'));
const job = doc.jobs && doc.jobs.calibrate;
if (!job) fail('jobs.calibrate not found');
const runsOn = job['runs-on'];
if (typeof runsOn !== 'string') {
  fail(`jobs.calibrate.runs-on is not a scalar string: ${JSON.stringify(runsOn)}`);
}
const actual = norm(runsOn);
const want = norm(expected);
if (actual !== want) {
  fail(
    `jobs.calibrate.runs-on is not the fork-safe LINUX_RUNNER ternary.\n` +
      `  expected: ${want}\n` +
      `  actual:   ${actual}`,
  );
}
console.log(`${file}: jobs.calibrate.runs-on is the fork-safe LINUX_RUNNER ternary`);
NODE

assert_calibrate_runs_on() {
  NODE_PATH="${repo_root}/node_modules" node "${checker}" "$1"
}

assert_calibrate_runs_on "${workflow}"

# Negative: the ternary appears in a comment and a dead job, but calibrate
# still uses the unguarded org runner. Whole-file grep would pass; this must
# not.
negative="${tmp_dir}/parked-ternary.yml"
cat >"${negative}" <<'YAML'
name: parked-ternary
jobs:
  calibrate:
    name: Detection / false-positive report
    # ${{ github.event.pull_request.head.repo.fork && 'ubuntu-latest' || vars.LINUX_RUNNER || 'ubuntu-latest' }}
    runs-on: ${{ vars.LINUX_RUNNER }}
    steps:
      - run: echo decoy
  decoy:
    runs-on: ${{ github.event.pull_request.head.repo.fork && 'ubuntu-latest' || vars.LINUX_RUNNER || 'ubuntu-latest' }}
    steps:
      - run: echo dead
YAML

if assert_calibrate_runs_on "${negative}" 2>"${tmp_dir}/negative.err"; then
  echo "expected parked-ternary fixture to fail when calibrate.runs-on is vars.LINUX_RUNNER" >&2
  exit 1
fi
if ! grep -Fq 'jobs.calibrate.runs-on is not the fork-safe LINUX_RUNNER ternary' \
  "${tmp_dir}/negative.err"; then
  echo "parked-ternary fixture failed for the wrong reason:" >&2
  cat "${tmp_dir}/negative.err" >&2
  exit 1
fi
if ! grep -Fq '${{ vars.LINUX_RUNNER }}' "${tmp_dir}/negative.err"; then
  echo "parked-ternary fixture stderr missing the unguarded vars.LINUX_RUNNER:" >&2
  cat "${tmp_dir}/negative.err" >&2
  exit 1
fi
echo 'ok: parked ternary does not satisfy jobs.calibrate.runs-on'

printf 'secret-calibration-workflow.test.sh: ok\n'
