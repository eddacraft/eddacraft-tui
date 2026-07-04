#!/usr/bin/env bash
# Validate fast PR workflow invariants that are easy to regress in YAML edits.

set -euo pipefail

repo_root=$(git rev-parse --show-toplevel 2>/dev/null || pwd)
ci_workflow="${repo_root}/.github/workflows/ci.yml"
detect_action="${repo_root}/.github/actions/detect-changes/action.yml"

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

# CIB-137: the classifier script is resolved from the action's own (trusted)
# checkout via $GITHUB_ACTION_PATH, not the PR-head workspace CWD. These pin the
# exact run-block string contract in the composite action.
assert_contains "${detect_action}" 'classifier_script="${GITHUB_ACTION_PATH}/../../../scripts/ci/classify-changes.sh"'
assert_contains "${detect_action}" 'classification=$(bash "${classifier_script}" --context "${CLASSIFIER_CONTEXT}" --paths-file "${paths_file}")'

# CIB-137: structural (not literal-grep) assertions that every workflow whose
# detect-changes job can satisfy the shared "Detect Changes" required context
# resolves the classifier from a TRUSTED ref. A regression that swapped the
# checkout `ref:` back to the PR head, dropped the `main` base-branch filter the
# trust argument depends on, or reordered the steps would fail here — a literal
# grep would not catch those. The classifier logic itself (the ~20 outputs) is
# still exercised by classify-changes.test.sh.
assert_trusted_classifier() {
  local workflow="$1"
  local job="$2"
  NODE_PATH="${repo_root}/node_modules" node "${trusted_checker}" "${workflow}" "${job}"
}

trusted_checker="$(mktemp -d)/trusted-classifier-check.cjs"
cat >"${trusted_checker}" <<'NODE'
const fs = require('node:fs');
const yaml = require('yaml');
const [file, jobName] = process.argv.slice(2);
// The yaml parser normalises the folded multi-line `ref:` scalar to one line.
const EXPECTED_REF =
  "${{ github.event_name == 'pull_request' && github.event.pull_request.base.sha || github.sha }}";
const fail = (m) => {
  console.error(`[${file} / ${jobName}] ${m}`);
  process.exit(1);
};
const doc = yaml.parse(fs.readFileSync(file, 'utf8'));
// The base-SHA trust argument holds ONLY while the PR base is pinned to the
// protected `main` branch — assert the filter is intact.
const pr = doc.on && doc.on.pull_request;
if (!pr) fail('no on.pull_request trigger');
const branches = pr.branches;
if (!Array.isArray(branches) || branches.length !== 1 || branches[0] !== 'main') {
  fail(`on.pull_request.branches must be ['main'], got ${JSON.stringify(branches)}`);
}
const job = doc.jobs && doc.jobs[jobName];
if (!job) fail('job not found');
const steps = Array.isArray(job.steps) ? job.steps : [];
const isCheckout = (s) => typeof s.uses === 'string' && s.uses.startsWith('actions/checkout@');
const trustedIdx = steps.findIndex((s) => s.with && s.with.path === 'trusted-classifier');
if (trustedIdx < 0) fail('no checkout step with `path: trusted-classifier`');
const trusted = steps[trustedIdx];
if (!isCheckout(trusted)) fail('trusted-classifier step is not an actions/checkout');
if (trusted.with.ref !== EXPECTED_REF) {
  fail('trusted checkout with.ref must be the base-SHA conditional, got ' + JSON.stringify(trusted.with.ref));
}
// A primary PR-head checkout (no `path:`) must precede the trusted one so the
// classifier still inspects the PR's own diff.
const primaryIdx = steps.findIndex(
  (s, i) => i < trustedIdx && isCheckout(s) && (!s.with || s.with.path === undefined),
);
if (primaryIdx < 0) fail('no primary PR-head checkout before the trusted checkout');
const invokeIdx = steps.findIndex(
  (s) => s.uses === './trusted-classifier/.github/actions/detect-changes',
);
if (invokeIdx < 0) fail('classifier is not invoked from `./trusted-classifier/...`');
if (trustedIdx >= invokeIdx) fail('trusted checkout must precede the classifier invocation');
console.log(`[${file} / ${jobName}] trusted-classifier structural checks passed`);
NODE

assert_trusted_classifier "${ci_workflow}" detect-changes
assert_trusted_classifier "${repo_root}/.github/workflows/security.yml" detect-changes
assert_trusted_classifier "${repo_root}/.github/workflows/codeql.yml" detect-changes
rm -rf "$(dirname "${trusted_checker}")"

assert_contains "${detect_action}" 'required-checks:'
assert_contains "${detect_action}" 'unit-tests-required:'
assert_contains "${detect_action}" 'echo "unit-tests-required=true" >> "$GITHUB_OUTPUT"'
assert_contains "${detect_action}" 'PR fallback returned no changed files — treating all outputs as changed.'
assert_contains "${detect_action}" 'workflow-lint-required:'
assert_contains "${detect_action}" 'shell-syntax-required:'
assert_contains "${detect_action}" 'dependency-audit-required:'
assert_contains "${detect_action}" 'opa-required:'
assert_contains "${detect_action}" 'regal-required:'
assert_contains "${detect_action}" 'infra-static-check-required:'

assert_contains "${ci_workflow}" "needs.detect-changes.outputs.lint-required == 'true'"
assert_contains "${ci_workflow}" "needs.detect-changes.outputs.typecheck-required == 'true'"
assert_contains "${ci_workflow}" "needs.detect-changes.outputs.unit-tests-required == 'true'"
assert_contains "${ci_workflow}" "needs.detect-changes.outputs.workflow-lint-required == 'true'"
assert_contains "${ci_workflow}" "needs.detect-changes.outputs.shell-syntax-required == 'true'"
assert_contains "${ci_workflow}" "needs.detect-changes.outputs.dependency-audit-required == 'true'"
assert_contains "${ci_workflow}" "needs.detect-changes.outputs.opa-required == 'true'"
assert_contains "${ci_workflow}" "needs.detect-changes.outputs.regal-required == 'true'"
assert_contains "${ci_workflow}" "needs.detect-changes.outputs.infra-static-check-required == 'true'"
assert_contains "${ci_workflow}" 'const yaml = require("yaml")'
assert_contains "${ci_workflow}" "git ls-files 'scripts/*.sh' 'scripts/**/*.sh'"
assert_contains "${ci_workflow}" 'aquasecurity/trivy-action@ed142fd0673e97e23eac54620cfb913e5ce36c25'
assert_contains "${ci_workflow}" 'bash -n scripts/release/*.sh'
assert_contains "${ci_workflow}" 'pnpm --filter @eddacraft/anvil-checks-native build:debug'
assert_contains "${ci_workflow}" "needs.detect-changes.result != 'success'"
assert_contains "${ci_workflow}" "needs.detect-changes.outputs.lint-required != 'true'"
assert_contains "${ci_workflow}" 'pnpm exec nx affected -t test --exclude=@eddacraft/anvil-e2e --exclude=@eddacraft/anvil-checks-native "${RUST_EXCLUDES[@]}"'
assert_contains "${ci_workflow}" 'pnpm exec nx run-many -t test --exclude=@eddacraft/anvil-e2e --exclude=@eddacraft/anvil-checks-native "${RUST_EXCLUDES[@]}"'
# CICD-006: PR/integration runs do not invoke coverage flags or upload coverage artefacts.
assert_not_contains "${ci_workflow}" '--coverage --coverage.reporter=json-summary --coverage.reporter=text'
assert_not_contains "${ci_workflow}" 'name: coverage-report-22.x'

# CIB-156: structural (not literal-grep) assertions that the fail-closed
# classifier guard applied in CIB-137 is present — with the correct NESTING —
# on the two residual consumer jobs (`test-release-gate`, `build`). The guard is
# only sound if the fail-closed clause is AND'd AFTER the job's original
# trigger-context, i.e. `always() && (<trigger>) && (result != 'success' ||
# <source>)`. Substring checks cannot pin that: a regression that OR'd the
# fail-closed clause ACROSS the trigger context (making the heavy cross-platform
# matrix run on ANY feature-PR classifier hiccup) would still contain both
# `always()` and `result != 'success'` as substrings. So we assert a STRICT
# whitespace-normalised template match against the exact expected `if`, built
# from a per-job trigger-context string (stored verbatim here — a deliberate
# change to a job's trigger MUST fail this test and force an update) plus the
# shared fail-closed tail. We also pin the fail-fast first step positionally.
FAILCLOSED_TAIL="( needs.detect-changes.result != 'success' || needs.detect-changes.outputs.source-changed == 'true' )"
# Verbatim original trigger-context of each job (whitespace-normalised form).
# CI-cost 2026-07: the release gate is release-class-PR-only — the
# `push`-to-`main` arm moved to nightly assurance (ci-nightly.yml
# `test-cross-platform`), so per-merge runs no longer pay the macOS (10x)
# + Windows (2x) matrix.
TRELEASE_TRIGGER="( github.event_name == 'pull_request' && github.base_ref == 'main' && github.event.pull_request.head.repo.full_name == github.repository && (startsWith(github.head_ref, 'release/') || startsWith(github.head_ref, 'hotfix/')) )"
BUILD_TRIGGER="github.event_name != 'pull_request'"
EXPECTED_RELEASE_IF="always() && ${TRELEASE_TRIGGER} && ${FAILCLOSED_TAIL}"
EXPECTED_BUILD_IF="always() && ${BUILD_TRIGGER} && ${FAILCLOSED_TAIL}"

assert_fail_closed_guard() {
  local workflow="$1"
  local job="$2"
  local expected_if="$3"
  NODE_PATH="${repo_root}/node_modules" node "${guard_checker}" "${workflow}" "${job}" "${expected_if}"
}

guard_checker_dir="$(mktemp -d)"
trap 'rm -rf "${guard_checker_dir}"' EXIT
guard_checker="${guard_checker_dir}/fail-closed-guard-check.cjs"
cat >"${guard_checker}" <<'NODE'
const fs = require('node:fs');
const yaml = require('yaml');
const [file, jobName, expectedIf] = process.argv.slice(2);
const RESULT_GUARD = "needs.detect-changes.result != 'success'";
const norm = (s) => String(s).replace(/\s+/g, ' ').trim();
const fail = (m) => {
  console.error(`[${file} / ${jobName}] ${m}`);
  process.exit(1);
};
const doc = yaml.parse(fs.readFileSync(file, 'utf8'));
const job = doc.jobs && doc.jobs[jobName];
if (!job) fail('job not found');
if (typeof job.if !== 'string') fail('job `if` is not a scalar string');
// STRICT template match: pins `always() && (<trigger>) && (<fail-closed>)`.
// This rejects an OR-across regression (fail-closed clause hoisted into the
// trigger context) that independent substring checks would pass.
const actual = norm(job.if);
const expected = norm(expectedIf);
if (actual !== expected) {
  fail(`job \`if\` does not match the expected fail-closed template.\n  expected: ${expected}\n  actual:   ${actual}`);
}
// Positionally pin the fail-fast first step (short-circuits before the
// unguarded checkout/setup on a failed classifier; runs per matrix leg).
const steps = Array.isArray(job.steps) ? job.steps : [];
const first = steps[0];
if (!first) fail('job has no steps');
if (first.name !== 'Fail when change detection failed') {
  fail(`first step must be the fail-fast guard, got ${JSON.stringify(first.name)}`);
}
if (typeof first.if !== 'string' || norm(first.if) !== RESULT_GUARD) {
  fail(`fail-fast step must be conditioned on \`${RESULT_GUARD}\`, got ${JSON.stringify(first.if)}`);
}
if (typeof first.run !== 'string' || first.run.trim() !== 'exit 1') {
  fail(`fail-fast step must \`run: exit 1\`, got ${JSON.stringify(first.run)}`);
}
console.log(`[${file} / ${jobName}] fail-closed guard structural checks passed`);
NODE

assert_fail_closed_guard "${ci_workflow}" test-release-gate "${EXPECTED_RELEASE_IF}"
assert_fail_closed_guard "${ci_workflow}" build "${EXPECTED_BUILD_IF}"

echo 'fast PR validation workflow checks passed'
