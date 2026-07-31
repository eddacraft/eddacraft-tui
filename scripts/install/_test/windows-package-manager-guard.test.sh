#!/usr/bin/env bash
# Contract tests for the Windows package-manager dual-install guard (#2885).
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
GUARD="$ROOT/scripts/install/windows-package-manager-guard.ps1"

fail() {
  echo "windows-package-manager-guard.test.sh: $*" >&2
  exit 1
}

[[ -f "$GUARD" ]] || fail "guard script missing: $GUARD"

for needle in \
  'Get-Command anvil -All' \
  'winget upgrade --id eddacraft.anvil' \
  'scoop update anvil' \
  'WindowsApps' \
  'scoop\\shims' \
  'exit 2' \
  'Force'
do
  grep -Fq -- "$needle" "$GUARD" || fail "guard missing required content: $needle"
done

# Pure path classifiers must match production markers used by anvil version.
grep -Eq 'WindowsApps|WinGet' "$GUARD" || fail "WinGet path markers missing"
grep -Eq 'scoop' "$GUARD" || fail "Scoop path markers missing"

echo "windows-package-manager-guard.test.sh: ok"
