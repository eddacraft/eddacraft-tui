#!/usr/bin/env bash
# Python driver accepts classifier names when licences.toml is SPDX-only.
#
# A fixture package reports "Apache Software License" via Trove
# classifiers (no SPDX License-Expression). The allow-list is only
# Apache-2.0. The strict gate must pass.
#
# Local invocation:
#   tools/starters/acknowledgements/tests/python-driver-aliases.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
GENERATOR="$SCRIPT_DIR/../generate-acknowledgements.sh"

if ! command -v python3 >/dev/null 2>&1; then
  echo "skip: python3 not installed; CI provisions Python before running this test" >&2
  exit 0
fi

fixture_root="$(mktemp -d)"
trap 'rm -rf "$fixture_root"' EXIT

pkg="$fixture_root/fixture-apache"
mkdir -p "$pkg/fixture_apache"
cat >"$pkg/pyproject.toml" <<'EOF'
[project]
name = "fixture-apache"
version = "1.0.0"
classifiers = ["License :: OSI Approved :: Apache Software License"]

[build-system]
requires = ["setuptools>=61"]
build-backend = "setuptools.build_meta"
EOF
echo 'x = 1' >"$pkg/fixture_apache/__init__.py"

venv="$fixture_root/venv"
if ! python3 -m venv "$venv" >/dev/null 2>&1 || [ ! -x "$venv/bin/python" ]; then
  echo "skip: python3 -m venv unavailable; CI uses a seeded Python" >&2
  exit 0
fi
pip_licenses_spec="pip-licenses${PIP_LICENSES_VERSION:+==$PIP_LICENSES_VERSION}"
if ! "$venv/bin/python" -m pip install --quiet --disable-pip-version-check "$pip_licenses_spec" "$pkg" >/dev/null 2>&1; then
  echo "skip: could not install pip-licenses + fixture package" >&2
  exit 0
fi
if [ ! -x "$venv/bin/pip-licenses" ]; then
  echo "skip: pip-licenses missing after install" >&2
  exit 0
fi
reported="$("$venv/bin/pip-licenses" --format markdown --order name 2>/dev/null || true)"
if ! printf '%s' "$reported" | grep -qi 'fixture-apache'; then
  echo "skip: pip-licenses did not list fixture-apache" >&2
  exit 0
fi

cat >"$fixture_root/licences.python-allow.txt" <<'EOF'
# BEGIN AUTO-GENERATED FROM licences.toml — python-allow
Apache-2.0
# END AUTO-GENERATED FROM licences.toml — python-allow
EOF

cat >"$fixture_root/attribution.toml" <<EOF
[project]
target_path   = "ACKNOWLEDGEMENTS.md"
fixit_command = "regenerate"

[[blocks]]
name              = "python"
ecosystem         = "python"
venv_path         = "$venv"
python_allow_path = "licences.python-allow.txt"
EOF

cat >"$fixture_root/ACKNOWLEDGEMENTS.md" <<'EOF'
# Acknowledgements

<!-- BEGIN AUTO-GENERATED python -->
<!-- END AUTO-GENERATED python -->
EOF

if ! (cd "$fixture_root" && "$GENERATOR" --config attribution.toml); then
  echo "fail: SPDX-only Apache-2.0 allow-list rejected a classifier-named Apache package" >&2
  echo "  pip-licenses reported:" >&2
  printf '%s\n' "$reported" | sed 's/^/    /' >&2
  exit 1
fi

block="$(awk '/BEGIN AUTO-GENERATED python/,/END AUTO-GENERATED python/' \
         "$fixture_root/ACKNOWLEDGEMENTS.md")"
if ! printf '%s' "$block" | grep -q 'fixture-apache'; then
  echo "fail: generated block missing fixture-apache (body: $block)" >&2
  exit 1
fi

echo "ok: classifier 'Apache Software License' accepted via SPDX Apache-2.0 alias"
