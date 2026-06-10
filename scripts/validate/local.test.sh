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
jq -e '.commands | index("pnpm test:ci-fast-pr")' >/dev/null <<<"${plan}"
jq -e '.commands | index("pnpm test:validate-local")' >/dev/null <<<"${plan}"
jq -e '.commands[] | select(contains("scripts/validate/local.sh"))' >/dev/null <<<"${plan}"
jq -e '.commands | index("cargo test --workspace")' >/dev/null <<<"${plan}"
jq -e '.commands | index("opa test --verbose policies/fixtures/")' >/dev/null <<<"${plan}"
# DEVENV-007: a TS source change selects the E2E harness surface (mirrors CI).
jq -e '.commands | index("pnpm --filter @eddacraft/anvil-e2e test")' >/dev/null <<<"${plan}"

dependency_paths="${tmp_dir}/dependency.paths"
printf '%s\n' 'pnpm-lock.yaml' >"${dependency_paths}"
dependency_plan=$(bash "${validator}" --changed --paths-file "${dependency_paths}" --dry-run --json)
jq -e '.commands[] | select(contains("trivy is required"))' >/dev/null <<<"${dependency_plan}"

release_paths="${tmp_dir}/release.paths"
printf '%s\n' 'scripts/release/tag.sh' >"${release_paths}"
release_plan=$(bash "${validator}" --changed --paths-file "${release_paths}" --dry-run --json)
jq -e '.commands[] | select(. == "bash -n scripts/release/*.sh")' >/dev/null <<<"${release_plan}"

full=$(bash "${validator}" --full --dry-run --json)
jq -e '.mode == "full"' >/dev/null <<<"${full}"
jq -e '.commands | index("pnpm format:check")' >/dev/null <<<"${full}"
jq -e '.commands | index("pnpm lint:check")' >/dev/null <<<"${full}"
jq -e '.commands | index("pnpm typecheck")' >/dev/null <<<"${full}"
jq -e '.commands | index("pnpm test")' >/dev/null <<<"${full}"
jq -e '.commands | index("pnpm test:ci-classify")' >/dev/null <<<"${full}"
jq -e '.commands | index("pnpm test:ci-cost")' >/dev/null <<<"${full}"
jq -e '.commands | index("pnpm test:ci-fast-pr")' >/dev/null <<<"${full}"
jq -e '.commands | index("pnpm test:validate-local")' >/dev/null <<<"${full}"
jq -e '.commands | index("cargo test --workspace")' >/dev/null <<<"${full}"
jq -e '.commands | index("opa test --verbose policies/fixtures/")' >/dev/null <<<"${full}"
jq -e '.commands | index("regal lint policies/fixtures/")' >/dev/null <<<"${full}"

empty_paths="${tmp_dir}/empty.paths"
: >"${empty_paths}"
empty=$(bash "${validator}" --changed --paths-file "${empty_paths}" --dry-run --json)
jq -e '.commands == []' >/dev/null <<<"${empty}"

dev_null=$(bash "${validator}" --changed --paths-file /dev/null --dry-run --json)
jq -e '.commands == []' >/dev/null <<<"${dev_null}"

agent_shell_paths="${tmp_dir}/agent-shell.paths"
printf '%s\n' 'scripts/agent/guidance.sh' >"${agent_shell_paths}"
agent_shell=$(bash "${validator}" --changed --paths-file "${agent_shell_paths}" --dry-run --json)
jq -e '.commands[] | select(contains("scripts/agent/guidance.sh"))' >/dev/null <<<"${agent_shell}"

mkdir -p "${tmp_dir}/scripts"
printf '%s\n' 'if then' >"${tmp_dir}/scripts/bad.sh"
printf '%s\n' '#!/usr/bin/env bash' 'exit 0' >"${tmp_dir}/scripts/good.sh"
syntax_paths="${tmp_dir}/syntax.paths"
printf '%s\n' 'scripts/bad.sh' 'scripts/good.sh' >"${syntax_paths}"
syntax_plan=$(bash "${validator}" --changed --paths-file "${syntax_paths}" --dry-run --json)
syntax_command=$(jq -r '.commands[] | select(contains("bash -n"))' <<<"${syntax_plan}")
if (cd "${tmp_dir}" && bash -lc "${syntax_command}"); then
  echo 'expected shell syntax command to fail on the first invalid script' >&2
  exit 1
fi

if bash "${validator}" --changed --paths-file "${tmp_dir}/missing.paths" --dry-run --json >/dev/null 2>&1; then
  echo 'expected missing paths file to fail' >&2
  exit 1
fi

echo 'local validation fixtures passed'
