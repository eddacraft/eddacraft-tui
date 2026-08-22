#!/usr/bin/env bash
# CICD-005: lock the integration push validation contract.
#
# The integration push (push to `dev` during migration, `main` after
# OPMODEL-012) is a distinct contract from PR validation. This fixture
# asserts the YAML still encodes that separation so regressions on the
# trigger gating get caught at fixture-test time.

set -euo pipefail

repo_root=$(git rev-parse --show-toplevel 2>/dev/null || pwd)
ci_workflow="${repo_root}/.github/workflows/ci.yml"
security_workflow="${repo_root}/.github/workflows/security.yml"
detect_action="${repo_root}/.github/actions/detect-changes/action.yml"

assert_contains() {
  local file="$1"
  local expected="$2"
  if ! grep -Fq -- "${expected}" "${file}"; then
    echo "expected ${file} to contain: ${expected}" >&2
    exit 1
  fi
}

assert_not_contains_block() {
  local file="$1"
  local block_marker="$2"
  local forbidden="$3"
  if awk -v marker="${block_marker}" -v forbidden="${forbidden}" '
    $0 ~ marker { inside = 1; next }
    inside && /^  [a-z]/ { inside = 0 }
    inside && $0 ~ forbidden { found = 1 }
    END { exit (found ? 0 : 1) }
  ' "${file}"; then
    echo "expected block after '${block_marker}' in ${file} not to contain: ${forbidden}" >&2
    exit 1
  fi
}

# ── PR quick-skip paths inside primary required-check jobs ───────
# The primary jobs (lint, typecheck, test, docs-lint) now internally
# provide the required-check-name conclusion on *every* PR via a cheap
# early "filler" step when their work is not required (docs-only etc).
# This guarantees exactly one conclusion per name (no success+skipped
# duplicates from twin jobs). The quick-skip step is gated to PR only.
# On push the primary jobs provide "skipped" where appropriate (accepted
# by integration-readiness). CIB-038 consolidated the old CICD-005 twins.
# No separate *-skip: job ids remain.
assert_not_contains_block "${ci_workflow}" "^  lint-skip:" 'github.event_name'
assert_not_contains_block "${ci_workflow}" "^  typecheck-skip:" 'github.event_name'
assert_not_contains_block "${ci_workflow}" "^  test-skip:" 'github.event_name'
assert_not_contains_block "${ci_workflow}" "^  docs-lint-skip:" 'github.event_name'
# The primary jobs themselves now carry the PR-gated filler logic.
# Verify the internal skip echo strings exist inside the primary job blocks.
assert_contains "${ci_workflow}" "Skip (filler) when no lint/format required on PR"
assert_contains "${ci_workflow}" "Skip (filler) when no typecheck required on PR"
assert_contains "${ci_workflow}" "Skip (filler) when no unit tests required on PR"
assert_contains "${ci_workflow}" "Skip (filler) when no .md changes on PR"
# And that the filler steps are inside the primary job defs (not as top level *-skip).
# (The old twin job ids are gone; the echo strings live under the real job names.)
# Verify the filler steps themselves are gated to pull_request only (per CIB-038 design).
for filler in 'Skip (filler) when no lint/format required on PR' 'Skip (filler) when no typecheck required on PR' 'Skip (filler) when no unit tests required on PR' 'Skip (filler) when no .md changes on PR'; do
  if ! grep -A 10 -F "name: $filler" "${ci_workflow}" | grep -q "github.event_name == 'pull_request'"; then
    echo "expected filler step \"$filler\" in ${ci_workflow} to gate on github.event_name == 'pull_request'" >&2
    exit 1
  fi
done

# ── PR-only dependency audit ────────────────────────────────────
# ci.yml hosts the PR-summary Trivy job ("Dependency Audit (PR)"). The
# integration push is covered by security.yml's dependency-audit job, so
# ci.yml must gate this one to PR events to avoid running Trivy twice on
# the same SHA.
assert_contains "${ci_workflow}" 'name: Dependency Audit (PR)'
awk '
  /^  dependency-audit:/ { inside = 1; next }
  inside && /^  [a-z]/ { inside = 0 }
  inside && /github.event_name == .pull_request./ { found = 1 }
  END { exit (found ? 0 : 1) }
' "${ci_workflow}" || {
  echo "expected dependency-audit block in ci.yml to gate on pull_request" >&2
  exit 1
}

# Security workflow still owns dependency-audit on push (no event gate at
# the job level — path filter takes over).
assert_contains "${security_workflow}" '  dependency-audit:'
assert_contains "${security_workflow}" '    name: Dependency Audit'

# ── Integration readiness summary ───────────────────────────────
# Push-only aggregator that names the SHA and the validating jobs.
assert_contains "${ci_workflow}" '  integration-readiness:'
assert_contains "${ci_workflow}" '    name: Integration Readiness'
assert_contains "${ci_workflow}" "if: always() && github.event_name == 'push'"
assert_contains "${ci_workflow}" '## Integration Readiness'
assert_contains "${ci_workflow}" 'Fail if any required integration job failed'

# Readiness depends on the integration-validating jobs.
for need in detect-changes docs-lint metadata-validation platform-smoke aps-drift lint typecheck test build e2e-harness; do
  awk -v need="${need}" '
    /^  integration-readiness:/ { inside = 1; next }
    inside && /^  [a-z]/ { inside = 0 }
    inside && $0 ~ ("- " need "$") { found = 1 }
    END { exit (found ? 0 : 1) }
  ' "${ci_workflow}" || {
    echo "expected integration-readiness to depend on ${need}" >&2
    exit 1
  }
done

# Docs Lint split: the required check name stays on the `docs-lint`
# aggregator. Corpus work lives in `docs-lint-corpus`. The fixture suite
# lives in `docs-lint-tooling` so a corpus failure can be re-run without
# paying for tooling, and both run in parallel when a change needs both.
assert_contains "${ci_workflow}" '  docs-lint-corpus:'
assert_contains "${ci_workflow}" '    name: Docs corpus'
assert_contains "${ci_workflow}" '  docs-lint-tooling:'
assert_contains "${ci_workflow}" '    name: Docs tooling'

job_block_contains() {
  local job="$1"
  local needle="$2"
  awk -v job="^  ${job}:$" -v needle="${needle}" '
    $0 ~ job { inside = 1; next }
    inside && /^  [a-z]/ { inside = 0 }
    inside && index($0, needle) { found = 1 }
    END { exit (found ? 0 : 1) }
  ' "${ci_workflow}"
}

job_block_excludes() {
  local job="$1"
  local needle="$2"
  if job_block_contains "${job}" "${needle}"; then
    echo "expected ${job} in ${ci_workflow} not to contain: ${needle}" >&2
    exit 1
  fi
}

step_block_contains() {
  local step="$1"
  local needle="$2"
  awk -v marker="      - name: ${step}" -v needle="${needle}" '
    $0 == marker { inside = 1; next }
    inside && /^      - name:/ { inside = 0 }
    inside && index($0, needle) { found = 1 }
    END { exit (found ? 0 : 1) }
  ' "${ci_workflow}"
}

step_block_excludes() {
  local step="$1"
  local needle="$2"
  if step_block_contains "${step}" "${needle}"; then
    echo "expected step '${step}' in ${ci_workflow} not to contain: ${needle}" >&2
    exit 1
  fi
}

for need in detect-changes docs-lint-corpus docs-lint-tooling; do
  job_block_contains docs-lint "- ${need}" || {
    echo "expected docs-lint aggregator to depend on ${need}" >&2
    exit 1
  }
done

job_block_contains docs-lint-tooling 'pnpm run test:docs-check' || {
  echo "expected docs-lint-tooling to run pnpm run test:docs-check" >&2
  exit 1
}
job_block_contains docs-lint-tooling 'pnpm run test:aps-active-lint' || {
  echo "expected docs-lint-tooling to run pnpm run test:aps-active-lint" >&2
  exit 1
}
job_block_contains docs-lint-tooling 'pnpm run test:install-sh' || {
  echo "expected docs-lint-tooling to run pnpm run test:install-sh" >&2
  exit 1
}

job_block_excludes docs-lint-corpus 'pnpm run test:docs-check'
job_block_excludes docs-lint-corpus 'pnpm run test:aps-active-lint'
job_block_excludes docs-lint-corpus 'pnpm run test:install-sh'
job_block_excludes docs-lint 'pnpm run test:docs-check'
job_block_excludes docs-lint 'pnpm run docs:check'

job_block_contains docs-lint-corpus 'pnpm run docs:check' || {
  echo "expected docs-lint-corpus to run pnpm run docs:check" >&2
  exit 1
}
job_block_contains docs-lint-corpus 'check-docs-owed.mjs' || {
  echo "expected docs-lint-corpus to run the owed-docs gate" >&2
  exit 1
}
job_block_contains docs-lint-corpus 'check-diagram-impact.mjs --since "${base}"' || {
  echo "expected docs-lint-corpus to run the diagram-impact gate" >&2
  exit 1
}
step_block_contains 'Check diagram impact for this change' '! git cat-file -e "${base}^{commit}"' || {
  echo "expected diagram-impact base validation to reject an invalid commit" >&2
  exit 1
}
step_block_contains 'Check diagram impact for this change' '::error::Cannot determine diagram-impact diff base' || {
  echo "expected invalid diagram-impact base selection to fail loudly" >&2
  exit 1
}
step_block_contains 'Check diagram impact for this change' 'exit 2' || {
  echo "expected invalid diagram-impact base selection to fail closed" >&2
  exit 1
}
step_block_excludes 'Check diagram impact for this change' 'HEAD~1'
assert_contains "${detect_action}" 'diagram-impact-required:'
assert_contains "${ci_workflow}" 'diagram-impact-required: ${{ steps.changes.outputs.diagram-impact-required }}'

# Pin the required-check name and fail-closed aggregator. A rename or an
# `if` that drops `always()` / worker failure would skip-satisfy the
# ruleset the way the old twin filler jobs did.
job_block_contains docs-lint 'name: Docs Lint' || {
  echo "expected docs-lint aggregator to be named Docs Lint" >&2
  exit 1
}
job_block_contains docs-lint "needs.docs-lint-corpus.result == 'failure'" || {
  echo "expected docs-lint aggregator to fail when corpus failed" >&2
  exit 1
}
job_block_contains docs-lint "needs.docs-lint-tooling.result == 'failure'" || {
  echo "expected docs-lint aggregator to fail when tooling failed" >&2
  exit 1
}

awk '
  /^  docs-lint:$/ { inside = 1; next }
  inside && /^  [a-z]/ { inside = 0 }
  inside && $0 ~ /^    if:/ { seen_if = 1 }
  inside && seen_if && /always\(\)/ { found = 1 }
  END { exit (found ? 0 : 1) }
' "${ci_workflow}" || {
  echo "expected docs-lint aggregator job if to include always()" >&2
  exit 1
}

# Push must not grow a new corpus/tooling bill: code-only and scripts-only
# pushes skipped the combined job. Workers may run those paths on PRs.
awk '
  /^  docs-lint-corpus:$/ { inside = 1; next }
  inside && /^  [a-z]/ { inside = 0 }
  inside && /github.event_name == .pull_request./ { found = 1 }
  END { exit (found ? 0 : 1) }
' "${ci_workflow}" || {
  echo "expected docs-lint-corpus to gate code-only work on pull_request" >&2
  exit 1
}
awk '
  /^  docs-lint-tooling:$/ { inside = 1; next }
  inside && /^  [a-z]/ { inside = 0 }
  inside && /github.event_name == .pull_request./ { found = 1 }
  END { exit (found ? 0 : 1) }
' "${ci_workflow}" || {
  echo "expected docs-lint-tooling to keep scripts-only pushes from always running" >&2
  exit 1
}

# Security summary remains PR-only — push integration must not post a
# PR-style comment because there is no PR to comment on.
assert_contains "${security_workflow}" "github.event_name == 'pull_request'"
awk '
  /^  summary:/ { inside = 1; next }
  inside && /^  [a-z]/ { inside = 0 }
  inside && /github.event_name == .pull_request./ { found = 1 }
  END { exit (found ? 0 : 1) }
' "${security_workflow}" || {
  echo "expected security summary block to gate on pull_request" >&2
  exit 1
}

echo 'integration push validation contract checks passed'
