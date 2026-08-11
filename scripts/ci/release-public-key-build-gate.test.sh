#!/usr/bin/env bash
# Contract tests for the DISTRIB-001 release-public-key packaging gate
# (Clawpatch fnd_sig-feat-cli-command-c2cc6bd208-_e6b2eeb4df).
#
# Asserts that release packaging:
#   1. injects ANVIL_RELEASE_PUBLIC_KEY from the repo variable,
#   2. fails closed with ANVIL_REQUIRE_RELEASE_PUBLIC_KEY=1 so a missing or
#      development-fallback key cannot produce a distributable binary, and
#   3. keeps the shell preflight that catches the same failure earlier.

set -euo pipefail

repo_root=$(git rev-parse --show-toplevel 2>/dev/null || pwd)
release_workflow="${repo_root}/.github/workflows/release.yml"
build_rs="${repo_root}/crates/anvil-cli/build.rs"
gate_rs="${repo_root}/crates/anvil-cli/build_support/release_public_key_gate.rs"
dev_key="RWRbilgipcbv8egsndfKxcAxjJCTusQPh/IsOy6ROFDiqvz8QNCVZRZ5"

for required in "${release_workflow}" "${build_rs}" "${gate_rs}"; do
  if [ ! -f "${required}" ]; then
    echo "expected ${required} to exist" >&2
    exit 1
  fi
done

assert_release_contains() {
  local expected="$1"
  if ! grep -Fq -- "${expected}" "${release_workflow}"; then
    echo "expected ${release_workflow} to contain: ${expected}" >&2
    exit 1
  fi
}

assert_file_contains() {
  local path="$1"
  local expected="$2"
  if ! grep -Fq -- "${expected}" "${path}"; then
    echo "expected ${path} to contain: ${expected}" >&2
    exit 1
  fi
}

# Env injection for compile-time embedding.
assert_release_contains "ANVIL_RELEASE_PUBLIC_KEY: \${{ vars.ANVIL_MINISIGN_PUBLIC_KEY }}"
# Packaging hard-fail (mirrors ANVIL_DASHBOARD_REQUIRE_BUNDLE).
assert_release_contains "ANVIL_REQUIRE_RELEASE_PUBLIC_KEY: '1'"
# Pre-build shell preflight still present as defence in depth.
assert_release_contains "DISTRIB-001 preflight: release public key is set and not dev fallback"
assert_release_contains "${dev_key}"

# Build script + pure gate must exist and reference the require flag.
assert_file_contains "${build_rs}" "ANVIL_REQUIRE_RELEASE_PUBLIC_KEY"
assert_file_contains "${build_rs}" "release_public_key_rejection"
assert_file_contains "${gate_rs}" "${dev_key}"
assert_file_contains "${gate_rs}" "is_acceptable_release_public_key"

printf 'release-public-key-build-gate.test.sh: ok\n'
