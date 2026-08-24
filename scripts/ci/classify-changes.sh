#!/usr/bin/env bash
# Classify changed paths into deterministic CI validation requirements.
#
# CIB-137 security contract: this script decides which REQUIRED checks a PR must
# run, so it must be executed from a TRUSTED ref, never the PR's own head. In CI
# `.github/actions/detect-changes/action.yml` locates it via $GITHUB_ACTION_PATH
# from an actions/checkout of the base SHA (see `.github/workflows/ci.yml`,
# "Checkout trusted classifier"). This script is a PURE path classifier: it
# reads only the `--paths-file` (an absolute path) / stdin and never inspects
# the working tree, so it is CWD-independent and needs no repo-root argument.
# Do NOT reintroduce a workspace-relative invocation of this script — that would
# re-open the PR-controlled spoof path.

set -euo pipefail

context='branch'
paths_file=''

usage() {
  cat <<'EOF'
Usage: scripts/ci/classify-changes.sh [options] [-- path ...]

Options:
  --context <name>      Classification context: staged, branch, pr, or push
  --paths-file <file>   Newline-delimited changed path list
  --json               Emit JSON (default; reserved for future formats)
  -h, --help           Show this help

If no paths are passed, the classifier reads newline-delimited paths from stdin.
EOF
}

paths=()
while (($#)); do
  case "$1" in
    --context)
      context="${2:?--context requires a value}"
      shift 2
      ;;
    --paths-file)
      paths_file="${2:?--paths-file requires a value}"
      shift 2
      ;;
    --json)
      shift
      ;;
    -h | --help)
      usage
      exit 0
      ;;
    --)
      shift
      while (($#)); do
        paths+=("$1")
        shift
      done
      ;;
    *)
      paths+=("$1")
      shift
      ;;
  esac
done

case "${context}" in
  staged | branch | pr | push) ;;
  *)
    echo "unsupported context: ${context}" >&2
    exit 2
    ;;
esac

if ! command -v jq >/dev/null 2>&1; then
  echo "jq is required" >&2
  exit 2
fi

if [[ -n "${paths_file}" ]]; then
  if [[ ! -f "${paths_file}" ]]; then
    echo "paths file not found: ${paths_file}" >&2
    exit 2
  fi
  while IFS= read -r path || [[ -n "${path}" ]]; do
    [[ -z "${path}" ]] && continue
    paths+=("${path}")
  done <"${paths_file}"
elif ((${#paths[@]} == 0)) && [[ ! -t 0 ]]; then
  while IFS= read -r path || [[ -n "${path}" ]]; do
    [[ -z "${path}" ]] && continue
    paths+=("${path}")
  done
fi

add_unique() {
  local name="$1"
  local value="$2"
  local item
  case "${name}" in
    path_classes | risk_classes | required_checks | required_reviews | warnings) ;;
    *)
      echo "unsupported target array: ${name}" >&2
      exit 2
      ;;
  esac
  local -n target="${name}"
  for item in "${target[@]}"; do
    [[ "${item}" == "${value}" ]] && return 0
  done
  target+=("${value}")
}

json_array() {
  if (($# == 0)); then
    printf '[]\n'
    return 0
  fi
  printf '%s\n' "$@" | jq -R . | jq -s .
}

path_classes=()
risk_classes=()
required_checks=()
required_reviews=()
warnings=()

if ((${#paths[@]} == 0)); then
  add_unique warnings 'no-changed-paths'
else
  # DOCRB-009: route every trusted non-empty change set through the one cheap
  # diagram-impact signal. The semantic checker owns declared-upstream
  # matching, so shell routing cannot drift from documentation metadata.
  add_unique required_checks 'diagram-impact'
fi

for path in "${paths[@]}"; do
  matched=false

  case "${path}" in
    docs/* | README.md | CONTRIBUTING.md | AGENTS.md | CHANGELOG.md | plans/*.md | plans/**/*.md | *.md)
      add_unique path_classes 'docs'
      matched=true
      ;;
  esac

  case "${path}" in
    *.ts | *.tsx | *.js | *.jsx | *.mjs | *.cjs | packages/*/src/* | packages/*/__tests__/* | apps/*/src/*)
      add_unique path_classes 'ts'
      add_unique risk_classes 'source'
      matched=true
      ;;
  esac

  case "${path}" in
    *.rs | crates/* | crates/**/* | Cargo.toml | Cargo.lock | rust-toolchain.toml | dist-workspace.toml)
      add_unique path_classes 'rust'
      add_unique risk_classes 'source'
      matched=true
      ;;
  esac

  # FLAGCAT-012: the product catalogue is compiled/runtime source truth shared
  # by TypeScript and Rust hosts. Keep it out of the unknown fallback so a
  # catalogue-only PR runs both host projection suites.
  case "${path}" in
    flags/surfaces.json)
      add_unique path_classes 'catalogue'
      add_unique risk_classes 'source'
      matched=true
      ;;
  esac

  # DEVENV-007 (ADR-057): the cross-surface E2E harness (apps/e2e) and the
  # Playwright config are the E2E surface itself, so editing them requires the
  # `e2e` check directly. TS *source* anywhere also implies the E2E surface (see
  # the `ts` → `e2e` mapping below): the harness builds the anvil-api dependency
  # closure and exercises core/api/adapters/contracts, so a TS source change is
  # E2E-impacting even when it never touches apps/e2e. That closes the path-gate
  # that let an observability source break skip E2E on its PR (which gated only
  # on apps/e2e edits) and land the failure on the integration branch.
  case "${path}" in
    apps/e2e/* | apps/e2e/**/* | playwright.config.ts)
      add_unique path_classes 'e2e'
      add_unique risk_classes 'source'
      matched=true
      ;;
  esac

  case "${path}" in
    policies/* | policies/**/* | *.rego)
      add_unique path_classes 'policy'
      add_unique risk_classes 'policy'
      matched=true
      ;;
  esac

  case "${path}" in
    .github/workflows/* | .github/actions/* | .github/actions/**/*)
      add_unique path_classes 'workflow'
      add_unique risk_classes 'workflow'
      matched=true
      ;;
  esac

  # CIB-041: agent-tooling config directories (.codex / .claude / .opencode)
  # hold no TypeScript or Rust source. Without an explicit class they fall
  # through to the conservative `unknown` fallback below, which forces the
  # full unit-test / typecheck matrix on pure agent-config bookkeeping PRs
  # even though there is nothing to build or test. Markdown within these dirs
  # still also matches the `docs` case above (markdownlint accumulates); the
  # dirs are oxfmt-excluded (see .prettierignore), so there is no format gate
  # — an operations review covers the agent-execution config surface.
  case "${path}" in
    .codex/* | .codex/**/* | .claude/* | .claude/**/* | .opencode/* | .opencode/**/*)
      add_unique path_classes 'agent-config'
      add_unique risk_classes 'tooling'
      matched=true
      ;;
  esac

  # Repository metadata shapes local tooling behaviour but is not compiled
  # source. Keep it out of the conservative `unknown` bucket so allowlist-only
  # edits (for example pairing `.gitignore` with a tracked agent skill) do not
  # force the full Node build/test matrix.
  case "${path}" in
    .gitignore | .gitattributes | .editorconfig | .prettierignore | .prettierrc | .prettierrc.*)
      add_unique path_classes 'repo-metadata'
      add_unique risk_classes 'tooling'
      matched=true
      ;;
  esac

  case "${path}" in
    scripts/*.sh | scripts/**/*.sh)
      add_unique path_classes 'shell'
      add_unique risk_classes 'automation'
      matched=true
      ;;
  esac

  # CIB-277: the git hooks are shell scripts with no `.sh` suffix, so they fell
  # through to `unknown` — which requires format/lint/typecheck/unit-tests but
  # NOT `script-fixtures`. Editing the pre-commit gate therefore skipped the
  # fixtures that exercise it, which is how a defect in the gate reaches main.
  case "${path}" in
    .husky/* | .husky/**/*)
      add_unique path_classes 'shell'
      add_unique risk_classes 'automation'
      matched=true
      ;;
  esac

  # CIB-022: node-based automation scripts also carry shell-style fixture tests
  # (e.g. scripts/aps/_test/*.test.sh exercising scripts/aps/*.mjs), so a
  # change to the .mjs must run `script-fixtures`. They still match the `ts`
  # case above for lint/format/typecheck; classes accumulate.
  case "${path}" in
    scripts/*.mjs | scripts/**/*.mjs)
      add_unique path_classes 'shell'
      add_unique risk_classes 'automation'
      matched=true
      ;;
  esac

  case "${path}" in
    infra/* | infra/**/* | deploy/* | deploy/**/* | Pulumi.yaml | Pulumi.*.yaml | docker-compose.* | Dockerfile)
      add_unique path_classes 'infra'
      add_unique risk_classes 'infra'
      matched=true
      ;;
  esac

  case "${path}" in
    scripts/release* | scripts/release/* | scripts/release/**/* | .changeset/* | .changeset/**/* | docs/public/anvil/releases/* | docs/public/anvil/releases/**/*)
      add_unique path_classes 'release'
      add_unique risk_classes 'release'
      matched=true
      ;;
  esac

  case "${path}" in
    *native* | *napi* | packages/*-native/* | packages/*-native/**/*)
      add_unique path_classes 'napi'
      add_unique risk_classes 'platform'
      matched=true
      ;;
  esac

  # CIB-031: the `lockfile` class drives the `dependency-audit` required
  # check, which gates the npm-facing Trivy scan and `license-check` in
  # `.github/workflows/security.yml`. Restrict it to npm manifests/lockfiles
  # so a Rust-only `Cargo.lock` / `Cargo.toml` change does not run a
  # whole-repo Trivy scan that surfaces unrelated `pnpm-lock.yaml`
  # advisories. Rust dependency changes already route to the `rust` class
  # above (`cargo-deny` lives in `.github/workflows/rust.yml`).
  #
  # The Trivy suppression files join this class so the audit guards its own
  # configuration. A malformed ignorefile — or a `trivyignores` input that
  # stops pointing at it — parses cleanly and silently suppresses nothing,
  # which is only observable when the audit actually runs. Without them a
  # config-only PR could land a no-op suppression against a green board.
  case "${path}" in
    pnpm-lock.yaml | package-lock.json | yarn.lock | package.json | packages/*/package.json | packages/**/package.json | .trivyignore | .trivyignore.yaml)
      add_unique path_classes 'lockfile'
      add_unique risk_classes 'dependencies'
      matched=true
      ;;
  esac

  # DEVENV-010: the root manifest and CONTRIBUTING jointly state the toolchain
  # floors, and they drifted — `engines` moved to node >=24 / pnpm >=11 while
  # the onboarding doc still said 22/10, sending a fresh clone into a pnpm that
  # could not run at all. `contributing-engines-parity.test.sh` guards the
  # pair, so a change to either side must run it.
  #
  # Its own class rather than reusing `shell`: that one also pulls in
  # `shell-syntax` and an `operations` review, which would land on every
  # dependency bump including Dependabot's. This adds only the check needed.
  case "${path}" in
    package.json | CONTRIBUTING.md)
      add_unique path_classes 'toolchain-contract'
      matched=true
      ;;
  esac

  if [[ "${matched}" == false ]]; then
    add_unique path_classes 'unknown'
    add_unique risk_classes 'unknown'
    add_unique warnings 'unclassified-paths'
  fi
done

if ((${#path_classes[@]} == 1)) && [[ "${path_classes[0]}" == 'docs' ]]; then
  add_unique risk_classes 'docs-only'
fi

if ((${#path_classes[@]} > 1)); then
  add_unique path_classes 'mixed'
  add_unique warnings 'mixed-change-set'
fi

if ((${#path_classes[@]} == 0)) && ((${#paths[@]} > 0)); then
  add_unique path_classes 'unknown'
  add_unique risk_classes 'unknown'
  add_unique warnings 'unclassified-paths'
fi

for path_class in "${path_classes[@]}"; do
  case "${path_class}" in
    docs)
      add_unique required_checks 'markdownlint'
      ;;
    ts)
      add_unique required_checks 'format'
      add_unique required_checks 'lint'
      add_unique required_checks 'typecheck'
      add_unique required_checks 'unit-tests'
      # DEVENV-007: TS source is E2E-impacting (harness builds the anvil-api
      # closure). Mirrors CI's intended trigger: "any TS source or the harness".
      add_unique required_checks 'e2e'
      ;;
    e2e)
      # The E2E harness / Playwright config themselves.
      add_unique required_checks 'e2e'
      ;;
    rust)
      add_unique required_checks 'cargo-fmt'
      add_unique required_checks 'cargo-clippy'
      add_unique required_checks 'cargo-check'
      add_unique required_checks 'cargo-test'
      ;;
    catalogue)
      add_unique required_checks 'format'
      add_unique required_checks 'lint'
      add_unique required_checks 'typecheck'
      add_unique required_checks 'unit-tests'
      add_unique required_checks 'cargo-test'
      ;;
    policy)
      add_unique required_checks 'opa-test'
      add_unique required_checks 'regal'
      ;;
    workflow)
      add_unique required_checks 'workflow-lint'
      add_unique required_reviews 'operations'
      ;;
    agent-config)
      # No build/test/format gate (oxfmt-excluded, no compiled source); the
      # agent-execution config surface still warrants a human ops review.
      add_unique required_reviews 'operations'
      ;;
    repo-metadata)
      # Metadata-only changes are reviewed through the surrounding class.
      # They do not require build, test, or format gates by themselves.
      ;;
    toolchain-contract)
      # Only the fixture that compares `engines` with CONTRIBUTING (DEVENV-010).
      # Deliberately no review requirement: the pair is machine-checkable, so a
      # human gate would add friction without adding assurance.
      add_unique required_checks 'script-fixtures'
      ;;
    shell)
      add_unique required_checks 'shell-syntax'
      add_unique required_checks 'script-fixtures'
      add_unique required_reviews 'operations'
      ;;
    infra)
      add_unique required_checks 'infra-static-check'
      add_unique required_reviews 'operations'
      ;;
    release)
      add_unique required_checks 'release-dry-run'
      add_unique required_reviews 'release'
      ;;
    napi)
      add_unique required_checks 'platform-smoke'
      add_unique required_reviews 'operations'
      ;;
    lockfile)
      add_unique required_checks 'dependency-audit'
      add_unique required_reviews 'security'
      ;;
    unknown)
      add_unique required_checks 'format'
      add_unique required_checks 'lint'
      add_unique required_checks 'typecheck'
      add_unique required_checks 'unit-tests'
      add_unique required_reviews 'operations'
      ;;
  esac
done

jq -n \
  --arg context "${context}" \
  --argjson paths "$(json_array "${paths[@]}")" \
  --argjson pathClasses "$(json_array "${path_classes[@]}")" \
  --argjson riskClasses "$(json_array "${risk_classes[@]}")" \
  --argjson requiredChecks "$(json_array "${required_checks[@]}")" \
  --argjson requiredReviews "$(json_array "${required_reviews[@]}")" \
  --argjson warnings "$(json_array "${warnings[@]}")" \
  '{schemaVersion: 1, context: $context, paths: $paths, pathClasses: $pathClasses, riskClasses: $riskClasses, requiredChecks: $requiredChecks, requiredReviews: $requiredReviews, warnings: $warnings}'
