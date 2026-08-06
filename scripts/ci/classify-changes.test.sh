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

# DEVENV-007: TS source is E2E-impacting (the harness builds the anvil-api
# closure), so any `ts` change must require the `e2e` surface. This closes the
# path-gate that let a TS source break (e.g. observability) skip E2E on its PR.
assert_json_contains "${ts}" '.requiredChecks | index("e2e")' 'TS source requires the e2e surface'

ts_only=$(run_case ts-only packages/anvil-observability/src/tracer.ts)
assert_json_contains "${ts_only}" '.requiredChecks | index("e2e")' 'lone TS source change still requires e2e (observability gap)'

# Harness-internal non-TS files (package.json, tsconfig) route to the `e2e`
# class directly so editing the harness still requires the e2e check without
# falling through to the conservative `unknown` bucket.
e2e_harness=$(run_case e2e-harness apps/e2e/package.json playwright.config.ts)
assert_json_contains "${e2e_harness}" '.pathClasses | index("e2e")' 'harness paths route to the e2e class'
assert_json_contains "${e2e_harness}" '.requiredChecks | index("e2e")' 'harness paths require the e2e check'
assert_json_contains "${e2e_harness}" '.pathClasses | index("unknown") == null' 'harness paths do NOT fall through to unknown'

rust=$(run_case rust crates/anvil-cli/src/main.rs Cargo.toml Cargo.lock rust-toolchain.toml dist-workspace.toml)
assert_json_contains "${rust}" '.pathClasses | index("rust")' 'Rust path class'
assert_json_contains "${rust}" '.requiredChecks | index("cargo-check")' 'Rust requires cargo check'
assert_json_contains "${rust}" '.requiredChecks | index("cargo-test")' 'Rust requires cargo test'

policy=$(run_case policy policies/fixtures/security.rego)
assert_json_contains "${policy}" '.pathClasses | index("policy")' 'policy path class'
assert_json_contains "${policy}" '.requiredChecks | index("opa-test")' 'policy requires OPA tests'

release=$(run_case release scripts/release/tag.sh .changeset/example.md)
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

# CIB-277: git hooks carry no `.sh` suffix, so they classified as `unknown` —
# which requires format/lint/typecheck/unit-tests but NOT `script-fixtures`.
# Editing the pre-commit gate therefore skipped the fixtures that exercise it.
hooks=$(run_case hooks .husky/pre-commit)
assert_json_contains "${hooks}" '.pathClasses | index("shell")' 'husky hook is a shell change'
assert_json_contains "${hooks}" '.requiredChecks | index("script-fixtures")' 'husky hook requires script fixtures'
assert_json_contains "${hooks}" '.pathClasses | index("unknown") | not' 'husky hook is classified'

# DEVENV-010: engines and CONTRIBUTING state the toolchain floors together and
# drifted apart. Both sides must run the fixture that compares them — and
# neither should acquire an operations review for doing so, or every dependency
# bump would need one.
for tc_path in package.json CONTRIBUTING.md; do
  tc=$(run_case "toolchain-${tc_path%%.*}" "${tc_path}")
  assert_json_contains "${tc}" '.pathClasses | index("toolchain-contract")' "${tc_path} is a toolchain-contract change"
  assert_json_contains "${tc}" '.requiredChecks | index("script-fixtures")' "${tc_path} requires script fixtures"
  assert_json_contains "${tc}" '.requiredReviews | index("operations") | not' "${tc_path} needs no operations review"
done

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

# CIB-031: scope the dependency-audit gate so Rust-only lockfile changes
# skip the npm Trivy audit. The classifier contract is a three-row truth
# table: Rust-only → cargo-deny + no dependency-audit; npm-only →
# dependency-audit; mixed → both. Rust lockfile/manifest paths already
# route to the `rust` class (cargo-deny lives in .github/workflows/rust.yml).

# (a) Cargo.lock-only — routes to rust audit, must NOT add dependency-audit.
cargo_lock_only=$(run_case cargo-lock-only Cargo.lock)
assert_json_contains "${cargo_lock_only}" '.pathClasses | index("rust")' 'Cargo.lock routes to rust class'
assert_json_contains "${cargo_lock_only}" '.pathClasses | index("lockfile") == null' 'Cargo.lock does NOT route to lockfile class (CIB-031)'
assert_json_contains "${cargo_lock_only}" '.requiredChecks | index("dependency-audit") == null' 'Cargo.lock does NOT add dependency-audit (CIB-031)'
assert_json_contains "${cargo_lock_only}" '.requiredChecks | index("cargo-check")' 'Cargo.lock still requires cargo-check'
assert_json_contains "${cargo_lock_only}" '.warnings | index("mixed-change-set") == null' 'Cargo.lock alone is not a mixed-change-set'

# (b) pnpm-lock.yaml-only — still adds dependency-audit.
npm_lock_only=$(run_case npm-lock-only pnpm-lock.yaml)
assert_json_contains "${npm_lock_only}" '.pathClasses | index("lockfile")' 'pnpm-lock.yaml routes to lockfile class'
assert_json_contains "${npm_lock_only}" '.requiredChecks | index("dependency-audit")' 'pnpm-lock.yaml adds dependency-audit'

# (c) Mixed Cargo.lock + pnpm-lock.yaml — Rust-only suppression must NOT
# silence the npm audit when npm also changed.
mixed_lockfiles=$(run_case mixed-lockfiles Cargo.lock pnpm-lock.yaml)
assert_json_contains "${mixed_lockfiles}" '.pathClasses | index("rust")' 'mixed adds rust class for Cargo.lock'
assert_json_contains "${mixed_lockfiles}" '.pathClasses | index("lockfile")' 'mixed adds lockfile class for pnpm-lock.yaml'
assert_json_contains "${mixed_lockfiles}" '.requiredChecks | index("dependency-audit")' 'mixed still adds dependency-audit (npm presence wins)'

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

# CIB-041: agent-tooling config dirs (.codex / .claude / .opencode) carry no
# TypeScript or Rust source, so a pure agent-config change must NOT fall through
# to the conservative `unknown` fallback and force the unit-test / typecheck
# matrix. These dirs are oxfmt-excluded (their skill files embed copy/paste
# ```markdown fences that the formatter reflows and corrupts — see
# .prettierignore), so they carry no automated build/test/format gate; an
# operations review covers the agent-execution config surface.
agent_config=$(run_case agent-config .codex/config.toml)
assert_json_contains "${agent_config}" '.pathClasses | index("agent-config")' 'codex config routes to agent-config class'
assert_json_contains "${agent_config}" '.pathClasses | index("unknown") == null' 'codex config does NOT fall through to unknown'
assert_json_contains "${agent_config}" '.requiredChecks | index("unit-tests") == null' 'agent-config does NOT require unit tests'
assert_json_contains "${agent_config}" '.requiredChecks | index("typecheck") == null' 'agent-config does NOT require typecheck'
assert_json_contains "${agent_config}" '.requiredChecks | index("format") == null' 'agent-config does NOT require format (oxfmt-excluded)'
assert_json_contains "${agent_config}" '.requiredReviews | index("operations")' 'agent-config requires operations review'

# Real PR shape: codex config + codex skill markdown + plans/docs bookkeeping.
# Skill/docs markdown still routes to `docs` (markdownlint), but the set as a
# whole must not require unit tests.
codex_pr=$(run_case codex-pr .codex/config.toml .codex/skills/dev-workflow/SKILL.md plans/index.aps.md docs/guides/agent-surface-inventory.md)
assert_json_contains "${codex_pr}" '.pathClasses | index("agent-config")' 'codex PR includes agent-config class'
assert_json_contains "${codex_pr}" '.pathClasses | index("docs")' 'codex PR includes docs class'
assert_json_contains "${codex_pr}" '.pathClasses | index("unknown") == null' 'codex PR does NOT include unknown class'
assert_json_contains "${codex_pr}" '.requiredChecks | index("unit-tests") == null' 'codex bookkeeping PR does NOT require unit tests'
assert_json_contains "${codex_pr}" '.requiredChecks | index("markdownlint")' 'codex PR still runs markdownlint for skill + docs'

# Sibling agent-tooling dirs (.claude / .opencode) route the same way.
claude_config=$(run_case claude-config .claude/settings.json .opencode/agents/example.md)
assert_json_contains "${claude_config}" '.pathClasses | index("agent-config")' 'claude/opencode tooling routes to agent-config class'
assert_json_contains "${claude_config}" '.requiredChecks | index("unit-tests") == null' 'claude/opencode tooling does NOT require unit tests'

# Main push shape from f322522d3: a tracked agent skill plus the .gitignore
# allowlist line needed to keep that skill in the repo. The skill markdown still
# gets markdownlint through the docs class, but .gitignore must not fall through
# to unknown and trigger the full Node build/test matrix.
agent_skill_allowlist=$(run_case agent-skill-allowlist .claude/skills/anvil-opportunity-assessment/SKILL.md .gitignore)
assert_json_contains "${agent_skill_allowlist}" '.pathClasses | index("docs")' 'agent skill allowlist includes docs class'
assert_json_contains "${agent_skill_allowlist}" '.pathClasses | index("agent-config")' 'agent skill allowlist includes agent-config class'
assert_json_contains "${agent_skill_allowlist}" '.pathClasses | index("repo-metadata")' 'agent skill allowlist includes repo-metadata class'
assert_json_contains "${agent_skill_allowlist}" '.pathClasses | index("unknown") == null' 'agent skill allowlist does NOT include unknown class'
assert_json_contains "${agent_skill_allowlist}" '.requiredChecks | index("markdownlint")' 'agent skill allowlist still runs markdownlint'
assert_json_contains "${agent_skill_allowlist}" '.requiredChecks | index("unit-tests") == null' 'agent skill allowlist does NOT require unit tests'
assert_json_contains "${agent_skill_allowlist}" '.requiredChecks | index("typecheck") == null' 'agent skill allowlist does NOT require typecheck'

empty=$(run_case empty)
assert_json_contains "${empty}" '.pathClasses == []' 'empty path set has no path classes'
assert_json_contains "${empty}" '.warnings == ["no-changed-paths"]' 'empty path set warns no changed paths'

echo 'classify-changes fixtures passed'
