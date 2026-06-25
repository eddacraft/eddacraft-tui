#!/usr/bin/env bash
# Lock rust nightly coverage collection invariants (CICD-006).

set -euo pipefail

repo_root=$(git rev-parse --show-toplevel 2>/dev/null || pwd)
ci_nightly="${repo_root}/.github/workflows/ci-nightly.yml"
script="${repo_root}/scripts/ci/rust-coverage.sh"

assert_contains() {
  local file="$1"
  local expected="$2"
  if [ ! -f "${file}" ]; then
    echo "expected file not found: ${file} (run from within the git checkout)" >&2
    exit 1
  fi
  if ! grep -Fq -- "${expected}" "${file}"; then
    echo "expected ${file} to contain: ${expected}" >&2
    exit 1
  fi
}

assert_contains "${ci_nightly}" 'scripts/ci/rust-coverage.sh'
assert_contains "${ci_nightly}" 'shared-key: rust-coverage'
assert_contains "${script}" 'cargo llvm-cov clean --workspace'
assert_contains "${script}" 'cargo llvm-cov --no-report nextest --workspace --test-threads 1'
assert_contains "${script}" '--test-threads 1'
assert_contains "${script}" 'cargo llvm-cov report --summary-only --output-path'

if [ ! -x "${script}" ]; then
  echo "expected ${script} to be executable" >&2
  exit 1
fi

echo 'rust coverage workflow checks passed'