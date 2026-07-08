#!/bin/bash
# Vercel Ignored Build Step
#
# Skips builds when no files in the project's directory or explicitly watched
# paths have changed since the last deployment. Vercel sets
# VERCEL_GIT_PREVIOUS_SHA to the last successfully deployed commit.
#
# Usage (set as ignoreCommand in Vercel project config):
#   bash tools/scripts/vercel-ignore-build.sh apps/website
#   bash tools/scripts/vercel-ignore-build.sh apps/docs-site docs/public
#   bash tools/scripts/vercel-ignore-build.sh --skip-preview apps/anvil-api
#
# Options:
#   --skip-preview  Skip builds on non-production branches (exit 0)
#   --always-skip   Always skip builds (exit 0); for retired Vercel projects
#
# Vercel ignoreCommand semantics:
#   Exit 1 = proceed with build
#   Exit 0 = skip build (cancel deployment)

set -euo pipefail

# Source logging library if available
REPO_ROOT=$(git rev-parse --show-toplevel)
_LOG_LIB="${REPO_ROOT}/.claude/hooks/lib/log.sh"
if [ -f "$_LOG_LIB" ]; then
  export ANVIL_LOG_TAG="vercel-ignore-build"
  source "$_LOG_LIB"
fi

type log_enter >/dev/null 2>&1 && log_enter "$@"

SKIP_PREVIEW=false
ALWAYS_SKIP=false
PROD_BRANCH="main"
while [[ "${1:-}" == --* ]]; do
  case "$1" in
    --skip-preview) SKIP_PREVIEW=true; shift ;;
    --always-skip) ALWAYS_SKIP=true; shift ;;
    --prod-branch) PROD_BRANCH="$2"; shift 2 ;;
    *) echo "Unknown option: $1" >&2; exit 1 ;;
  esac
done

if [ "$ALWAYS_SKIP" = true ]; then
  echo ">> Always-skip enabled — skipping build"
  type log_info >/dev/null 2>&1 && log_info "always-skip enabled, skipping build"
  type log_exit >/dev/null 2>&1 && log_exit 0
  exit 0
fi

if [ "$SKIP_PREVIEW" = true ]; then
  if [ -n "${VERCEL_ENV:-}" ] && [ "$VERCEL_ENV" != "production" ]; then
    echo "Skipping ${VERCEL_ENV} deployment"
    type log_info >/dev/null 2>&1 && log_info "skipping vercel env '${VERCEL_ENV}'"
    type log_exit >/dev/null 2>&1 && log_exit 0
    exit 0
  fi

  if [ -n "${VERCEL_GIT_COMMIT_REF:-}" ] && [ "$VERCEL_GIT_COMMIT_REF" != "$PROD_BRANCH" ]; then
    echo "Skipping non-production branch"
    type log_info >/dev/null 2>&1 && log_info "skipping non-production branch '${VERCEL_GIT_COMMIT_REF}'"
    type log_exit >/dev/null 2>&1 && log_exit 0
    exit 0
  fi
fi

PROJECT_DIR="${1:?Usage: vercel-ignore-build.sh [--skip-preview] <project-dir> [extra-path ...]}"
shift
SHARED_ROOT_PATHS=(
  "package.json"
  "pnpm-lock.yaml"
  "pnpm-workspace.yaml"
  "nx.json"
  ".nxignore"
  "tsconfig.base.json"
  ".npmrc"
  "tools/scripts/vercel-ignore-build.sh"
)
EXTRA_PATHS=("${SHARED_ROOT_PATHS[@]}" "$@")

# Ensure we work from the repo root regardless of where Vercel invokes us
cd "$REPO_ROOT"
type log_debug >/dev/null 2>&1 && log_debug "project_dir='${PROJECT_DIR}' repo_root='${REPO_ROOT}'"

# Vercel provides the SHA of the last successful deployment
if [ -z "${VERCEL_GIT_PREVIOUS_SHA:-}" ]; then
  echo ">> No previous deployment SHA found — building"
  type log_info >/dev/null 2>&1 && log_info "no previous SHA, triggering build"
  type log_exit >/dev/null 2>&1 && log_exit 1
  exit 1
fi

CURRENT_SHA="${VERCEL_GIT_COMMIT_SHA:-HEAD}"
type log_debug >/dev/null 2>&1 && log_debug "prev_sha=${VERCEL_GIT_PREVIOUS_SHA:0:8} current_sha=${CURRENT_SHA:0:8}"

echo ">> Checking for changes in '$PROJECT_DIR' between ${VERCEL_GIT_PREVIOUS_SHA:0:8} and ${CURRENT_SHA:0:8}"

# Get list of changed files between last deploy and current commit
# If diff cannot be computed (for example, shallow clone missing previous SHA),
# fail OPEN and build instead of silently skipping.
if ! CHANGED_FILES=$(git diff --name-only "$VERCEL_GIT_PREVIOUS_SHA" "$CURRENT_SHA" 2>/dev/null); then
  echo ">> Could not diff commits (likely shallow clone) — building"
  type log_warn >/dev/null 2>&1 && log_warn "git diff failed, triggering build"
  type log_exit >/dev/null 2>&1 && log_exit 1
  exit 1
fi

CHANGED_COUNT=$(echo "$CHANGED_FILES" | grep -c . 2>/dev/null || echo "0")
type log_debug >/dev/null 2>&1 && log_debug "total changed files: ${CHANGED_COUNT}"

if [ -z "$CHANGED_FILES" ]; then
  echo ">> No changes detected — skipping build"
  type log_info >/dev/null 2>&1 && log_info "no changes detected, skipping build"
  type log_exit >/dev/null 2>&1 && log_exit 0
  exit 0
fi

path_changed() {
  local path="${1%/}"
  echo "$CHANGED_FILES" | awk -v path="$path" '
    $0 == path || index($0, path "/") == 1 { found=1; exit }
    END { exit !found }
  '
}

# Check if any changed file is in the project directory
# Use awk for anchored literal prefix match (grep -F can't anchor to start-of-line)
if path_changed "$PROJECT_DIR"; then
  echo ">> Changes detected in $PROJECT_DIR — building"
  type log_info >/dev/null 2>&1 && log_info "changes in project dir, triggering build"
  type log_trace >/dev/null 2>&1 && log_trace "matching files: $(echo "$CHANGED_FILES" | awk -v prefix="${PROJECT_DIR}/" 'index($0, prefix) == 1' | head -5)"
  type log_exit >/dev/null 2>&1 && log_exit 1
  exit 1
fi

# Check extra watched paths (e.g. docs/public for docs-site)
for extra in "${EXTRA_PATHS[@]}"; do
  if path_changed "$extra"; then
    echo ">> Changes detected in extra watched path $extra — building"
    type log_info >/dev/null 2>&1 && log_info "extra path '${extra}' changed, triggering build"
    type log_exit >/dev/null 2>&1 && log_exit 1
    exit 1
  fi
done

echo ">> No relevant changes for $PROJECT_DIR — skipping build"
type log_info >/dev/null 2>&1 && log_info "no relevant changes, skipping build"
type log_exit >/dev/null 2>&1 && log_exit 0
exit 0
