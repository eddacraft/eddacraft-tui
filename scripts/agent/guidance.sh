#!/usr/bin/env bash
# Deterministic advisory guidance for agents, hooks, and CI.
# Maps changed paths to playbooks, review tiers, and validation checks.

set -euo pipefail

MODE=""
BASE_REF="origin/main"
OUTPUT="text"
FILES_FROM=""
SOURCE=""

usage() {
  cat <<'EOF'
Usage: scripts/agent/guidance.sh [--staged|--branch|--pr|--files-from <path>] [--base <ref>] [--json]

Modes:
  --staged            Use staged files.
  --branch           Use changed files from <base>...HEAD. Default base: origin/main.
  --pr               Use PR changed files via gh when available; falls back to branch mode.
  --files-from PATH  Read newline-delimited paths from PATH. Intended for tests/CI fixtures.

Output defaults to concise text. Use --json for machine-readable advisory output.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --staged|--branch|--pr)
      MODE="${1#--}"
      ;;
    --files-from)
      if [[ -z "${2:-}" || "${2:-}" == --* ]]; then
        echo "guidance.sh: --files-from requires a path" >&2
        exit 2
      fi
      MODE="files"
      FILES_FROM="$2"
      shift
      ;;
    --base)
      if [[ -z "${2:-}" || "${2:-}" == --* ]]; then
        echo "guidance.sh: --base requires a ref" >&2
        exit 2
      fi
      BASE_REF="$2"
      shift
      ;;
    --json)
      OUTPUT="json"
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    *)
      echo "guidance.sh: unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
  shift
done

if [[ -z "$MODE" ]]; then
  MODE="staged"
fi

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

branch_changed_files() {
  git diff --name-only "$BASE_REF...HEAD" || {
    echo "guidance.sh: unable to diff base ref: $BASE_REF" >&2
    return 2
  }
  git diff --name-only
  git diff --cached --name-only
  git ls-files --others --exclude-standard
}

changed_files() {
  local gh_files

  case "$MODE" in
    staged)
      git diff --cached --name-only
      ;;
    branch)
      branch_changed_files
      ;;
    pr)
      if [[ "$SOURCE" == "gh-pr" ]]; then
        if gh_files="$(gh pr diff --name-only)"; then
          # PR mode is PR-scoped: do not mix in live working-tree / branch
          # overlays when gh succeeds. Local overlays live under --branch.
          printf '%s\n' "$gh_files"
        else
          SOURCE="branch-fallback"
          add_unique WARNINGS "PR diff unavailable; guidance fell back to branch/local git diff."
          branch_changed_files
        fi
      else
        branch_changed_files
      fi
      ;;
    files)
      if [[ -z "$FILES_FROM" || ! -f "$FILES_FROM" ]]; then
        echo "guidance.sh: --files-from requires a readable file" >&2
        exit 2
      fi
      sed '/^[[:space:]]*$/d' "$FILES_FROM"
      ;;
    *)
      echo "guidance.sh: unsupported mode: $MODE" >&2
      exit 2
      ;;
  esac
}

declare -a FILES=()

declare -a PLAYBOOKS=()
declare -a REVIEWS=()
declare -a CHECKS=()
declare -a WARNINGS=()
declare -a RISK_CLASSES=()
REVIEW_TIER="targeted"

add_unique() {
  local target_array="$1"
  local value="$2"
  local existing
  case "$target_array" in
    PLAYBOOKS|REVIEWS|CHECKS|WARNINGS|RISK_CLASSES) ;;
    *)
      echo "guidance.sh: invalid array target: $target_array" >&2
      exit 2
      ;;
  esac

  eval "local values=(\"\${${target_array}[@]}\")"
  for existing in "${values[@]}"; do
    [[ "$existing" == "$value" ]] && return 0
  done
  eval "${target_array}+=(\"\$value\")"
}

case "$MODE" in
  staged) SOURCE="git-staged" ;;
  branch) SOURCE="git-branch" ;;
  files) SOURCE="files-from" ;;
  pr)
    if command -v gh >/dev/null 2>&1 && gh pr view --json number >/dev/null 2>&1; then
      SOURCE="gh-pr"
    else
      SOURCE="branch-fallback"
      add_unique WARNINGS "PR diff unavailable; guidance fell back to branch/local git diff."
    fi
    ;;
esac

changed_output="$(changed_files | sort -u)"
if [[ -n "$changed_output" ]]; then
  while IFS= read -r file; do
    FILES+=("$file")
  done <<< "$changed_output"
fi

set_review_tier() {
  local tier="$1"
  case "$tier" in
    full)
      REVIEW_TIER="full"
      ;;
    mini)
      [[ "$REVIEW_TIER" != "full" ]] && REVIEW_TIER="mini"
      ;;
    targeted)
      :
      ;;
  esac
}

classify_file() {
  local file="$1"

  case "$file" in
    .github/workflows/release.yml|scripts/release/*|docs/guides/release-runbook.md|.claude/skills/release/*|.claude/skills/release/SKILL.md|plans/specs/*release*|plans/specs/*readiness*)
      add_unique RISK_CLASSES "release"
      add_unique PLAYBOOKS "docs/guides/release-runbook.md"
      add_unique REVIEWS "operations-reviewer"
      add_unique REVIEWS "security-reviewer"
      add_unique CHECKS "release-readiness-impact"
      add_unique WARNINGS "Release or workflow surface changed; verify command availability, branch authority, and release-record impact."
      set_review_tier "mini"
      ;;
  esac

  case "$file" in
    plans/*.aps.md|plans/modules/*.aps.md|plans/index.aps.md|plans/aps-rules.md|plans/specs/*.md)
      add_unique RISK_CLASSES "aps"
      add_unique PLAYBOOKS "plans/aps-rules.md"
      add_unique CHECKS "pnpm lint:md"
      add_unique WARNINGS "APS or operating-model files changed; update plans/index.aps.md when module status/count changes."
      ;;
  esac

  case "$file" in
    docs/*|*.md|README.md|CONTRIBUTING.md|AGENTS.md)
      add_unique RISK_CLASSES "docs"
      add_unique PLAYBOOKS "docs/guides/documentation-governance.md"
      add_unique CHECKS "pnpm lint:md"
      add_unique CHECKS "pnpm format:check"
      ;;
  esac

  case "$file" in
    .claude/*|scripts/agent/*|scripts/agent/**|tools/local-agent-run.sh)
      add_unique RISK_CLASSES "agent-workflow"
      add_unique PLAYBOOKS "plans/specs/2026-05-09-agentic-execution-ecosystem-architecture.md"
      add_unique PLAYBOOKS "plans/specs/2026-05-09-council-agent-skill-change-proposal.md"
      add_unique REVIEWS "council-reviewer"
      add_unique REVIEWS "adversarial-reviewer"
      add_unique CHECKS "review/council alignment"
      set_review_tier "mini"
      ;;
  esac

  case "$file" in
    .github/*|.github/actions/*|.github/actions/**)
      add_unique RISK_CLASSES "ci"
      add_unique REVIEWS "operations-reviewer"
      add_unique CHECKS "CI path/change detection impact"
      set_review_tier "mini"
      ;;
  esac

  case "$file" in
    packages/*|packages/**|apps/*|apps/**|tools/*|tools/**|infra/*|infra/**|package.json|pnpm-lock.yaml|nx.json|tsconfig*.json)
      add_unique RISK_CLASSES "source"
      add_unique CHECKS "pnpm typecheck"
      add_unique CHECKS "pnpm test"
      add_unique REVIEWS "council-reviewer"
      ;;
  esac

  case "$file" in
    crates/*|crates/**|Cargo.toml|Cargo.lock|rust-toolchain.toml|dist-workspace.toml)
      add_unique RISK_CLASSES "source"
      add_unique CHECKS "cargo test --workspace"
      add_unique CHECKS "pnpm typecheck:rust"
      add_unique REVIEWS "council-reviewer"
      ;;
  esac
}

for file in "${FILES[@]}"; do
  classify_file "$file"
done

if [[ ${#RISK_CLASSES[@]} -eq 0 ]]; then
  add_unique RISK_CLASSES "unknown"
  add_unique WARNINGS "No guidance rule matched changed paths; use targeted review and relevant local validation."
fi

if [[ ${#REVIEWS[@]} -eq 0 ]]; then
  add_unique REVIEWS "council-reviewer"
fi

json_escape() {
  local input="$1"
  local output=""
  local char code hex

  local i
  for ((i = 0; i < ${#input}; i++)); do
    char="${input:i:1}"
    case "$char" in
      '\\') output+="\\\\" ;;
      '"') output+='\"' ;;
      $'\b') output+='\b' ;;
      $'\f') output+='\f' ;;
      $'\n') output+='\n' ;;
      $'\r') output+='\r' ;;
      $'\t') output+='\t' ;;
      *)
        printf -v code '%d' "'$char"
        if ((code < 32)); then
          printf -v hex '\\u%04x' "$code"
          output+="$hex"
        else
          output+="$char"
        fi
        ;;
    esac
  done

  printf '%s' "$output"
}

json_array() {
  local array_name="$1"
  local value
  case "$array_name" in
    PLAYBOOKS|REVIEWS|CHECKS|WARNINGS|RISK_CLASSES|FILES) ;;
    *)
      echo "guidance.sh: invalid array target: $array_name" >&2
      exit 2
      ;;
  esac

  eval "local values_ref=(\"\${${array_name}[@]}\")"
  printf '['
  local first=true
  for value in "${values_ref[@]}"; do
    if [[ "$first" == true ]]; then
      first=false
    else
      printf ','
    fi
    printf '"%s"' "$(json_escape "$value")"
  done
  printf ']'
}

primary_risk="${RISK_CLASSES[0]}"
for risk in "${RISK_CLASSES[@]}"; do
  if [[ "$risk" == "release" ]]; then
    primary_risk="release"
    break
  fi
  if [[ "$risk" == "agent-workflow" && "$primary_risk" != "release" ]]; then
    primary_risk="agent-workflow"
  fi
done

if [[ "$OUTPUT" == "json" ]]; then
  printf '{'
  printf '"advisory":true,'
  printf '"enforcement":"none",'
  printf '"mode":"%s",' "$MODE"
  printf '"source":"%s",' "$SOURCE"
  printf '"riskClass":"%s",' "$primary_risk"
  printf '"riskClasses":'; json_array RISK_CLASSES; printf ','
  printf '"reviewTier":"%s",' "$REVIEW_TIER"
  printf '"requiredPlaybooks":'; json_array PLAYBOOKS; printf ','
  printf '"requiredReviews":'; json_array REVIEWS; printf ','
  printf '"requiredChecks":'; json_array CHECKS; printf ','
  printf '"warnings":'; json_array WARNINGS; printf ','
  printf '"changedFiles":'; json_array FILES
  printf '}\n'
  exit 0
fi

echo "Agent guidance"
echo "Mode: $MODE"
echo "Source: $SOURCE"
echo "Risk: $primary_risk"
echo "Review tier: $REVIEW_TIER"
echo "Advisory: true (enforcement: none)"
echo

if [[ ${#PLAYBOOKS[@]} -gt 0 ]]; then
  echo "Read before proceeding:"
  for item in "${PLAYBOOKS[@]}"; do
    echo "- $item"
  done
  echo
fi

echo "Review:"
for item in "${REVIEWS[@]}"; do
  echo "- $item"
done
echo

if [[ ${#CHECKS[@]} -gt 0 ]]; then
  echo "Checks:"
  for item in "${CHECKS[@]}"; do
    echo "- $item"
  done
  echo
fi

if [[ ${#WARNINGS[@]} -gt 0 ]]; then
  echo "Warnings:"
  for item in "${WARNINGS[@]}"; do
    echo "- $item"
  done
fi
