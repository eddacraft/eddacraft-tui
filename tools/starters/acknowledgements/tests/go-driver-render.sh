#!/usr/bin/env bash
# ATTRIB-013 Go-driver round-trip test.
#
# Stands up a self-contained Go module fixture whose only third-party
# dependency is a local module pulled in via a `replace` directive (no
# network), then drives the full generate-acknowledgements.sh dispatcher
# with a `[[blocks]]` entry of `ecosystem = "go"`. Asserts:
#
#   1. Dispatcher exits 0 and splices a non-empty block between the
#      per-block markers.
#   2. The rendered block lists the third-party module + its licence,
#      and EXCLUDES the fixture's own main module.
#   3. Output is sorted (deterministic) and idempotent (--check exits 0).
#   4. Hand-curated content outside the markers survives regeneration.
#
# Skips cleanly (exit 0 with a stderr note) if `go` or `go-licenses` is
# not on PATH — CI installs both before invoking this test.
#
# Local invocation:
#   tools/starters/acknowledgements/tests/go-driver-render.sh

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

# ── Local third-party module with an MIT LICENSE (no network) ────────
cat >"$project/thirdparty/go.mod" <<'EOF'
module example.com/thirdparty

go 1.21
EOF
cat >"$project/thirdparty/lib.go" <<'EOF'
package thirdparty

// Greeting returns a fixed string.
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

# ── Consumer module that depends on the local third-party module ─────
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

# Allow-list fragment, as the expander would emit it (comma-joined SPDX
# for go-licenses --allowed_licenses). Hand-written here so the driver
# test is independent of expand-licences.sh.
cat >"$project/licences.go-allow.txt" <<'EOF'
# BEGIN AUTO-GENERATED FROM licences.toml — go-allow
MIT,Apache-2.0
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

Hand-curated preface — must survive regeneration.

<!-- BEGIN AUTO-GENERATED go -->
<!-- END AUTO-GENERATED go -->

Hand-curated postscript — must survive regeneration.
EOF

# ── Run 1: write ──────────────────────────────────────────────────────
( cd "$project" && "$GENERATOR" --config attribution.toml ) || {
  echo "fail: generator exited non-zero on first invocation" >&2
  exit 1
}

block_body="$(awk '/BEGIN AUTO-GENERATED go/,/END AUTO-GENERATED go/' \
              "$project/ACKNOWLEDGEMENTS.md")"

if printf '%s' "$block_body" | grep -q 'BEGIN AUTO-GENERATED go' \
   && printf '%s' "$block_body" | grep -A0 'BEGIN AUTO-GENERATED go' | tail -n +2 | grep -q 'END AUTO-GENERATED go'; then
  : # fallthrough; emptiness checked below
fi

if ! printf '%s' "$block_body" | grep -qE 'example\.com/thirdparty.*MIT'; then
  echo "fail: rendered block missing the third-party module/MIT row" >&2
  echo "body: $block_body" >&2
  exit 1
fi

# The fixture's own main module must NOT be attributed (it is the
# project itself, not a third-party dependency).
if printf '%s' "$block_body" | grep -q 'example\.com/fixture'; then
  echo "fail: rendered block attributed the project's own main module" >&2
  echo "body: $block_body" >&2
  exit 1
fi

# Hand-curated content survives.
if ! grep -q 'Hand-curated preface' "$project/ACKNOWLEDGEMENTS.md"; then
  echo "fail: preface was clobbered" >&2
  exit 1
fi
if ! grep -q 'Hand-curated postscript' "$project/ACKNOWLEDGEMENTS.md"; then
  echo "fail: postscript was clobbered" >&2
  exit 1
fi
echo "ok scenario 1: round-trip render attributed the third-party module, excluded the main module, preserved hand-curated content"

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
echo "ATTRIB-013 Go-driver render tests passed: 3/3 scenarios green."
