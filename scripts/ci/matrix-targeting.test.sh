#!/usr/bin/env bash
# CICD-008: lock the matrix-targeting contract.
#
# Platform matrices (macOS, Windows, cross-compile, NAPI) only run when
# the change is platform-sensitive, the workflow is a release-gate, or
# an operator dispatches the workflow explicitly. This fixture asserts
# the YAML still encodes that gating.

set -euo pipefail

repo_root=$(git rev-parse --show-toplevel 2>/dev/null || pwd)
ci_workflow="${repo_root}/.github/workflows/ci.yml"
rust_workflow="${repo_root}/.github/workflows/rust.yml"
napi_workflow="${repo_root}/.github/workflows/napi.yml"
bench_workflow="${repo_root}/.github/workflows/bench.yml"
ci_nightly_workflow="${repo_root}/.github/workflows/ci-nightly.yml"

assert_contains() {
  local file="$1"
  local expected="$2"
  if ! grep -Fq -- "${expected}" "${file}"; then
    echo "expected ${file} to contain: ${expected}" >&2
    exit 1
  fi
}

assert_block_contains() {
  local file="$1"
  local block_marker="$2"
  local expected="$3"
  if ! awk -v marker="${block_marker}" -v expected="${expected}" '
    $0 ~ marker { inside = 1; next }
    inside && /^  [a-z]/ { inside = 0 }
    inside && $0 ~ expected { found = 1 }
    END { exit (found ? 0 : 1) }
  ' "${file}"; then
    echo "expected block matching '${block_marker}' in ${file} to contain: ${expected}" >&2
    exit 1
  fi
}

assert_block_not_contains() {
  local file="$1"
  local block_marker="$2"
  local forbidden="$3"
  if awk -v marker="${block_marker}" -v forbidden="${forbidden}" '
    $0 ~ marker { inside = 1; next }
    inside && /^  [a-z]/ { inside = 0 }
    inside && $0 ~ forbidden { found = 1 }
    END { exit (found ? 0 : 1) }
  ' "${file}"; then
    echo "expected block matching '${block_marker}' in ${file} not to contain: ${forbidden}" >&2
    exit 1
  fi
}

# ── ci.yml: test-release-gate gates on source-changed ───────────
# Docs-only release PRs and docs-only release-sync pushes must not
# spin up the macOS + Windows matrix.
assert_contains "${ci_workflow}" '  test-release-gate:'
assert_block_contains "${ci_workflow}" "^  test-release-gate:" "source-changed == .true."
assert_block_contains "${ci_workflow}" "^  test-release-gate:" "github.base_ref == .main."
assert_block_contains "${ci_workflow}" "^  test-release-gate:" "github.ref == .refs/heads/main."

# ── rust.yml: cross-compile is release-gate-only ────────────────
# - Push to `dev` must NOT trigger cross-compile (dev is the integration
#   branch during migration but not a release gate).
# - Push to `main` and `release/*` must trigger.
# - PR to `main` must trigger.
# - `workflow_dispatch` is the operator override.
# - The job must still gate on rust-changed when not dispatched.
assert_contains "${rust_workflow}" '  workflow_dispatch: {}'
assert_contains "${rust_workflow}" '  cross-compile:'
assert_block_contains "${rust_workflow}" "^  cross-compile:" "github.event_name == .workflow_dispatch."
assert_block_contains "${rust_workflow}" "^  cross-compile:" "github.ref == .refs/heads/main."
assert_block_contains "${rust_workflow}" "^  cross-compile:" "refs/heads/release/"
assert_block_contains "${rust_workflow}" "^  cross-compile:" "github.base_ref == .main."
assert_block_contains "${rust_workflow}" "^  cross-compile:" "rust-changed == .true."
# Critical: dev pushes must NOT trigger cross-compile. The previous
# gating contained `refs/heads/dev`; the new gating must not.
assert_block_not_contains "${rust_workflow}" "^  cross-compile:" "refs/heads/dev"

# ── napi.yml: path-gated matrix (correct as-is) ─────────────────
# The NAPI binding is inherently platform-sensitive; the workflow-level
# path filter scopes the matrix to napi-related changes. Lock the path
# filter so it cannot regress to broader triggers.
assert_contains "${napi_workflow}" "      - 'crates/anvil-checks-napi/**'"
assert_contains "${napi_workflow}" "    tags:"
assert_contains "${napi_workflow}" "      - 'napi-v*'"

# ── bench.yml: release-gate (push-to-main + dispatch) ───────────
assert_contains "${bench_workflow}" "    branches: [main]"
assert_contains "${bench_workflow}" "  workflow_dispatch:"

# ── ci-nightly.yml: scheduled cross-platform matrix ─────────────
# macOS + Windows Node tests live in nightly assurance, not routine PR
# or integration push.
assert_contains "${ci_nightly_workflow}" "  test-cross-platform:"
assert_contains "${ci_nightly_workflow}" "          - os: macos-latest"
assert_contains "${ci_nightly_workflow}" "          - os: windows-latest"
assert_contains "${ci_nightly_workflow}" "  schedule:"

echo 'matrix-targeting workflow checks passed'
