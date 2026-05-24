#!/usr/bin/env bash
# ATTRIB-012 Node-driver preflight test.
#
# Exercises the driver-author contract rule 1 (actionable errors for
# missing tools or missing state). Invokes `drivers/node.sh` directly
# — no dispatcher, no npm install — so this test runs fast and does
# not require `license-checker` or the network. Each scenario asserts
# a non-zero exit AND a substring match in stderr that proves the
# error is actionable (names the missing thing / hints at the fix).
#
# Scenarios:
#   1. Wrong argv count (driver-author contract: exactly two args)
#   2. manifest_path file does not exist
#   3. manifest_path exists but no node_modules sibling
#      (consumer forgot `npm install` / `pnpm install`)
#   4. license-checker not on PATH (controlled PATH)
#
# Local invocation:
#   tools/starters/acknowledgements/tests/node-driver-preflight.sh
#
# CI wires this into the Acknowledgements freshness job (see
# .github/workflows/rust.yml).

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
DRIVER="$SCRIPT_DIR/../drivers/node.sh"

if [ ! -x "$DRIVER" ]; then
  echo "error: driver script not found or not executable at $DRIVER" >&2
  exit 1
fi

fixture_root="$(mktemp -d)"
trap 'rm -rf "$fixture_root"' EXIT

# Build a minimal allow-list file used by every scenario that gets
# past preflight — preflight failures hit before allow-list is read,
# but make the file exist so test failures are unambiguous (we want
# preflight to be the reason, not a missing allow-list).
allow_list="$fixture_root/licences.node-allow.txt"
printf 'MIT;Apache-2.0\n' >"$allow_list"

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

# ── Scenario 2: manifest_path file does not exist ────────────────────
config2_json="$(jq -n \
  --arg manifest "$fixture_root/does-not-exist/package.json" \
  --arg allow    "$allow_list" \
  '{name: "node", ecosystem: "node", manifest_path: $manifest, node_allow_path: $allow, prod_only: true}')"

exit2=0
out2="$("$DRIVER" "$config2_json" "$fixture_root/out2.md" 2>&1 >/dev/null)" || exit2=$?
if [ "$exit2" -eq 0 ]; then
  echo "fail scenario 2: driver accepted nonexistent manifest_path" >&2
  exit 1
fi
if ! printf '%s' "$out2" | grep -q "manifest_path"; then
  echo "fail scenario 2: stderr lacks 'manifest_path' (got: $out2)" >&2
  exit 1
fi
echo "ok scenario 2: missing manifest_path rejected (exit $exit2)"

# ── Scenario 3: manifest exists, node_modules sibling missing ────────
pkg3_dir="$fixture_root/pkg3"
mkdir -p "$pkg3_dir"
cat >"$pkg3_dir/package.json" <<'EOF'
{
  "name": "fixture-pkg3",
  "version": "0.0.0",
  "private": true,
  "license": "UNLICENSED"
}
EOF

config3_json="$(jq -n \
  --arg manifest "$pkg3_dir/package.json" \
  --arg allow    "$allow_list" \
  '{name: "node", ecosystem: "node", manifest_path: $manifest, node_allow_path: $allow, prod_only: true}')"

exit3=0
out3="$("$DRIVER" "$config3_json" "$fixture_root/out3.md" 2>&1 >/dev/null)" || exit3=$?
if [ "$exit3" -eq 0 ]; then
  echo "fail scenario 3: driver accepted missing node_modules" >&2
  exit 1
fi
if ! printf '%s' "$out3" | grep -q "node_modules"; then
  echo "fail scenario 3: stderr lacks 'node_modules' (got: $out3)" >&2
  exit 1
fi
if ! printf '%s' "$out3" | grep -qiE "npm install|pnpm install|yarn install"; then
  echo "fail scenario 3: stderr lacks an installer hint (got: $out3)" >&2
  exit 1
fi
echo "ok scenario 3: missing node_modules rejected with installer hint (exit $exit3)"

# ── Scenario 4: license-checker not on PATH ──────────────────────────
# Plant a node_modules dir so the node_modules preflight passes;
# license-checker preflight must still fail because we strip it from
# PATH. We need jq on PATH for the driver to parse its config arg.
mkdir -p "$pkg3_dir/node_modules"

# Locate jq and bash by absolute path; build a PATH that contains
# those but NOT license-checker.
jq_bin="$(command -v jq)"
jq_dir="$(dirname "$jq_bin")"
bash_bin="$(command -v bash)"
bash_dir="$(dirname "$bash_bin")"

# Stage a PATH dir containing only jq + bash. Use symlinks so we
# don't need to ship binaries.
path_dir="$fixture_root/path-without-license-checker"
mkdir -p "$path_dir"
ln -sf "$jq_bin"   "$path_dir/jq"
ln -sf "$bash_bin" "$path_dir/bash"
# Also link in any coreutils the driver might shell out to. Keep the
# list minimal so a stray license-checker on the real PATH cannot
# leak in.
for tool in cat sed dirname basename mktemp rm sort awk grep cut head tr; do
  src="$(command -v "$tool" 2>/dev/null || true)"
  if [ -n "$src" ]; then
    ln -sf "$src" "$path_dir/$tool"
  fi
done

exit4=0
out4="$(PATH="$path_dir" "$DRIVER" "$config3_json" "$fixture_root/out4.md" 2>&1 >/dev/null)" || exit4=$?
if [ "$exit4" -eq 0 ]; then
  echo "fail scenario 4: driver ran with no license-checker on PATH" >&2
  exit 1
fi
if ! printf '%s' "$out4" | grep -q "license-checker"; then
  echo "fail scenario 4: stderr lacks 'license-checker' (got: $out4)" >&2
  exit 1
fi
echo "ok scenario 4: missing license-checker rejected (exit $exit4)"

echo
echo "ATTRIB-012 Node-driver preflight tests passed: 4/4 scenarios green."
