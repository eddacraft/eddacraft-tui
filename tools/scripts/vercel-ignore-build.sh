#!/bin/bash
# Vercel Ignored Build Step
#
# Skips builds when no files in the project's directory (or shared config)
# have changed since the last deployment. Vercel sets VERCEL_GIT_PREVIOUS_SHA
# to the last successfully deployed commit.
#
# Usage (set as ignoreCommand in Vercel project config):
#   bash tools/scripts/vercel-ignore-build.sh apps/website
#   bash tools/scripts/vercel-ignore-build.sh apps/docs-site
#   bash tools/scripts/vercel-ignore-build.sh apps/anvil-api
#
# Vercel ignoreCommand semantics:
#   Exit 1 = proceed with build
#   Exit 0 = skip build (cancel deployment)

set -euo pipefail

PROJECT_DIR="${1:?Usage: vercel-ignore-build.sh <project-dir>}"

# Shared paths that should trigger a rebuild for any project
SHARED_PATHS="pnpm-lock.yaml package.json"

# Ensure we work from the repo root regardless of where Vercel invokes us
REPO_ROOT=$(git rev-parse --show-toplevel)
cd "$REPO_ROOT"

# Vercel provides the SHA of the last successful deployment
if [ -z "${VERCEL_GIT_PREVIOUS_SHA:-}" ]; then
  echo ">> No previous deployment SHA found — building"
  exit 1
fi

CURRENT_SHA="${VERCEL_GIT_COMMIT_SHA:-HEAD}"

echo ">> Checking for changes in '$PROJECT_DIR' between ${VERCEL_GIT_PREVIOUS_SHA:0:8} and ${CURRENT_SHA:0:8}"

# Get list of changed files between last deploy and current commit
CHANGED_FILES=$(git diff --name-only "$VERCEL_GIT_PREVIOUS_SHA" "$CURRENT_SHA" 2>/dev/null || true)

if [ -z "$CHANGED_FILES" ]; then
  echo ">> No changes detected — skipping build"
  exit 0
fi

# Check if any changed file is in the project directory
if echo "$CHANGED_FILES" | grep -q "^${PROJECT_DIR}/"; then
  echo ">> Changes detected in $PROJECT_DIR — building"
  exit 1
fi

# Check shared paths
for path in $SHARED_PATHS; do
  if echo "$CHANGED_FILES" | grep -q "^${path}$"; then
    echo ">> Shared config changed ($path) — building"
    exit 1
  fi
done

echo ">> No relevant changes for $PROJECT_DIR — skipping build"
exit 0
