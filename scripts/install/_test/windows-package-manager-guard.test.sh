#!/usr/bin/env bash
# Contract tests for the Windows package-manager dual-install guard (CIB-228/230)
# and powershell -File $Args safety (CIB-325).
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

# Happy path must NOT terminate the whole installer. This is the assertion
# that stands between us and the v0.9.2-beta regression, where the guard's
# `exit 0` killed the installer before cargo-dist's body ran and `irm | iex`
# silently did nothing on a clean machine.
#
# `[[:space:]]`, not `\s`: `\s` is a GNU extension, absent from POSIX ERE.
# BSD/macOS grep reads it as a literal `s`, so the pattern became
# `^s*exits+0s*$` and matched nothing — the guard reported ok with `exit 0`
# sitting in the file. Verified both ways before this change.
#
# The statement forms are enumerated rather than assumed: PowerShell accepts
# `exit 0`, `exit(0)`, a trailing `;`, and a trailing comment, and every one
# of them terminates the installer just as fatally as a bare `exit 0`. An
# earlier version matched only the bare form, so the regression could have
# returned as `exit 0;` and passed. Leading `#` is still safe — these very
# comments say "exit 0" and must not self-trip.
if grep -nE '^[[:space:]]*exit[[:space:]]*\(?[[:space:]]*0[[:space:]]*\)?[[:space:]]*([;#].*)?$' "$GUARD" >/dev/null; then
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
#
# Shape matches cargo-dist's generated installer: param, then later
# Set-StrictMode + Install-Binary "$Args". powershell -File leaves $Args
# unset after param; irm | iex inherits a caller $args.
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
fake_ps1="$tmp/eddacraft-anvil-installer.ps1"
cat >"$fake_ps1" <<'PS1'
# fake cargo-dist installer head
param (
    [switch]$NoModifyPath,
    [switch]$Help
)
function Install-Binary($install_args) {
    Write-Host "CARGO_DIST_BODY_REACHED"
}
Set-StrictMode -Version Latest
try {
  Install-Binary "$Args"
} catch {
  Write-Information $_
  exit 1
}
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
if grep -nE '(CIB|ADR|EVAL)-[0-9]+' "$fake_ps1" >/dev/null; then
  fail "assembled public installer must not contain internal tracker ids (CIB-230)"
fi

# Conceptual powershell -File run: $Args starts unset (no inherited caller
# $args). After param, Set-StrictMode makes a later "$Args" read throw
# unless the post-process initialised $Args or dropped the pass-through.
# Must reach the cargo-dist install body, not variable-not-set.
python3 - "$fake_ps1" <<'PY' || fail "assembled installer is not safe under powershell -File + Set-StrictMode (\$Args unset)"
import re
import sys
from pathlib import Path

text = Path(sys.argv[1]).read_text(encoding="utf-8")
match = re.search(r"(?ms)^(\s*param\s*\(.*?\))\s*\r?\n", text)
if not match:
    print("conceptual -File run: no param block", file=sys.stderr)
    sys.exit(1)
rest = text[match.end() :]
args_defined = False
strict = False
reached_body = False


def code_only(line: str) -> str:
    # Strip unquoted comments. Quoted # is not a comment.
    out = []
    in_squote = False
    in_dquote = False
    i = 0
    while i < len(line):
        ch = line[i]
        if ch == "'" and not in_dquote:
            in_squote = not in_squote
            out.append(ch)
        elif ch == '"' and not in_squote:
            in_dquote = not in_dquote
            out.append(ch)
        elif ch == "#" and not in_squote and not in_dquote:
            break
        else:
            out.append(ch)
        i += 1
    return "".join(out)


for raw in rest.splitlines():
    line = code_only(raw)
    if re.search(r"(?i)Set-StrictMode\b", line):
        strict = True
    if re.search(r"(?i)\$Args\s*=", line):
        args_defined = True
    if re.search(r"(?i)CARGO_DIST_BODY_REACHED", line):
        reached_body = True
    # A read of $Args (not an assignment) under StrictMode with $Args unset
    # is the -File failure mode.
    if (
        strict
        and not args_defined
        and re.search(r"(?i)\$Args\b", line)
        and not re.search(r"(?i)\$Args\s*=", line)
    ):
        print(
            "conceptual -File run: $Args read under Set-StrictMode while unset:",
            line.strip(),
            file=sys.stderr,
        )
        sys.exit(1)
    if re.search(r"(?i)\bInstall-Binary\b", line) and not re.search(
        r"(?i)\bfunction\s+Install-Binary\b", line
    ):
        reached_body = True

if not reached_body:
    print("conceptual -File run: never reached install body", file=sys.stderr)
    sys.exit(1)
print("conceptual -File run: reached install body")
PY

echo "windows-package-manager-guard.test.sh: ok"
