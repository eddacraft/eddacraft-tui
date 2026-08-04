#!/usr/bin/env bash
# Contract tests for the Windows package-manager dual-install guard (#2885 / CIB-228).
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
GUARD="$ROOT/scripts/install/windows-package-manager-guard.ps1"
RELEASE_YML="$ROOT/.github/workflows/release.yml"

fail() {
  echo "windows-package-manager-guard.test.sh: $*" >&2
  exit 1
}

[[ -f "$GUARD" ]] || fail "guard script missing: $GUARD"
[[ -f "$RELEASE_YML" ]] || fail "release workflow missing: $RELEASE_YML"

for needle in \
  'Get-Command anvil -All' \
  'winget upgrade --id eddacraft.anvil' \
  'scoop update anvil' \
  'WindowsApps' \
  'scoop\\shims' \
  'exit 2' \
  'ANVIL_INSTALL_FORCE' \
  'fall through'
do
  grep -Fq -- "$needle" "$GUARD" || fail "guard missing required content: $needle"
done

# Happy path must NOT terminate the whole installer.
if grep -nE '^\s*exit\s+0\s*$' "$GUARD" >/dev/null; then
  fail "guard must not use exit 0 (terminates cargo-dist body when injected)"
fi

# Must not introduce a second *script-level* param block for inject-after-param.
# Function-scoped `param([string]$Path)` is fine; only a top-level param(
# at column 0 / before any function is forbidden.
if grep -nE '^[ \t]*param[ \t]*\(' "$GUARD" | grep -v 'function ' >/dev/null; then
  # Allow param only when indented under a function (previous non-empty line
  # contains `function` or we are inside a function body with leading spaces
  # after function). Simpler: ban unindented `param (` at BOL.
  if grep -nE '^param[ \t]*\(' "$GUARD" >/dev/null; then
    fail "guard must not declare top-level param(...) — inject after cargo-dist param only"
  fi
fi

# Pure path classifiers must match production markers used by anvil version.
grep -Eq 'WindowsApps|WinGet' "$GUARD" || fail "WinGet path markers missing"
grep -Eq 'scoop' "$GUARD" || fail "Scoop path markers missing"

# Release inject must place the guard *after* the cargo-dist param block, not
# as a blind prepend that leaves two param blocks / early exit 0.
grep -Fq 'Inject Windows package-manager dual-install guard' "$RELEASE_YML" \
  || grep -Fq 'package-manager dual-install guard' "$RELEASE_YML" \
  || fail "release.yml missing dual-install guard inject step"
grep -Fq 'insert_after_param' "$RELEASE_YML" \
  || grep -Fq 'after cargo-dist param' "$RELEASE_YML" \
  || fail "release.yml must inject after cargo-dist param (CIB-228)"

# Fixture: assemble a fake cargo-dist installer the way release does and assert
# the cargo-dist body remains reachable (no early exit 0 in the prefix).
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
fake_ps1="$tmp/eddacraft-anvil-installer.ps1"
cat >"$fake_ps1" <<'PS1'
# fake cargo-dist installer head
param (
    [switch]$NoModifyPath,
    [switch]$Help
)
Write-Host "CARGO_DIST_BODY_REACHED"
PS1

# Simulate inject-after-param: keep param, then guard, then rest of body.
python3 - "$GUARD" "$fake_ps1" <<'PY'
import re, sys
from pathlib import Path
guard = Path(sys.argv[1]).read_text()
ps1 = Path(sys.argv[2]).read_text()
# Match first param (...) block non-greedily across newlines
m = re.search(r"(?ms)^(\s*param\s*\(.*?\))\s*\r?\n", ps1)
if not m:
    raise SystemExit("fake installer has no param block")
head, rest = m.group(0), ps1[m.end() :]
out = (
    head
    + "\n# --- begin anvil package-manager dual-install guard (CIB-228) ---\n"
    + guard
    + "\n# --- end anvil package-manager dual-install guard ---\n\n"
    + rest
)
Path(sys.argv[2]).write_text(out)
PY

# Assembled file: single *top-level* param (cargo-dist), guard present, body
# marker present, no exit 0 before body. Function-level param() in the guard
# is allowed and expected.
top_level_params=$(grep -cE '^param[ \t]*\(' "$fake_ps1" || true)
[[ "$top_level_params" -eq 1 ]] \
  || fail "assembled installer must have exactly one top-level param block (got $top_level_params)"
grep -Fq 'CARGO_DIST_BODY_REACHED' "$fake_ps1" || fail "cargo-dist body marker missing after inject"
# exit 0 must not appear before the body marker
body_line=$(grep -n 'CARGO_DIST_BODY_REACHED' "$fake_ps1" | head -1 | cut -d: -f1)
if awk -v n="$body_line" 'NR<n && /^\s*exit\s+0\s*$/ { found=1 } END { exit found?0:1 }' "$fake_ps1"; then
  fail "exit 0 appears before cargo-dist body in assembled installer"
fi

echo "windows-package-manager-guard.test.sh: ok"
