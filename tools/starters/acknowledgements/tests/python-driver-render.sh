#!/usr/bin/env bash
# ATTRIB-014 Python-driver round-trip test.
#
# Builds a consumer-style virtualenv containing pip-licenses plus a
# local fixture package (no network for the package itself), then drives
# the full generate-acknowledgements.sh dispatcher with a `[[blocks]]`
# entry of `ecosystem = "python"`. Asserts:
#
#   1. Dispatcher exits 0 and splices a non-empty block.
#   2. The rendered block lists the fixture package + its licence.
#   3. Output is idempotent (--check exits 0).
#   4. Hand-curated content outside the markers survives.
#
# Skips cleanly (exit 0 with a stderr note) when the environment can't
# provide a working pip-licenses: no python3, venv/pip unavailable,
# offline (pip-licenses fetch fails), or a pip-licenses/Python
# incompatibility that yields no rows for a known-populated venv (e.g.
# bleeding-edge CPython). CI pins a stable Python via setup-python.
#
# Local invocation:
#   tools/starters/acknowledgements/tests/python-driver-render.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
GENERATOR="$SCRIPT_DIR/../generate-acknowledgements.sh"

if ! command -v python3 >/dev/null 2>&1; then
  echo "skip: python3 not installed; CI provisions Python before running this test" >&2
  exit 0
fi
if [ ! -x "$GENERATOR" ]; then
  echo "error: generator script not found or not executable at $GENERATOR" >&2
  exit 1
fi

fixture_root="$(mktemp -d)"
trap 'rm -rf "$fixture_root"' EXIT

# ── Local fixture package (no network) ───────────────────────────────
pkg="$fixture_root/fixture-dep"
mkdir -p "$pkg/fixture_dep"
cat >"$pkg/pyproject.toml" <<'EOF'
[project]
name = "fixture-dep"
version = "1.0.0"
license = "MIT"

[build-system]
requires = ["setuptools>=61"]
build-backend = "setuptools.build_meta"
EOF
echo 'def hi(): return "hi"' >"$pkg/fixture_dep/__init__.py"

# ── Build the consumer venv ──────────────────────────────────────────
venv="$fixture_root/venv"
if ! python3 -m venv "$venv" >/dev/null 2>&1 || [ ! -x "$venv/bin/python" ]; then
  echo "skip: python3 -m venv unavailable (no ensurepip?); CI uses a seeded Python" >&2
  exit 0
fi
pip_licenses_spec="pip-licenses${PIP_LICENSES_VERSION:+==$PIP_LICENSES_VERSION}"
if ! "$venv/bin/python" -m pip install --quiet --disable-pip-version-check "$pip_licenses_spec" "$pkg" >/dev/null 2>&1; then
  echo "skip: could not install pip-licenses + fixture package (likely offline)" >&2
  exit 0
fi
if [ ! -x "$venv/bin/pip-licenses" ]; then
  echo "skip: pip-licenses console script not present after install" >&2
  exit 0
fi
# Guard against a pip-licenses/Python incompatibility that enumerates no
# packages (seen on bleeding-edge CPython) — that is an environment
# limitation, not a driver defect, so skip rather than fail.
if ! "$venv/bin/pip-licenses" --format markdown 2>/dev/null | grep -q 'fixture-dep'; then
  echo "skip: pip-licenses reports no packages for a known-populated venv (Python/tool incompat)" >&2
  exit 0
fi

# ── Consumer config ──────────────────────────────────────────────────
cat >"$fixture_root/licences.python-allow.txt" <<'EOF'
# BEGIN AUTO-GENERATED FROM licences.toml — python-allow
MIT;Apache-2.0;BSD-3-Clause
# END AUTO-GENERATED FROM licences.toml — python-allow
EOF

cat >"$fixture_root/attribution.toml" <<EOF
[project]
target_path   = "ACKNOWLEDGEMENTS.md"
fixit_command = "regenerate via the kit"

[[blocks]]
name              = "python"
ecosystem         = "python"
venv_path         = "venv"
python_allow_path = "licences.python-allow.txt"
EOF

cat >"$fixture_root/ACKNOWLEDGEMENTS.md" <<'EOF'
# Acknowledgements

Hand-curated preface — must survive regeneration.

<!-- BEGIN AUTO-GENERATED python -->
<!-- END AUTO-GENERATED python -->

Hand-curated postscript — must survive regeneration.
EOF

# ── Run 1: write ──────────────────────────────────────────────────────
( cd "$fixture_root" && "$GENERATOR" --config attribution.toml ) || {
  echo "fail: generator exited non-zero on first invocation" >&2
  exit 1
}

block_body="$(awk '/BEGIN AUTO-GENERATED python/,/END AUTO-GENERATED python/' \
              "$fixture_root/ACKNOWLEDGEMENTS.md")"
if ! printf '%s' "$block_body" | grep -qE 'fixture-dep.*1\.0\.0.*MIT'; then
  echo "fail: rendered block missing fixture-dep/1.0.0/MIT row" >&2
  echo "body: $block_body" >&2
  exit 1
fi
if ! grep -q 'Hand-curated preface' "$fixture_root/ACKNOWLEDGEMENTS.md"; then
  echo "fail: preface was clobbered" >&2
  exit 1
fi
if ! grep -q 'Hand-curated postscript' "$fixture_root/ACKNOWLEDGEMENTS.md"; then
  echo "fail: postscript was clobbered" >&2
  exit 1
fi
echo "ok scenario 1: round-trip render listed the fixture package, preserved hand-curated content"

# ── Run 2: idempotency ────────────────────────────────────────────────
sha_before="$(sha256sum "$fixture_root/ACKNOWLEDGEMENTS.md" | cut -d' ' -f1)"
( cd "$fixture_root" && "$GENERATOR" --config attribution.toml ) || {
  echo "fail: generator exited non-zero on second invocation" >&2
  exit 1
}
sha_after="$(sha256sum "$fixture_root/ACKNOWLEDGEMENTS.md" | cut -d' ' -f1)"
if [ "$sha_before" != "$sha_after" ]; then
  echo "fail: second invocation changed the target (not idempotent)" >&2
  exit 1
fi
echo "ok scenario 2: second invocation is byte-identical"

# ── Run 3: --check on the up-to-date target ──────────────────────────
( cd "$fixture_root" && "$GENERATOR" --check --config attribution.toml ) || {
  echo "fail: --check exited non-zero on an up-to-date target" >&2
  exit 1
}
echo "ok scenario 3: --check exits 0 on up-to-date target"

echo
echo "ATTRIB-014 Python-driver render tests passed: 3/3 scenarios green."
