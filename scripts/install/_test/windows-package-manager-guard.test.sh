#!/usr/bin/env bash
# Contract tests for the Windows package-manager dual-install guard (CIB-228/230).
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
GUARD="$ROOT/scripts/install/windows-package-manager-guard.ps1"
INJECT="$ROOT/scripts/install/inject-windows-pm-guard.py"
RELEASE_YML="$ROOT/.github/workflows/release.yml"

fail() {
  echo "windows-package-manager-guard.test.sh: $*" >&2
  exit 1
}

[[ -f "$GUARD" ]] || fail "guard script missing: $GUARD"
[[ -f "$INJECT" ]] || fail "inject script missing: $INJECT"
[[ -f "$RELEASE_YML" ]] || fail "release workflow missing: $RELEASE_YML"

# CIB-230: public ship artefacts must not embed private tracker ids.
#
# The guard ships into eddacraft-anvil-installer.ps1 comments and all, and the
# inject banner is written straight into it, so this covers comments — not just
# user-facing strings. CIB-315 widened the pattern: the original check caught
# only `GH #N` / `GitHub #N`, so `CIB-NNN` survived in shipped comments through
# v0.9.3-beta and was reported back to us from a beta estate.
for public_file in "$GUARD" "$INJECT"; do
  if grep -nE 'GH[[:space:]]*#[0-9]+|GitHub[[:space:]]*#[0-9]+' "$public_file" >/dev/null; then
    fail "public install path must not contain GH/GitHub #NNNN (CIB-230): $public_file"
  fi
  # No `\b`: it is a GNU extension, not POSIX ERE. BSD/macOS grep reads it as
  # a literal `b`, which would demand a `b` before the id and quietly match
  # nothing — a guard that cannot fail, which is the defect class this check
  # exists to catch. Matching anywhere in the line is also the stricter
  # reading: no internal id belongs in a shipped artefact in any position.
  if grep -nE '(CIB|ADR|EVAL)-[0-9]+' "$public_file" >/dev/null; then
    fail "public install path must not contain internal tracker ids (CIB-230): $public_file"
  fi
done

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

python3 "$INJECT" "$fake_ps1" "$GUARD"

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

# CIB-230: assembled public installer body forbids private tracker ids.
if grep -nE 'GH[[:space:]]*#[0-9]+|GitHub[[:space:]]*#[0-9]+' "$fake_ps1" >/dev/null; then
  fail "assembled public installer must not contain GH/GitHub #NNNN (CIB-230)"
fi

echo "windows-package-manager-guard.test.sh: ok"
