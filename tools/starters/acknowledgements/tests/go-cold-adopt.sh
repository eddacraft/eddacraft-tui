#!/usr/bin/env bash
# Cold-adopt as a Go-only consumer using shipped templates.
# Skips if go or go-licenses is missing.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
KIT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
GENERATOR="$KIT_DIR/generate-acknowledgements.sh"
EXPANDER="$KIT_DIR/expand-licences.sh"

if ! command -v go >/dev/null 2>&1 || ! command -v go-licenses >/dev/null 2>&1; then
  echo "skip: go or go-licenses not installed; CI installs them before this test" >&2
  exit 0
fi

fixture_root="$(mktemp -d)"
trap 'rm -rf "$fixture_root"' EXIT
project="$fixture_root/project"
mkdir -p "$project/thirdparty"
cat >"$project/go.mod" <<'EOF'
module example.com/cold

go 1.22

require example.com/thirdparty v0.0.0
replace example.com/thirdparty => ./thirdparty
EOF
cat >"$project/main.go" <<'EOF'
package main
import _ "example.com/thirdparty"
func main() {}
EOF
cat >"$project/thirdparty/go.mod" <<'EOF'
module example.com/thirdparty
go 1.22
EOF
cat >"$project/thirdparty/lib.go" <<'EOF'
package thirdparty
func X() {}
EOF
cat >"$project/thirdparty/LICENSE" <<'EOF'
MIT License

Copyright (c) 2026 Example Third Party

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
EOF

cp "$KIT_DIR/ACKNOWLEDGEMENTS.md.template" "$project/ACKNOWLEDGEMENTS.md"
cp "$KIT_DIR/licences.toml.template" "$project/licences.toml"
cp "$KIT_DIR/licences.go-allow.txt.template" "$project/licences.go-allow.txt"
cat >"$project/attribution.toml" <<'EOF'
[project]
target_path   = "ACKNOWLEDGEMENTS.md"
fixit_command = "generate-acknowledgements.sh"

[[blocks]]
name          = "go"
ecosystem     = "go"
module_path   = "."
go_allow_path = "licences.go-allow.txt"
EOF
sed \
  -e 's|{{PROJECT_NAME}}|Fixture|g' \
  -e 's|{{PROJECT_BINARY}}|fixture|g' \
  -e 's|{{GENERATOR_TOOL}}|go-licenses|g' \
  -e 's|{{GENERATOR_TOOL_URL}}|https://github.com/google/go-licenses|g' \
  -e 's|{{LOCKFILE_NAME}}|go.sum|g' \
  -e 's|{{GENERATOR_SCRIPT_PATH}}|generate-acknowledgements.sh|g' \
  -e 's|{{BLOCK_NAME}}|go|g' \
  "$project/ACKNOWLEDGEMENTS.md" >"$project/ACKNOWLEDGEMENTS.md.tmp"
mv "$project/ACKNOWLEDGEMENTS.md.tmp" "$project/ACKNOWLEDGEMENTS.md"

if ! (cd "$project" && "$EXPANDER"); then
  echo "fail: expander refused Go-only bootstrap (no about.toml/deny.toml)" >&2
  exit 1
fi
if ! (cd "$project" && "$GENERATOR" --config attribution.toml); then
  echo "fail: documented generator command failed on a Go-only cold adopt" >&2
  exit 1
fi
if ! (cd "$project" && "$GENERATOR" --config attribution.toml --check); then
  echo "fail: --check drifted after Go cold adopt" >&2
  exit 1
fi
echo "ok: documented Go-only cold adopt generates and --check is green"
