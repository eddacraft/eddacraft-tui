#!/usr/bin/env bash
# Lock CICD-012 invariants: validation workflows survive the
# `dev` → `main` cutover without silently changing meaning.

set -euo pipefail

repo_root=$(git rev-parse --show-toplevel 2>/dev/null || pwd)
ci_workflow="${repo_root}/.github/workflows/ci.yml"
rust_workflow="${repo_root}/.github/workflows/rust.yml"
security_workflow="${repo_root}/.github/workflows/security.yml"
codeql_workflow="${repo_root}/.github/workflows/codeql.yml"
release_readiness="${repo_root}/.github/workflows/release-readiness.yml"
pr_base_guard="${repo_root}/.github/workflows/pr-base-guard.yml"
pr_template="${repo_root}/.github/PULL_REQUEST_TEMPLATE.md"
branching_doc="${repo_root}/docs/guides/branching-strategy.md"
worktree_doc="${repo_root}/docs/guides/worktree-policy.md"

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

# Integration validation workflows must fire on both `dev` and `main`
# so the cutover does not strand them.
assert_contains "${ci_workflow}" '      - main'
assert_contains "${ci_workflow}" '      - dev'
assert_contains "${security_workflow}" '      - main'
assert_contains "${security_workflow}" '      - dev'
assert_contains "${codeql_workflow}" 'branches: [main, dev]'
assert_contains "${rust_workflow}" "branches: [main, dev, 'rust-*', 'release/*']"

# ci.yml release gate must not fire on every base-main PR. The head
# allowlist below stops normal `feat/*` PRs from triggering cross-
# platform after the cutover.
assert_contains "${ci_workflow}" "github.head_ref == 'dev' ||"
assert_contains "${ci_workflow}" "startsWith(github.head_ref, 'release/') ||"
assert_contains "${ci_workflow}" "startsWith(github.head_ref, 'hotfix/'))"
assert_not_contains "${ci_workflow}" "(github.event_name == 'pull_request' && github.base_ref == 'main') ||"

# rust.yml cross-compile must follow the same head allowlist pattern.
assert_contains "${rust_workflow}" "startsWith(github.head_ref, 'release/') ||"
assert_contains "${rust_workflow}" "startsWith(github.head_ref, 'hotfix/')))"
assert_not_contains "${rust_workflow}" "github.ref)) || (github.event_name == 'pull_request' && github.base_ref == 'main')) &&"

# pr-base-guard must self-identify as migration-only so the cutover
# task owner sees the retirement path.
assert_contains "${pr_base_guard}" 'CICD-012: MIGRATION-MODE GUARD'
assert_contains "${pr_base_guard}" 'OPMODEL-012'

# Release readiness must already speak the dual-mode vocabulary.
assert_contains "${release_readiness}" '          - main'
assert_contains "${release_readiness}" '          - migration-dev'

# PR template must name both modes so contributors pick the right
# base branch.
assert_contains "${pr_template}" 'CICD-012'
assert_contains "${pr_template}" 'Migration mode'
assert_contains "${pr_template}" 'Target mode'

# Branching strategy must keep the compatibility and target sections
# both present and add the cutover-aware CI gate table.
assert_contains "${branching_doc}" '## Current Compatibility Model'
assert_contains "${branching_doc}" '## Target Model'
assert_contains "${branching_doc}" '### Cutover-aware CI gates (CICD-012)'

# Worktree policy must also describe both modes (already true via
# OPMODEL-002, but lock it so a future edit cannot collapse it).
assert_contains "${worktree_doc}" '### Current Compatibility Model'
assert_contains "${worktree_doc}" '### Target Model'

echo 'cutover readiness workflow checks passed'
