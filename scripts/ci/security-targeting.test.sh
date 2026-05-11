#!/usr/bin/env bash
# Lock CICD-007 invariants: security workflow jobs are path/risk targeted
# and Rust dependency-only jobs gate on `rust-deps-changed`.

set -euo pipefail

repo_root=$(git rev-parse --show-toplevel 2>/dev/null || pwd)
security_workflow="${repo_root}/.github/workflows/security.yml"
rust_workflow="${repo_root}/.github/workflows/rust.yml"

assert_contains() {
  local file="$1"
  local expected="$2"
  if ! grep -Fq -- "${expected}" "${file}"; then
    echo "expected ${file} to contain: ${expected}" >&2
    exit 1
  fi
}

# security.yml — scheduled assurance and manual dispatch
assert_contains "${security_workflow}" '  schedule:'
assert_contains "${security_workflow}" "    - cron: '15 6 * * 1'"
assert_contains "${security_workflow}" '  workflow_dispatch: {}'

# security.yml — detect-changes skipped on schedule/dispatch
assert_contains "${security_workflow}" "if: github.event_name != 'schedule' && github.event_name != 'workflow_dispatch'"

# security.yml — per-job classifier gates
# Semgrep: TS/JS or Rust source changes (or schedule/dispatch skip path)
assert_contains "${security_workflow}" "needs.detect-changes.outputs.source-changed == 'true' ||"
assert_contains "${security_workflow}" "needs.detect-changes.outputs.rust-changed == 'true'"
# Dependency audit + license: lockfile/manifest signal only
assert_contains "${security_workflow}" "needs.detect-changes.outputs.dependency-audit-required == 'true'"
# Secret scan keeps broad code-changed gate
assert_contains "${security_workflow}" "needs.detect-changes.outputs.code-changed == 'true'"

# rust.yml — cargo-deny + acknowledgements-diff gate on rust-deps-changed
assert_contains "${rust_workflow}" "rust-deps-changed: \${{ steps.filter.outputs.rust-deps }}"
assert_contains "${rust_workflow}" "needs.detect-rust-changes.outputs.rust-deps-changed == 'true'"

echo 'security targeting workflow checks passed'
