#!/usr/bin/env bash
# ATTRIB-004 bundled-binaries driver preflight + validation test.
#
# Invokes `drivers/bundled-binaries.sh` directly (no dispatcher). Each
# scenario asserts a non-zero exit AND an actionable stderr substring.
#
# Scenarios:
#   1. Wrong argv count (driver-author contract: exactly two args)
#   2. inventory_path file does not exist
#   3. inventory has a [[binary]] entry missing a required field (spdx)
#   4. inventory has no [[binary]] entries at all (empty inventory)
#
# Local invocation:
#   tools/starters/acknowledgements/tests/bundled-binaries-preflight.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
DRIVER="$SCRIPT_DIR/../drivers/bundled-binaries.sh"

if [ ! -x "$DRIVER" ]; then
  echo "error: driver script not found or not executable at $DRIVER" >&2
  exit 1
fi

fixture_root="$(mktemp -d)"
trap 'rm -rf "$fixture_root"' EXIT

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

# ── Scenario 2: inventory_path does not exist ────────────────────────
config2="$(jq -n --arg inv "$fixture_root/no-such-inventory.toml" \
  '{name:"binaries", ecosystem:"bundled-binaries", inventory_path:$inv}')"
exit2=0
out2="$("$DRIVER" "$config2" "$fixture_root/out2.md" 2>&1 >/dev/null)" || exit2=$?
if [ "$exit2" -eq 0 ]; then
  echo "fail scenario 2: driver accepted nonexistent inventory_path" >&2
  exit 1
fi
if ! printf '%s' "$out2" | grep -q "inventory_path"; then
  echo "fail scenario 2: stderr lacks 'inventory_path' (got: $out2)" >&2
  exit 1
fi
echo "ok scenario 2: missing inventory_path rejected (exit $exit2)"

# ── Scenario 3: entry missing a required field (spdx) ────────────────
bad_inv="$fixture_root/missing-spdx.toml"
cat >"$bad_inv" <<'EOF'
[[binary]]
name = "OpenSSH"
version = "9.6p1"
source = "https://www.openssh.com/"
EOF
config3="$(jq -n --arg inv "$bad_inv" \
  '{name:"binaries", ecosystem:"bundled-binaries", inventory_path:$inv}')"
exit3=0
out3="$("$DRIVER" "$config3" "$fixture_root/out3.md" 2>&1 >/dev/null)" || exit3=$?
if [ "$exit3" -eq 0 ]; then
  echo "fail scenario 3: driver accepted an entry with no spdx field" >&2
  exit 1
fi
if ! printf '%s' "$out3" | grep -qi "spdx"; then
  echo "fail scenario 3: stderr lacks a 'spdx' hint (got: $out3)" >&2
  exit 1
fi
# The error should name the offending binary so the curator can fix it.
if ! printf '%s' "$out3" | grep -q "OpenSSH"; then
  echo "fail scenario 3: error did not name the offending entry OpenSSH (got: $out3)" >&2
  exit 1
fi
echo "ok scenario 3: entry missing required spdx rejected, names the entry (exit $exit3)"

# ── Scenario 4: inventory with no [[binary]] entries ─────────────────
empty_inv="$fixture_root/empty.toml"
printf '# no binaries bundled yet\n' >"$empty_inv"
config4="$(jq -n --arg inv "$empty_inv" \
  '{name:"binaries", ecosystem:"bundled-binaries", inventory_path:$inv}')"
exit4=0
out4="$("$DRIVER" "$config4" "$fixture_root/out4.md" 2>&1 >/dev/null)" || exit4=$?
if [ "$exit4" -eq 0 ]; then
  echo "fail scenario 4: driver accepted an empty inventory (would splice an empty block)" >&2
  exit 1
fi
if ! printf '%s' "$out4" | grep -qiE "no .*binar|empty"; then
  echo "fail scenario 4: stderr lacks an empty-inventory hint (got: $out4)" >&2
  exit 1
fi
echo "ok scenario 4: empty inventory rejected (exit $exit4)"

echo
echo "ATTRIB-004 bundled-binaries preflight tests passed: 4/4 scenarios green."
