#!/usr/bin/env bash
# Fixture tests for the CICD local validation command surface.

set -euo pipefail

script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
validator="${script_dir}/local.sh"

if ! command -v jq >/dev/null 2>&1; then
  echo "jq is required" >&2
  exit 2
fi

tmp_dir=$(mktemp -d)
cleanup() {
  rm -rf "${tmp_dir}"
}
trap cleanup EXIT

paths_file="${tmp_dir}/changed.paths"
printf '%s\n' \
  'docs/guides/testing.md' \
  'packages/anvil-core/src/index.ts' \
  'crates/anvil-cli/src/main.rs' \
  'policies/fixtures/security.rego' \
  'scripts/validate/local.sh' \
  >"${paths_file}"

plan=$(bash "${validator}" --changed --paths-file "${paths_file}" --dry-run --json)

jq -e '.mode == "changed"' >/dev/null <<<"${plan}"
jq -e '.classification.pathClasses | index("docs")' >/dev/null <<<"${plan}"
jq -e '.classification.pathClasses | index("ts")' >/dev/null <<<"${plan}"
jq -e '.classification.pathClasses | index("rust")' >/dev/null <<<"${plan}"
jq -e '.classification.pathClasses | index("policy")' >/dev/null <<<"${plan}"
jq -e '.commands | index("pnpm format:check")' >/dev/null <<<"${plan}"
jq -e '.commands | index("pnpm lint:check")' >/dev/null <<<"${plan}"
jq -e '.commands | index("pnpm typecheck")' >/dev/null <<<"${plan}"
jq -e '.commands | index("pnpm test")' >/dev/null <<<"${plan}"
jq -e '.commands | index("pnpm test:ci-classify")' >/dev/null <<<"${plan}"
jq -e '.commands | index("pnpm test:ci-cost")' >/dev/null <<<"${plan}"
jq -e '.commands | index("pnpm test:validate-local")' >/dev/null <<<"${plan}"
jq -e '.commands | index("cargo test --workspace")' >/dev/null <<<"${plan}"
jq -e '.commands | index("opa test --verbose policies/fixtures/")' >/dev/null <<<"${plan}"

full=$(bash "${validator}" --full --dry-run --json)
jq -e '.mode == "full"' >/dev/null <<<"${full}"
jq -e '.commands | index("pnpm format:check")' >/dev/null <<<"${full}"
jq -e '.commands | index("pnpm lint:check")' >/dev/null <<<"${full}"
jq -e '.commands | index("pnpm typecheck")' >/dev/null <<<"${full}"
jq -e '.commands | index("pnpm test")' >/dev/null <<<"${full}"
jq -e '.commands | index("cargo test --workspace")' >/dev/null <<<"${full}"
jq -e '.commands | index("opa test --verbose policies/fixtures/")' >/dev/null <<<"${full}"

if bash "${validator}" --changed --paths-file "${tmp_dir}/missing.paths" --dry-run --json >/dev/null 2>&1; then
  echo 'expected missing paths file to fail' >&2
  exit 1
fi

echo 'local validation fixtures passed'
