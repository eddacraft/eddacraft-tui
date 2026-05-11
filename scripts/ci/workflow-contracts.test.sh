#!/usr/bin/env bash
# CICD-010: lock the workflow → contract map.
#
# Every `.github/workflows/*.yml` file (excluding `*.example`) must appear
# in the Workflow Contract Map table in `.github/workflows/README.md`.
# This fixture catches:
#   - new workflows added without a contract entry
#   - workflow files renamed/removed without a README update
#   - the contract map drifting away from the on-disk reality

set -euo pipefail

repo_root=$(git rev-parse --show-toplevel 2>/dev/null || pwd)
workflows_dir="${repo_root}/.github/workflows"
readme="${workflows_dir}/README.md"

if [ ! -f "${readme}" ]; then
  echo "expected ${readme} to exist" >&2
  exit 1
fi

# The README must declare the contract-map header so the test fails loud
# if the section is renamed or deleted.
if ! grep -Fq -- '### Workflow Contract Map (CICD-010)' "${readme}"; then
  echo "expected ${readme} to contain '### Workflow Contract Map (CICD-010)'" >&2
  exit 1
fi

# Walk every YAML file under workflows/, skipping the example. Assert
# each base name (`<file>.yml`) appears at least once inside a backtick
# in the README. The README per-file detail headings (`### \`<file>.yml\``)
# satisfy this, as do the contract-map table cells (`\`<file>.yml\``).
missing=()
while IFS= read -r path; do
  name=$(basename "${path}")
  case "${name}" in
    *.example | *.example.yml) continue ;;
  esac
  if ! grep -Fq -- "\`${name}\`" "${readme}"; then
    missing+=("${name}")
  fi
done < <(find "${workflows_dir}" -maxdepth 1 -type f \( -name '*.yml' -o -name '*.yaml' \) | sort)

if [ "${#missing[@]}" -gt 0 ]; then
  echo "workflow files not referenced in ${readme}:" >&2
  for name in "${missing[@]}"; do
    echo "  - ${name}" >&2
  done
  echo "add an entry to the Workflow Contract Map table." >&2
  exit 1
fi

# The contract map must explicitly name each of the five spec contracts at
# least once (so the table cannot lose a contract row by accident).
for contract in 'PR validation' 'Integration push' 'Assurance' 'Release candidate' 'Publish'; do
  if ! grep -Fq -- "${contract}" "${readme}"; then
    echo "expected ${readme} to name contract '${contract}' (CI/CD operating-model MVP contracts)" >&2
    exit 1
  fi
done

# Spot-check that the authority-audit section still calls out the
# previously-deduplicated surface so a regression that re-introduces
# `Dependency Audit (PR)` on push events surfaces in review.
if ! grep -Fq -- 'CICD-005 gated' "${readme}"; then
  echo "expected ${readme} authority-audit to credit CICD-005 for the dependency-audit gate" >&2
  exit 1
fi

echo 'workflow-contract map checks passed'
