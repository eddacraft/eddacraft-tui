#!/usr/bin/env bash
# Classify changed paths into deterministic CI validation requirements.

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

  case "${path}" in
    scripts/*.sh | scripts/**/*.sh)
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
  case "${path}" in
    pnpm-lock.yaml | package-lock.json | yarn.lock | package.json | packages/*/package.json | packages/**/package.json)
      add_unique path_classes 'lockfile'
      add_unique risk_classes 'dependencies'
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
      ;;
    rust)
      add_unique required_checks 'cargo-fmt'
      add_unique required_checks 'cargo-clippy'
      add_unique required_checks 'cargo-check'
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
