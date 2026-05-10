#!/usr/bin/env bash
# Run local deterministic validation selected by changed path classification.

set -euo pipefail

repo_root=$(git rev-parse --show-toplevel 2>/dev/null || pwd)
classifier="${repo_root}/scripts/ci/classify-changes.sh"

mode=''
paths_file=''
dry_run=false
json=false

usage() {
  cat <<'EOF'
Usage: scripts/validate/local.sh (--staged|--changed|--full) [options]

Options:
  --paths-file <file>  Use newline-delimited paths instead of git detection
  --dry-run            Print the selected command plan without running it
  --json               Emit JSON plan output
  -h, --help           Show this help

Modes:
  --staged   Validate staged files using the shared classifier
  --changed  Validate branch changes against the integration base
  --full     Run the full deterministic local validation suite
EOF
}

while (($#)); do
  case "$1" in
    --staged | --changed | --full)
      if [[ -n "${mode}" ]]; then
        echo 'choose only one validation mode' >&2
        exit 2
      fi
      mode="${1#--}"
      shift
      ;;
    --paths-file)
      paths_file="${2:?--paths-file requires a value}"
      shift 2
      ;;
    --dry-run)
      dry_run=true
      shift
      ;;
    --json)
      json=true
      shift
      ;;
    --)
      shift
      ;;
    -h | --help)
      usage
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

if [[ -z "${mode}" ]]; then
  usage >&2
  exit 2
fi

if ! command -v jq >/dev/null 2>&1; then
  echo 'jq is required' >&2
  exit 2
fi

if [[ ! -x "${classifier}" ]]; then
  echo "classifier not executable: ${classifier}" >&2
  exit 2
fi

path_file=$(mktemp)
cleanup() {
  rm -f "${path_file}"
}
trap cleanup EXIT

detect_paths() {
  case "${mode}" in
    staged)
      git diff --cached --name-only --diff-filter=ACMR >"${path_file}"
      ;;
    changed)
      local base_ref="${VALIDATE_BASE_REF:-origin/dev}"
      local merge_base
      merge_base=$(git merge-base "${base_ref}" HEAD 2>/dev/null || true)
      if [[ -z "${merge_base}" ]]; then
        echo "could not determine merge-base for ${base_ref}" >&2
        exit 2
      fi
      git diff --name-only --diff-filter=ACMR "${merge_base}" HEAD >"${path_file}"
      ;;
    full)
      : >"${path_file}"
      ;;
  esac
}

if [[ -n "${paths_file}" ]]; then
  if [[ ! -f "${paths_file}" ]]; then
    echo "paths file not found: ${paths_file}" >&2
    exit 2
  fi
  cp "${paths_file}" "${path_file}"
else
  detect_paths
fi

classification='{}'
if [[ "${mode}" != 'full' ]]; then
  classifier_context="${mode}"
  [[ "${classifier_context}" == 'changed' ]] && classifier_context='branch'
  classification=$(bash "${classifier}" --context "${classifier_context}" --paths-file "${path_file}")
fi

commands=()
add_command() {
  local command="$1"
  local existing
  for existing in "${commands[@]}"; do
    [[ "${existing}" == "${command}" ]] && return 0
  done
  commands+=("${command}")
}

if [[ "${mode}" == 'full' ]]; then
  add_command 'pnpm format:check'
  add_command 'pnpm lint:check'
  add_command 'pnpm typecheck'
  add_command 'pnpm test'
else
  mapfile -t required_checks < <(jq -r '.requiredChecks[]?' <<<"${classification}")
  for check in "${required_checks[@]}"; do
    case "${check}" in
      markdownlint)
        add_command 'pnpm lint:md'
        ;;
      format | cargo-fmt)
        add_command 'pnpm format:check'
        ;;
      lint | workflow-lint)
        add_command 'pnpm lint:check'
        ;;
      typecheck | cargo-check)
        add_command 'pnpm typecheck'
        ;;
      unit-tests)
        add_command 'pnpm test'
        ;;
      cargo-clippy)
        add_command 'pnpm lint:rust'
        ;;
      cargo-test)
        add_command 'cargo test --workspace'
        ;;
      opa-test)
        add_command 'opa test --verbose policies/fixtures/'
        ;;
      dependency-audit)
        add_command 'pnpm audit --audit-level high'
        ;;
      *)
        ;;
    esac
  done
fi

plan_json=$(jq -n \
  --arg mode "${mode}" \
  --argjson classification "${classification}" \
  --argjson commands "$(printf '%s\n' "${commands[@]}" | jq -R . | jq -s .)" \
  '{mode: $mode, classification: $classification, commands: $commands}')

if [[ "${json}" == true ]]; then
  jq '.' <<<"${plan_json}"
elif [[ "${dry_run}" == true ]]; then
  jq -r '.commands[]' <<<"${plan_json}"
fi

if [[ "${dry_run}" == true ]]; then
  exit 0
fi

for command in "${commands[@]}"; do
  printf '==> %s\n' "${command}"
  bash -lc "${command}"
done
