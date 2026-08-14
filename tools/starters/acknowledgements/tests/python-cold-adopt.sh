#!/usr/bin/env bash
# Cold-adopt as a Python-only consumer using shipped templates.
# Skips if python3 / pip-licenses cannot be provisioned.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
KIT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
GENERATOR="$KIT_DIR/generate-acknowledgements.sh"
EXPANDER="$KIT_DIR/expand-licences.sh"

if ! command -v python3 >/dev/null 2>&1; then
  echo "skip: python3 not installed" >&2
  exit 0
fi

fixture_root="$(mktemp -d)"
trap 'rm -rf "$fixture_root"' EXIT
project="$fixture_root/project"
mkdir -p "$project/pkg/fixture_dep"
cat >"$project/pkg/pyproject.toml" <<'EOF'
[project]
name = "fixture-dep"
version = "1.0.0"
license = "MIT"
[build-system]
requires = ["setuptools>=61"]
build-backend = "setuptools.build_meta"
EOF
echo 'x = 1' >"$project/pkg/fixture_dep/__init__.py"

venv="$project/.venv"
if ! python3 -m venv "$venv" >/dev/null 2>&1; then
  echo "skip: venv unavailable" >&2
  exit 0
fi
pip_licenses_spec="pip-licenses${PIP_LICENSES_VERSION:+==$PIP_LICENSES_VERSION}"
if ! "$venv/bin/python" -m pip install --quiet --disable-pip-version-check "$pip_licenses_spec" "$project/pkg" >/dev/null 2>&1; then
  echo "skip: could not install pip-licenses + fixture" >&2
  exit 0
fi

cp "$KIT_DIR/ACKNOWLEDGEMENTS.md.template" "$project/ACKNOWLEDGEMENTS.md"
cp "$KIT_DIR/licences.toml.template" "$project/licences.toml"
cp "$KIT_DIR/licences.python-allow.txt.template" "$project/licences.python-allow.txt"
cat >"$project/attribution.toml" <<EOF
[project]
target_path   = "ACKNOWLEDGEMENTS.md"
fixit_command = "generate-acknowledgements.sh"

[[blocks]]
name              = "python"
ecosystem         = "python"
venv_path         = ".venv"
python_allow_path = "licences.python-allow.txt"
EOF
sed \
  -e 's|{{PROJECT_NAME}}|Fixture|g' \
  -e 's|{{PROJECT_BINARY}}|fixture|g' \
  -e 's|{{GENERATOR_TOOL}}|pip-licenses|g' \
  -e 's|{{GENERATOR_TOOL_URL}}|https://github.com/raimon49/pip-licenses|g' \
  -e 's|{{LOCKFILE_NAME}}|requirements.txt|g' \
  -e 's|{{GENERATOR_SCRIPT_PATH}}|generate-acknowledgements.sh|g' \
  -e 's|{{BLOCK_NAME}}|python|g' \
  "$project/ACKNOWLEDGEMENTS.md" >"$project/ACKNOWLEDGEMENTS.md.tmp"
mv "$project/ACKNOWLEDGEMENTS.md.tmp" "$project/ACKNOWLEDGEMENTS.md"

if ! (cd "$project" && "$EXPANDER"); then
  echo "fail: expander refused Python-only bootstrap (no about.toml/deny.toml)" >&2
  exit 1
fi
if ! (cd "$project" && "$GENERATOR" --config attribution.toml); then
  echo "fail: documented generator command failed on a Python-only cold adopt" >&2
  exit 1
fi
if ! (cd "$project" && "$GENERATOR" --config attribution.toml --check); then
  echo "fail: --check drifted after Python cold adopt" >&2
  exit 1
fi
echo "ok: documented Python-only cold adopt generates and --check is green"
