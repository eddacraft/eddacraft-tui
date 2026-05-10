#!/usr/bin/env bash
# Fixture tests for the CICD shared path/risk classifier contract.

set -euo pipefail

script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
classifier="${script_dir}/classify-changes.sh"

if ! command -v jq >/dev/null 2>&1; then
  echo "jq is required" >&2
  exit 2
fi

tmp_dir=$(mktemp -d)
cleanup() {
  rm -rf "${tmp_dir}"
}
trap cleanup EXIT

run_case() {
  local name="$1"
  shift
  local paths_file="${tmp_dir}/${name}.paths"
  printf '%s\n' "$@" >"${paths_file}"
  bash "${classifier}" --context pr --paths-file "${paths_file}"
}

assert_json_contains() {
  local json="$1"
  local filter="$2"
  local message="$3"
  if ! jq -e "${filter}" >/dev/null <<<"${json}"; then
    echo "FAIL: ${message}" >&2
    echo "JSON: ${json}" >&2
    exit 1
  fi
}

docs=$(run_case docs docs/guides/release-runbook.md README.md)
assert_json_contains "${docs}" '.pathClasses == ["docs"]' 'docs-only path class'
assert_json_contains "${docs}" '.riskClasses == ["docs-only"]' 'docs-only risk class'
assert_json_contains "${docs}" '.requiredChecks | index("markdownlint")' 'docs require markdownlint'

ts=$(run_case ts packages/anvil-core/src/index.ts apps/e2e/src/smoke.e2e.test.ts)
assert_json_contains "${ts}" '.pathClasses | index("ts")' 'TS path class'
assert_json_contains "${ts}" '.requiredChecks | index("typecheck")' 'TS requires typecheck'
assert_json_contains "${ts}" '.requiredChecks | index("unit-tests")' 'TS requires unit tests'

rust=$(run_case rust crates/anvil-cli/src/main.rs Cargo.toml Cargo.lock rust-toolchain.toml dist-workspace.toml)
assert_json_contains "${rust}" '.pathClasses | index("rust")' 'Rust path class'
assert_json_contains "${rust}" '.requiredChecks | index("cargo-check")' 'Rust requires cargo check'
assert_json_contains "${rust}" '.requiredChecks | index("cargo-test")' 'Rust requires cargo test'

policy=$(run_case policy policies/fixtures/security.rego)
assert_json_contains "${policy}" '.pathClasses | index("policy")' 'policy path class'
assert_json_contains "${policy}" '.requiredChecks | index("opa-test")' 'policy requires OPA tests'

release=$(run_case release scripts/release.sh .changeset/example.md)
assert_json_contains "${release}" '.pathClasses | index("release")' 'release path class'
assert_json_contains "${release}" '.riskClasses | index("release")' 'release risk class'
assert_json_contains "${release}" '.requiredReviews | index("release")' 'release review required'

workflow=$(run_case workflow .github/workflows/ci.yml .github/actions/setup/action.yml)
assert_json_contains "${workflow}" '.pathClasses | index("workflow")' 'workflow path class'
assert_json_contains "${workflow}" '.requiredReviews | index("operations")' 'operations review required'

shell=$(run_case shell scripts/ci/classify-changes.sh scripts/validate/local.sh)
assert_json_contains "${shell}" '.pathClasses | index("shell")' 'shell path class'
assert_json_contains "${shell}" '.riskClasses | index("automation")' 'automation risk class'
assert_json_contains "${shell}" '.requiredChecks | index("shell-syntax")' 'shell syntax required'
assert_json_contains "${shell}" '.requiredChecks | index("script-fixtures")' 'script fixtures required'

infra=$(run_case infra infra/pulumi/Pulumi.yaml deploy/cloudformation/template.yml)
assert_json_contains "${infra}" '.pathClasses | index("infra")' 'infra path class'
assert_json_contains "${infra}" '.riskClasses | index("infra")' 'infra risk class'
assert_json_contains "${infra}" '.requiredChecks | index("infra-static-check")' 'infra static check required'

napi=$(run_case napi packages/anvil-checks-native/native/src/lib.rs packages/anvil-checks-native/npm/linux-x64/package.json)
assert_json_contains "${napi}" '.pathClasses | index("napi")' 'NAPI path class'
assert_json_contains "${napi}" '.riskClasses | index("platform")' 'NAPI platform risk class'

lockfile=$(run_case lockfile pnpm-lock.yaml Cargo.lock)
assert_json_contains "${lockfile}" '.pathClasses | index("lockfile")' 'lockfile path class'
assert_json_contains "${lockfile}" '.riskClasses | index("dependencies")' 'dependency risk class'
assert_json_contains "${lockfile}" '.requiredChecks | index("dependency-audit")' 'dependency audit required'

mixed=$(run_case mixed docs/guides/testing.md packages/anvil-core/src/index.ts crates/anvil-cli/src/main.rs)
assert_json_contains "${mixed}" '.pathClasses | index("mixed")' 'mixed path class'
assert_json_contains "${mixed}" '.warnings | index("mixed-change-set")' 'mixed warning emitted'

unknown=$(run_case unknown nx.json)
assert_json_contains "${unknown}" '.pathClasses | index("unknown")' 'unknown path class'
assert_json_contains "${unknown}" '.requiredChecks | index("typecheck")' 'unknown fails closed with typecheck'
assert_json_contains "${unknown}" '.requiredReviews | index("operations")' 'unknown requires operations review'

mixed_unknown=$(run_case mixed_unknown README.md nx.json)
assert_json_contains "${mixed_unknown}" '.pathClasses | index("docs")' 'mixed unknown includes docs class'
assert_json_contains "${mixed_unknown}" '.pathClasses | index("unknown")' 'mixed unknown includes unknown class'
assert_json_contains "${mixed_unknown}" '.warnings | index("unclassified-paths")' 'mixed unknown warns'
assert_json_contains "${mixed_unknown}" '.riskClasses | index("docs-only") | not' 'mixed unknown is not docs-only'

empty=$(run_case empty)
assert_json_contains "${empty}" '.pathClasses == []' 'empty path set has no path classes'
assert_json_contains "${empty}" '.warnings == ["no-changed-paths"]' 'empty path set warns no changed paths'

echo 'classify-changes fixtures passed'
