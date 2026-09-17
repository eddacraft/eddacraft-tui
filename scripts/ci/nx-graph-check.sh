#!/usr/bin/env bash
# CIB-320: compute the Nx project graph with a cold cache.
# CIB-431: install EXIT cleanup immediately after the first temp directory
# is created so a later mktemp failure cannot leak it.
#
# A warm `.nx/workspace-data` restored from an earlier run can mask an
# unnamed inferred project. Vercel `nx build` then fails after merge.
# Force a cold graph so the defect fails in CI instead of production.

set -euo pipefail

repo_root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)
cd "${repo_root}"

workspace_data=$(mktemp -d)
cache_dir=""
cleanup() {
  rm -rf "${workspace_data}"
  if [[ -n "${cache_dir}" ]]; then
    rm -rf "${cache_dir}"
  fi
}
trap cleanup EXIT
cache_dir=$(mktemp -d)

export NX_DAEMON=false
export NX_WORKSPACE_DATA_DIRECTORY="${workspace_data}"
export NX_CACHE_DIRECTORY="${cache_dir}"

# Nx writes the unnamed-project error to stderr; leave it unfiltered so a
# failing run surfaces the diagnostic verbatim.
#
# Prefer the installed binary so an isolated copy can compute the graph
# without `pnpm exec` treating a different workspace path as stale and
# trying to rebuild node_modules. Fall back to pnpm when the binary is
# missing (a fresh checkout before install).
nx_bin="${repo_root}/node_modules/.bin/nx"
if [[ -x "${nx_bin}" ]]; then
  "${nx_bin}" show projects
else
  pnpm exec nx show projects
fi
