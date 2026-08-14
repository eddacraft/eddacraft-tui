#!/usr/bin/env bash
# Cold-adopt the kit as a Node-only consumer using only shipped
# templates — the documented first-copy path. Pins ATTRIB-027..-032.
#
# Skips if npm is missing or npm install cannot fetch license-checker.
#
# Local invocation:
#   tools/starters/acknowledgements/tests/node-cold-adopt.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
KIT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
GENERATOR="$KIT_DIR/generate-acknowledgements.sh"
EXPANDER="$KIT_DIR/expand-licences.sh"

if ! command -v npm >/dev/null 2>&1; then
  echo "skip: npm not installed; CI installs Node before running this test" >&2
  exit 0
fi

fixture_root="$(mktemp -d)"
trap 'rm -rf "$fixture_root"' EXIT

project="$fixture_root/project"
mkdir -p "$project/packages/fake-a"

cat >"$project/packages/fake-a/package.json" <<'EOF'
{
  "name": "fake-a",
  "version": "1.0.0",
  "license": "MIT",
  "main": "index.js"
}
EOF
echo 'module.exports = "fake-a";' >"$project/packages/fake-a/index.js"

# Published-shaped consumer: not private, so --excludePrivatePackages
# will not hide it. The driver must drop name@version itself.
cat >"$project/package.json" <<'EOF'
{
  "name": "@eddacraft/nxrust-fixture",
  "version": "0.3.0",
  "private": false,
  "license": "MIT",
  "dependencies": {
    "fake-a": "file:./packages/fake-a"
  },
  "devDependencies": {
    "license-checker": "25.0.1"
  },
  "files": ["ACKNOWLEDGEMENTS.md"]
}
EOF

# Documented copy steps (ATTRIB-027, -028), then the documented edit:
# Node-only stanza from attribution.toml.example, rust example dropped.
cp "$KIT_DIR/ACKNOWLEDGEMENTS.md.template" "$project/ACKNOWLEDGEMENTS.md"
cp "$KIT_DIR/licences.toml.template" "$project/licences.toml"
cp "$KIT_DIR/licences.node-allow.txt.template" "$project/licences.node-allow.txt"
cat >"$project/attribution.toml" <<'EOF'
[project]
target_path   = "ACKNOWLEDGEMENTS.md"
fixit_command = "tools/starters/acknowledgements/generate-acknowledgements.sh"

[[blocks]]
name            = "node"
ecosystem       = "node"
manifest_path   = "package.json"
node_allow_path = "licences.node-allow.txt"
prod_only       = true
EOF

sed -i \
  -e 's|{{PROJECT_NAME}}|Fixture|g' \
  -e 's|{{PROJECT_BINARY}}|fixture|g' \
  -e 's|{{GENERATOR_TOOL}}|license-checker|g' \
  -e 's|{{GENERATOR_TOOL_URL}}|https://github.com/davglass/license-checker|g' \
  -e 's|{{LOCKFILE_NAME}}|package-lock.json|g' \
  -e 's|{{GENERATOR_SCRIPT_PATH}}|generate-acknowledgements.sh|g' \
  -e 's|{{BLOCK_NAME}}|node|g' \
  "$project/ACKNOWLEDGEMENTS.md"

if ! (cd "$project" && "$EXPANDER"); then
  echo "fail: expander refused the documented Node-only bootstrap" >&2
  exit 1
fi

if ! (cd "$project" && npm install --no-audit --no-fund --prefer-offline >/dev/null 2>&1); then
  echo "skip: npm install failed (likely no network for license-checker fetch)" >&2
  exit 0
fi

# No PATH surgery. This is the documented generator command.
if ! (cd "$project" && "$GENERATOR" --config attribution.toml); then
  echo "fail: documented generator command failed on a Node-only cold adopt" >&2
  exit 1
fi

if [ ! -s "$project/ACKNOWLEDGEMENTS.md" ]; then
  echo "fail: ACKNOWLEDGEMENTS.md missing or empty after generate" >&2
  exit 1
fi

block_body="$(awk '/BEGIN AUTO-GENERATED node/,/END AUTO-GENERATED node/' \
              "$project/ACKNOWLEDGEMENTS.md")"
if ! printf '%s' "$block_body" | grep -qE 'fake-a.*1\.0\.0.*MIT'; then
  echo "fail: generated block missing fake-a (body: $block_body)" >&2
  exit 1
fi
if printf '%s' "$block_body" | grep -q 'nxrust-fixture'; then
  echo "fail: consumer package was attributed to itself (body: $block_body)" >&2
  exit 1
fi

if ! (cd "$project" && "$GENERATOR" --config attribution.toml --check); then
  echo "fail: --check reported drift immediately after generate" >&2
  exit 1
fi

echo "ok: documented Node-only cold adopt generates, excludes self, and --check is green"
