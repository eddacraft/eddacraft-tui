#!/usr/bin/env bash
# Node-driver strict-licence enforcement test.
#
# Same fixture shape as node-driver-render.sh, except a third local
# package declares `"license": "GPL-3.0"` and the allow-list omits
# GPL. The dispatcher MUST exit non-zero with an actionable error
# naming the offending package — silently dropping it (or rendering a
# block that excludes it without erroring) would defeat the whole
# point of the strict gate.
#
# Skips cleanly (exit 0 with a stderr note) if `npm` is not on PATH.
#
# Local invocation:
#   tools/starters/acknowledgements/tests/node-driver-strict.sh

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
mkdir -p "$project/packages/fake-a" "$project/packages/fake-b" \
         "$project/packages/fake-gpl"

# ── Local-file packages — two compliant, one disallowed ──────────────
cat >"$project/packages/fake-a/package.json" <<'EOF'
{ "name": "fake-a", "version": "1.0.0", "license": "MIT" }
EOF

cat >"$project/packages/fake-b/package.json" <<'EOF'
{ "name": "fake-b", "version": "2.0.0", "license": "Apache-2.0" }
EOF

cat >"$project/packages/fake-gpl/package.json" <<'EOF'
{ "name": "fake-gpl", "version": "3.0.0", "license": "GPL-3.0" }
EOF

cat >"$project/package.json" <<'EOF'
{
  "name": "attrib-node-strict-fixture",
  "version": "0.0.0",
  "private": true,
  "license": "UNLICENSED",
  "dependencies": {
    "fake-a":   "file:./packages/fake-a",
    "fake-b":   "file:./packages/fake-b",
    "fake-gpl": "file:./packages/fake-gpl"
  },
  "devDependencies": {
    "license-checker": "25.0.1"
  }
}
EOF

# Allow-list deliberately excludes GPL-3.0 — fake-gpl must trip the gate.
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

<!-- BEGIN AUTO-GENERATED node -->
<!-- END AUTO-GENERATED node -->
EOF

if ! ( cd "$project" && npm install --no-audit --no-fund --prefer-offline >/dev/null 2>&1 ); then
  echo "skip: npm install failed (likely no network for license-checker fetch)" >&2
  exit 0
fi

export PATH="$project/node_modules/.bin:$PATH"

# Snapshot the on-disk target BEFORE the run — the strict-gate failure
# must leave it untouched (per the dispatcher's atomic-write contract).
sha_before="$(sha256sum "$project/ACKNOWLEDGEMENTS.md" | cut -d' ' -f1)"

# ── Run: dispatcher must exit non-zero ───────────────────────────────
exit_code=0
out="$( ( cd "$project" && "$GENERATOR" --config attribution.toml ) 2>&1 )" || exit_code=$?

if [ "$exit_code" -eq 0 ]; then
  echo "fail scenario 1: dispatcher accepted a disallowed-licence dep" >&2
  echo "  output: $out" >&2
  exit 1
fi

# stderr must name the offending package (`fake-gpl`) so the operator
# can act on it. Naming the licence (GPL-3.0) is a bonus but the
# package identity is the load-bearing claim.
if ! printf '%s' "$out" | grep -q 'fake-gpl'; then
  echo "fail scenario 1: error did not name offending package fake-gpl" >&2
  echo "  output: $out" >&2
  exit 1
fi
if ! printf '%s' "$out" | grep -qE 'GPL|disallowed|not.*allow'; then
  echo "fail scenario 1: error did not indicate the offending licence" >&2
  echo "  output: $out" >&2
  exit 1
fi
echo "ok scenario 1: dispatcher rejected disallowed-licence dep with actionable error (exit $exit_code)"

# Target must be byte-identical to its pre-run state.
sha_after="$(sha256sum "$project/ACKNOWLEDGEMENTS.md" | cut -d' ' -f1)"
if [ "$sha_before" != "$sha_after" ]; then
  echo "fail scenario 2: strict-gate failure clobbered the on-disk target" >&2
  exit 1
fi
echo "ok scenario 2: on-disk target left byte-identical after strict-gate failure"

echo
echo "Node-driver strict tests passed: 2/2 scenarios green."
