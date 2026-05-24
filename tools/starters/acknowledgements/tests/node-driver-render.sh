#!/usr/bin/env bash
# ATTRIB-012 Node-driver round-trip test.
#
# Stands up a self-contained project fixture with two local-file
# packages (no network), runs `npm install` to populate node_modules
# and install `license-checker`, then drives the full
# generate-acknowledgements.sh dispatcher with a `[[blocks]]` entry
# of `ecosystem = "node"`. Asserts:
#
#   1. Dispatcher exits 0 and splices a block between the per-block
#      markers.
#   2. The rendered block lists both fixture packages, sorted by
#      `name@version` ascending, in deterministic markdown form.
#   3. Second invocation produces no diff against the on-disk target
#      (`--check` exits 0).
#
# Skips cleanly (exit 0 with a stderr note) if `npm` is not on PATH
# — CI is expected to install Node before invoking this test.
#
# Local invocation:
#   tools/starters/acknowledgements/tests/node-driver-render.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
GENERATOR="$SCRIPT_DIR/../generate-acknowledgements.sh"

if ! command -v npm >/dev/null 2>&1; then
  echo "skip: npm not installed; CI installs Node before running this test" >&2
  exit 0
fi

if [ ! -x "$GENERATOR" ]; then
  echo "error: generator script not found or not executable at $GENERATOR" >&2
  exit 1
fi

fixture_root="$(mktemp -d)"
trap 'rm -rf "$fixture_root"' EXIT

project="$fixture_root/project"
mkdir -p "$project/packages/fake-a" "$project/packages/fake-b"

# ── Local-file packages (no network) ─────────────────────────────────
cat >"$project/packages/fake-a/package.json" <<'EOF'
{
  "name": "fake-a",
  "version": "1.0.0",
  "license": "MIT",
  "main": "index.js"
}
EOF
echo 'module.exports = "fake-a";' >"$project/packages/fake-a/index.js"

cat >"$project/packages/fake-b/package.json" <<'EOF'
{
  "name": "fake-b",
  "version": "2.0.0",
  "license": "Apache-2.0",
  "main": "index.js"
}
EOF
echo 'module.exports = "fake-b";' >"$project/packages/fake-b/index.js"

# ── Consumer project that depends on fake-a, fake-b, license-checker ─
cat >"$project/package.json" <<'EOF'
{
  "name": "attrib-node-fixture",
  "version": "0.0.0",
  "private": true,
  "license": "UNLICENSED",
  "dependencies": {
    "fake-a": "file:./packages/fake-a",
    "fake-b": "file:./packages/fake-b"
  },
  "devDependencies": {
    "license-checker": "^25.0.1"
  }
}
EOF

cat >"$project/licences.node-allow.txt" <<'EOF'
MIT;Apache-2.0
EOF

cat >"$project/attribution.toml" <<EOF
[project]
target_path   = "ACKNOWLEDGEMENTS.md"
fixit_command = "regenerate via the kit"

[[blocks]]
name             = "node"
ecosystem        = "node"
manifest_path    = "package.json"
node_allow_path  = "licences.node-allow.txt"
prod_only        = true
EOF

cat >"$project/ACKNOWLEDGEMENTS.md" <<'EOF'
# Acknowledgements

Hand-curated preface — must survive regeneration.

<!-- BEGIN AUTO-GENERATED node -->
<!-- END AUTO-GENERATED node -->

Hand-curated postscript — must survive regeneration.
EOF

# ── Install dependencies (best-effort; skip if offline) ──────────────
if ! ( cd "$project" && npm install --no-audit --no-fund --prefer-offline >/dev/null 2>&1 ); then
  echo "skip: npm install failed (likely no network for license-checker fetch)" >&2
  exit 0
fi

# Surface the fixture's installed license-checker binary on PATH so
# the driver's preflight + invocation finds it without the user
# needing a global install.
export PATH="$project/node_modules/.bin:$PATH"

# ── Run 1: write ──────────────────────────────────────────────────────
( cd "$project" && "$GENERATOR" --config attribution.toml ) || {
  echo "fail: generator exited non-zero on first invocation" >&2
  exit 1
}

# Sanity: target file changed (markers no longer adjacent).
if grep -A1 'BEGIN AUTO-GENERATED node' "$project/ACKNOWLEDGEMENTS.md" \
     | grep -q 'END AUTO-GENERATED node'; then
  echo "fail: generator left the node block empty (markers still adjacent)" >&2
  exit 1
fi

# Block must mention both fixture packages, each on a line that
# also carries the package's version + licence.
block_body="$(awk '/BEGIN AUTO-GENERATED node/,/END AUTO-GENERATED node/' \
              "$project/ACKNOWLEDGEMENTS.md")"
if ! printf '%s' "$block_body" | grep -qE 'fake-a.*1\.0\.0.*MIT'; then
  echo "fail: rendered block missing fake-a/1.0.0/MIT row (body: $block_body)" >&2
  exit 1
fi
if ! printf '%s' "$block_body" | grep -qE 'fake-b.*2\.0\.0.*Apache-2\.0'; then
  echo "fail: rendered block missing fake-b/2.0.0/Apache-2.0 row (body: $block_body)" >&2
  exit 1
fi

# Determinism: fake-a row must appear before fake-b row (sorted ascending).
a_line=$(printf '%s' "$block_body" | grep -nE 'fake-a.*1\.0\.0' | head -1 | cut -d: -f1)
b_line=$(printf '%s' "$block_body" | grep -nE 'fake-b.*2\.0\.0' | head -1 | cut -d: -f1)
if [ -z "$a_line" ] || [ -z "$b_line" ] || [ "$a_line" -ge "$b_line" ]; then
  echo "fail: render not sorted ascending (a_line=$a_line b_line=$b_line)" >&2
  exit 1
fi

# Hand-curated preface + postscript must be untouched.
if ! grep -q 'Hand-curated preface' "$project/ACKNOWLEDGEMENTS.md"; then
  echo "fail: preface was clobbered" >&2
  exit 1
fi
if ! grep -q 'Hand-curated postscript' "$project/ACKNOWLEDGEMENTS.md"; then
  echo "fail: postscript was clobbered" >&2
  exit 1
fi

echo "ok scenario 1: round-trip render produced sorted two-package block + preserved hand-curated content"

# ── Run 2: idempotency ────────────────────────────────────────────────
sha_before="$(sha256sum "$project/ACKNOWLEDGEMENTS.md" | cut -d' ' -f1)"
( cd "$project" && "$GENERATOR" --config attribution.toml ) || {
  echo "fail: generator exited non-zero on second invocation" >&2
  exit 1
}
sha_after="$(sha256sum "$project/ACKNOWLEDGEMENTS.md" | cut -d' ' -f1)"
if [ "$sha_before" != "$sha_after" ]; then
  echo "fail: second invocation changed the target (not idempotent)" >&2
  exit 1
fi
echo "ok scenario 2: second invocation is byte-identical"

# ── Run 3: --check on the up-to-date target ──────────────────────────
( cd "$project" && "$GENERATOR" --check --config attribution.toml ) || {
  echo "fail: --check exited non-zero on an up-to-date target" >&2
  exit 1
}
echo "ok scenario 3: --check exits 0 on up-to-date target"

echo
echo "ATTRIB-012 Node-driver render tests passed: 3/3 scenarios green."
