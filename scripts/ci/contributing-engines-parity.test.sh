#!/usr/bin/env bash
# DEVENV-010: CONTRIBUTING's stated toolchain floors must match `engines`.
#
# These drifted apart and the onboarding doc lost: `engines` moved to node
# >=24 / pnpm >=11 while CONTRIBUTING still said node >=22.13.0 /
# pnpm >=10.20.0, and git was never listed at all. A contributor following the
# documented setup installed Node 22 — on which pnpm 11 cannot run — and
# landed straight in the failure the dev-environment module exists to prevent.
#
# The drift came from the `engines` side, so this asserts in that direction:
# every floor `package.json` declares must appear in CONTRIBUTING with the same
# range. A floor that exists only in the doc is reported too, since that is the
# same defect mirrored.

set -euo pipefail

repo_root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)
contributing="${repo_root}/CONTRIBUTING.md"
manifest="${repo_root}/package.json"

for f in "${contributing}" "${manifest}"; do
  [ -f "${f}" ] || {
    echo "FAIL: ${f} not found" >&2
    exit 1
  }
done

# Engine key -> the label CONTRIBUTING uses for it. Bare keys are matched
# case-insensitively, so `git` matches a "**Git**:" bullet.
label_for() {
  case "$1" in
    node) echo 'Node\.js' ;;
    *) echo "$1" ;;
  esac
}

engines=$(node -e '
  const e = require(process.argv[1]).engines || {};
  for (const [k, v] of Object.entries(e)) console.log(`${k}\t${v}`);
' "${manifest}")

[ -n "${engines}" ] || {
  echo "FAIL: package.json declares no engines; this check has nothing to guard" >&2
  exit 1
}

failures=0
while IFS=$'\t' read -r key range; do
  [ -n "${key}" ] || continue
  label=$(label_for "${key}")

  # A prerequisite bullet: `- **Node.js**: >=24.0.0`, tolerant of emphasis and
  # spacing so the check tracks meaning rather than one exact rendering.
  if ! grep -qiE "^[[:space:]]*[-*][[:space:]]+\**${label}\**[[:space:]]*:?[[:space:]]*\`?${range//./\\.}" "${contributing}"; then
    stated=$(grep -iE "^[[:space:]]*[-*][[:space:]]+\**${label}\**[[:space:]]*:" "${contributing}" | head -1 | sed 's/^[[:space:]]*//' || true)
    if [ -n "${stated}" ]; then
      echo "FAIL: CONTRIBUTING states a different floor for '${key}'." >&2
      echo "      engines:      ${range}" >&2
      echo "      CONTRIBUTING: ${stated}" >&2
    else
      echo "FAIL: CONTRIBUTING does not state a floor for '${key}' (engines: ${range})." >&2
    fi
    failures=$((failures + 1))
  fi
done <<<"${engines}"

if [ "${failures}" -ne 0 ]; then
  echo "" >&2
  echo "${failures} toolchain floor(s) disagree between package.json engines and CONTRIBUTING.md." >&2
  echo "Update CONTRIBUTING's Prerequisites so a fresh clone follows a path that works." >&2
  exit 1
fi

echo 'contributing/engines parity ok'
