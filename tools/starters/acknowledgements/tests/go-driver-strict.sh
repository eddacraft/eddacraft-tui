#!/usr/bin/env bash
# Go-driver strict-licence enforcement test.
#
# Same network-free fixture shape as go-driver-render.sh (one local
# third-party module under an MIT LICENSE, pulled via `replace`), except
# the allow-list deliberately omits MIT. The dispatcher MUST exit
# non-zero with an actionable error naming the offending library before
# rendering, and MUST leave the on-disk target byte-identical (the
# dispatcher's atomic-write contract).
#
# Skips cleanly (exit 0) if `go` or `go-licenses` is not on PATH.
#
# Local invocation:
#   tools/starters/acknowledgements/tests/go-driver-strict.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
GENERATOR="$SCRIPT_DIR/../generate-acknowledgements.sh"

if ! command -v go >/dev/null 2>&1; then
  echo "skip: go not installed; CI installs Go before running this test" >&2
  exit 0
fi
if ! command -v go-licenses >/dev/null 2>&1; then
  echo "skip: go-licenses not installed; CI installs it before running this test" >&2
  exit 0
fi
if [ ! -x "$GENERATOR" ]; then
  echo "error: generator script not found or not executable at $GENERATOR" >&2
  exit 1
fi

fixture_root="$(mktemp -d)"
trap 'rm -rf "$fixture_root"' EXIT

project="$fixture_root/project"
mkdir -p "$project/thirdparty" "$project/cmd/tool"

cat >"$project/thirdparty/go.mod" <<'EOF'
module example.com/thirdparty

go 1.21
EOF
cat >"$project/thirdparty/lib.go" <<'EOF'
package thirdparty

func Greeting() string { return "hi" }
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

cat >"$project/go.mod" <<'EOF'
module example.com/fixture

go 1.21

require example.com/thirdparty v0.0.0

replace example.com/thirdparty => ./thirdparty
EOF
cat >"$project/cmd/tool/main.go" <<'EOF'
package main

import "example.com/thirdparty"

func main() { _ = thirdparty.Greeting() }
EOF

# Allow-list deliberately excludes MIT — the third-party module must
# trip the gate.
cat >"$project/licences.go-allow.txt" <<'EOF'
# BEGIN AUTO-GENERATED FROM licences.toml — go-allow
Apache-2.0
# END AUTO-GENERATED FROM licences.toml — go-allow
EOF

cat >"$project/attribution.toml" <<EOF
[project]
target_path   = "ACKNOWLEDGEMENTS.md"
fixit_command = "regenerate via the kit"

[[blocks]]
name          = "go"
ecosystem     = "go"
module_path   = "cmd/tool"
go_allow_path = "licences.go-allow.txt"
EOF

cat >"$project/ACKNOWLEDGEMENTS.md" <<'EOF'
# Acknowledgements

<!-- BEGIN AUTO-GENERATED go -->
<!-- END AUTO-GENERATED go -->
EOF

sha_before="$(sha256sum "$project/ACKNOWLEDGEMENTS.md" | cut -d' ' -f1)"

exit_code=0
out="$( ( cd "$project" && "$GENERATOR" --config attribution.toml ) 2>&1 )" || exit_code=$?

if [ "$exit_code" -eq 0 ]; then
  echo "fail scenario 1: dispatcher accepted a disallowed-licence dependency" >&2
  echo "  output: $out" >&2
  exit 1
fi

# Error must name the offending library and indicate the licence.
if ! printf '%s' "$out" | grep -q 'example\.com/thirdparty'; then
  echo "fail scenario 1: error did not name the offending library example.com/thirdparty" >&2
  echo "  output: $out" >&2
  exit 1
fi
if ! printf '%s' "$out" | grep -qE 'MIT|disallowed|[Nn]ot allowed'; then
  echo "fail scenario 1: error did not indicate the offending licence" >&2
  echo "  output: $out" >&2
  exit 1
fi
echo "ok scenario 1: dispatcher rejected disallowed-licence dep with actionable error (exit $exit_code)"

sha_after="$(sha256sum "$project/ACKNOWLEDGEMENTS.md" | cut -d' ' -f1)"
if [ "$sha_before" != "$sha_after" ]; then
  echo "fail scenario 2: strict-gate failure clobbered the on-disk target" >&2
  exit 1
fi
echo "ok scenario 2: on-disk target left byte-identical after strict-gate failure"

echo
echo "Go-driver strict tests passed: 2/2 scenarios green."
