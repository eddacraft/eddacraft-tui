#!/usr/bin/env bash
# CIB-320: compute the Nx project graph with a cold cache.
#
# A warm `.nx/workspace-data` restored from an earlier run can mask an
# unnamed inferred project. Vercel `nx build` then fails after merge.
# Force a cold graph so the defect fails in CI instead of production.

set -euo pipefail

repo_root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)
cd "${repo_root}"

workspace_data=$(mktemp -d)
cache_dir=$(mktemp -d)
cleanup() {
  rm -rf "${workspace_data}" "${cache_dir}"
}
trap cleanup EXIT

export NX_DAEMON=false
export NX_WORKSPACE_DATA_DIRECTORY="${workspace_data}"
export NX_CACHE_DIRECTORY="${cache_dir}"

# Nx writes the unnamed-project error to stderr; leave it unfiltered so a
# failing run surfaces the diagnostic verbatim.
pnpm exec nx show projects
