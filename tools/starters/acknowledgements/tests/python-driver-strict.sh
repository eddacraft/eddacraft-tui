#!/usr/bin/env bash
# ATTRIB-014 Python-driver strict-licence enforcement test.
#
# Same venv shape as python-driver-render.sh (pip-licenses + a local
# fixture package licensed MIT), except the allow-list omits MIT. The
# dispatcher MUST exit non-zero with an actionable error naming the
# offending package before render, and MUST leave the on-disk target
# byte-identical (the dispatcher's atomic-write contract).
#
# Skips cleanly (exit 0) when a working pip-licenses can't be provided
# (see python-driver-render.sh for the conditions).
#
# Local invocation:
#   tools/starters/acknowledgements/tests/python-driver-strict.sh

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
if ! "$venv/bin/pip-licenses" --format markdown 2>/dev/null | grep -q 'fixture-dep'; then
  echo "skip: pip-licenses reports no packages for a known-populated venv (Python/tool incompat)" >&2
  exit 0
fi

# Allow-list deliberately excludes MIT — fixture-dep must trip the gate.
cat >"$fixture_root/licences.python-allow.txt" <<'EOF'
# BEGIN AUTO-GENERATED FROM licences.toml — python-allow
Apache-2.0;BSD-3-Clause
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

<!-- BEGIN AUTO-GENERATED python -->
<!-- END AUTO-GENERATED python -->
EOF

sha_before="$(sha256sum "$fixture_root/ACKNOWLEDGEMENTS.md" | cut -d' ' -f1)"

exit_code=0
out="$( ( cd "$fixture_root" && "$GENERATOR" --config attribution.toml ) 2>&1 )" || exit_code=$?

if [ "$exit_code" -eq 0 ]; then
  echo "fail scenario 1: dispatcher accepted a disallowed-licence dependency" >&2
  echo "  output: $out" >&2
  exit 1
fi
if ! printf '%s' "$out" | grep -q 'fixture-dep'; then
  echo "fail scenario 1: error did not name the offending package fixture-dep" >&2
  echo "  output: $out" >&2
  exit 1
fi
if ! printf '%s' "$out" | grep -qiE 'MIT|not in allow|disallowed|rejected'; then
  echo "fail scenario 1: error did not indicate the offending licence" >&2
  echo "  output: $out" >&2
  exit 1
fi
echo "ok scenario 1: dispatcher rejected disallowed-licence dep with actionable error (exit $exit_code)"

sha_after="$(sha256sum "$fixture_root/ACKNOWLEDGEMENTS.md" | cut -d' ' -f1)"
if [ "$sha_before" != "$sha_after" ]; then
  echo "fail scenario 2: strict-gate failure clobbered the on-disk target" >&2
  exit 1
fi
echo "ok scenario 2: on-disk target left byte-identical after strict-gate failure"

echo
echo "ATTRIB-014 Python-driver strict tests passed: 2/2 scenarios green."
