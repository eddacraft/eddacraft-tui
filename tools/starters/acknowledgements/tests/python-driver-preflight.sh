#!/usr/bin/env bash
# Python-driver preflight test.
#
# Exercises the driver-author contract rule 1 (actionable errors for
# missing tools or missing state). Invokes `drivers/python.sh` directly
# — no real venv, no pip-licenses, no network — so this test runs fast
# everywhere. Each scenario asserts a non-zero exit AND a substring
# match in stderr that proves the error is actionable.
#
# Scenarios:
#   1. Wrong argv count (driver-author contract: exactly two args)
#   2. venv_path directory does not exist
#   3. venv exists but contains no pip-licenses
#   4. python_allow_path file does not exist
#   5. venv has pip-licenses but the allow-list is empty between markers
#
# Local invocation:
#   tools/starters/acknowledgements/tests/python-driver-preflight.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
DRIVER="$SCRIPT_DIR/../drivers/python.sh"

if [ ! -x "$DRIVER" ]; then
  echo "error: driver script not found or not executable at $DRIVER" >&2
  exit 1
fi

fixture_root="$(mktemp -d)"
trap 'rm -rf "$fixture_root"' EXIT

allow_list="$fixture_root/licences.python-allow.txt"
printf '# BEGIN AUTO-GENERATED FROM licences.toml — python-allow\nMIT;Apache-2.0\n# END AUTO-GENERATED FROM licences.toml — python-allow\n' >"$allow_list"

# A venv-shaped dir with a stub pip-licenses for scenarios that must get
# past the "venv contains pip-licenses" check. The stub never executes
# in these scenarios — they fail at a file/state check first.
good_venv="$fixture_root/venv"
mkdir -p "$good_venv/bin"
cat >"$good_venv/bin/pip-licenses" <<'STUB'
#!/usr/bin/env bash
echo "stub pip-licenses invoked — preflight tests should not reach it" >&2
exit 99
STUB
chmod +x "$good_venv/bin/pip-licenses"

# ── Scenario 1: wrong argv count ─────────────────────────────────────
exit1=0
out1="$("$DRIVER" 2>&1 >/dev/null)" || exit1=$?
if [ "$exit1" -eq 0 ]; then
  echo "fail scenario 1: driver accepted 0 arguments instead of erroring" >&2
  exit 1
fi
if ! printf '%s' "$out1" | grep -q "expected 2 arguments"; then
  echo "fail scenario 1: stderr lacks 'expected 2 arguments' (got: $out1)" >&2
  exit 1
fi
echo "ok scenario 1: wrong argv count rejected (exit $exit1)"

# ── Scenario 2: venv_path directory does not exist ───────────────────
config2="$(jq -n --arg v "$fixture_root/no-such-venv" --arg a "$allow_list" \
  '{name:"python", ecosystem:"python", venv_path:$v, python_allow_path:$a}')"
exit2=0
out2="$("$DRIVER" "$config2" "$fixture_root/out2.md" 2>&1 >/dev/null)" || exit2=$?
if [ "$exit2" -eq 0 ]; then
  echo "fail scenario 2: driver accepted nonexistent venv_path" >&2
  exit 1
fi
if ! printf '%s' "$out2" | grep -q "venv_path"; then
  echo "fail scenario 2: stderr lacks 'venv_path' (got: $out2)" >&2
  exit 1
fi
echo "ok scenario 2: missing venv_path rejected (exit $exit2)"

# ── Scenario 3: venv exists but no pip-licenses ──────────────────────
empty_venv="$fixture_root/empty-venv"
mkdir -p "$empty_venv/bin"
config3="$(jq -n --arg v "$empty_venv" --arg a "$allow_list" \
  '{name:"python", ecosystem:"python", venv_path:$v, python_allow_path:$a}')"
exit3=0
out3="$("$DRIVER" "$config3" "$fixture_root/out3.md" 2>&1 >/dev/null)" || exit3=$?
if [ "$exit3" -eq 0 ]; then
  echo "fail scenario 3: driver accepted a venv without pip-licenses" >&2
  exit 1
fi
if ! printf '%s' "$out3" | grep -q "pip-licenses"; then
  echo "fail scenario 3: stderr lacks 'pip-licenses' (got: $out3)" >&2
  exit 1
fi
echo "ok scenario 3: missing pip-licenses in venv rejected (exit $exit3)"

# ── Scenario 4: python_allow_path file does not exist ────────────────
config4="$(jq -n --arg v "$good_venv" --arg a "$fixture_root/no-allow.txt" \
  '{name:"python", ecosystem:"python", venv_path:$v, python_allow_path:$a}')"
exit4=0
out4="$("$DRIVER" "$config4" "$fixture_root/out4.md" 2>&1 >/dev/null)" || exit4=$?
if [ "$exit4" -eq 0 ]; then
  echo "fail scenario 4: driver accepted nonexistent python_allow_path" >&2
  exit 1
fi
if ! printf '%s' "$out4" | grep -q "python_allow_path"; then
  echo "fail scenario 4: stderr lacks 'python_allow_path' (got: $out4)" >&2
  exit 1
fi
echo "ok scenario 4: missing python_allow_path rejected (exit $exit4)"

# ── Scenario 5: allow-list empty between the markers ─────────────────
empty_allow="$fixture_root/empty-allow.txt"
printf '# BEGIN AUTO-GENERATED FROM licences.toml — python-allow\n# END AUTO-GENERATED FROM licences.toml — python-allow\n' >"$empty_allow"
config5="$(jq -n --arg v "$good_venv" --arg a "$empty_allow" \
  '{name:"python", ecosystem:"python", venv_path:$v, python_allow_path:$a}')"
exit5=0
out5="$("$DRIVER" "$config5" "$fixture_root/out5.md" 2>&1 >/dev/null)" || exit5=$?
if [ "$exit5" -eq 0 ]; then
  echo "fail scenario 5: driver accepted an empty allow-list" >&2
  exit 1
fi
if ! printf '%s' "$out5" | grep -qi "empty"; then
  echo "fail scenario 5: stderr lacks an 'empty' allow-list hint (got: $out5)" >&2
  exit 1
fi
echo "ok scenario 5: empty allow-list rejected (exit $exit5)"

echo
echo "Python-driver preflight tests passed: 5/5 scenarios green."
