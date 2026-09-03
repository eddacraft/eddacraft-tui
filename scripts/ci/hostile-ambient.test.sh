#!/usr/bin/env bash
# CIB-391: the hostile-ambient nightly job must exist, name the profile, and
# stay off the per-PR workflow. This does not run the suite — a red nightly
# is expected until ambient-dependent tests are fixed elsewhere.
set -euo pipefail

repo_root=$(git rev-parse --show-toplevel 2>/dev/null || pwd)
nightly="${repo_root}/.github/workflows/ci-nightly.yml"
pr_ci="${repo_root}/.github/workflows/ci.yml"
script="${repo_root}/scripts/ci/hostile-ambient.sh"

if [[ ! -f "$nightly" ]]; then
  echo "expected ${nightly}" >&2
  exit 1
fi
if [[ ! -f "$script" ]]; then
  echo "expected ${script}" >&2
  exit 1
fi

if ! grep -Fq -- 'name: Hostile ambient (umask 002, privacy opt-out, live daemon)' "$nightly"; then
  echo "ci-nightly.yml must declare the hostile-ambient job name" >&2
  exit 1
fi
if ! grep -Fq -- 'bash scripts/ci/hostile-ambient.sh' "$nightly"; then
  echo "ci-nightly.yml must invoke scripts/ci/hostile-ambient.sh" >&2
  exit 1
fi
if ! grep -Fq -- 'umask 002' "$script"; then
  echo "hostile-ambient.sh must set umask 002" >&2
  exit 1
fi
if ! grep -Fq -- 'DO_NOT_TRACK=1' "$script"; then
  echo "hostile-ambient.sh must set DO_NOT_TRACK" >&2
  exit 1
fi
if ! grep -Fq -- 'intercept start' "$script"; then
  echo "hostile-ambient.sh must start a live intercept daemon" >&2
  exit 1
fi
if grep -Fq -- 'hostile-ambient' "$pr_ci"; then
  echo "hostile-ambient must not be a per-PR ci.yml job" >&2
  exit 1
fi

echo "hostile-ambient nightly contract ok"
