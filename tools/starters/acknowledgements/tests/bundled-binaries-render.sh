#!/usr/bin/env bash
# ATTRIB-004 bundled-binaries driver round-trip test.
#
# The bundled-binaries driver attributes third-party binaries that are
# NOT package-manager dependencies (OpenSSH, Mosh, FFmpeg, …) from a
# hand-maintained `bundled-binaries.toml` inventory. It needs no external
# tool — pure bash/awk over the inventory — so this test always runs.
#
# Drives the full generate-acknowledgements.sh dispatcher with a
# `[[blocks]]` entry of `ecosystem = "bundled-binaries"`. Asserts:
#
#   1. Dispatcher exits 0 and splices a non-empty `binaries` block.
#   2. The block lists every inventory entry, sorted by name, with
#      version + licence + source.
#   3. Output is idempotent (--check exits 0).
#   4. Hand-curated content outside the markers survives.
#
# Local invocation:
#   tools/starters/acknowledgements/tests/bundled-binaries-render.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
GENERATOR="$SCRIPT_DIR/../generate-acknowledgements.sh"

if [ ! -x "$GENERATOR" ]; then
  echo "error: generator script not found or not executable at $GENERATOR" >&2
  exit 1
fi

fixture_root="$(mktemp -d)"
trap 'rm -rf "$fixture_root"' EXIT

# ── Inventory with two entries (deliberately out of name order to
#    prove the driver sorts) ───────────────────────────────────────────
cat >"$fixture_root/bundled-binaries.toml" <<'EOF'
# Hand-maintained inventory of bundled third-party binaries.
[[binary]]
name = "OpenSSH"
version = "9.6p1"
spdx = "BSD-3-Clause"
source = "https://www.openssh.com/"
note = "Bundled for the remote-exec transport."

[[binary]]
name = "FFmpeg"
version = "7.0"
spdx = "LGPL-2.1-or-later"
source = "https://ffmpeg.org/"

# Mosh: exercises an inline `# comment` after a value (must not corrupt
# the parsed value) and a literal `|` in a cell (must be escaped so it
# can't break the markdown table).
[[binary]]
name = "Mosh"
version = "1.4.0"  # latest stable as of bundling
spdx = "GPL-3.0-or-later"
source = "https://mosh.org/ | mirror"
EOF

cat >"$fixture_root/attribution.toml" <<EOF
[project]
target_path   = "ACKNOWLEDGEMENTS.md"
fixit_command = "regenerate via the kit"

[[blocks]]
name           = "binaries"
ecosystem      = "bundled-binaries"
inventory_path = "bundled-binaries.toml"
EOF

cat >"$fixture_root/ACKNOWLEDGEMENTS.md" <<'EOF'
# Acknowledgements

Hand-curated preface — must survive regeneration.

<!-- BEGIN AUTO-GENERATED binaries -->
<!-- END AUTO-GENERATED binaries -->

Hand-curated postscript — must survive regeneration.
EOF

# ── Run 1: write ──────────────────────────────────────────────────────
( cd "$fixture_root" && "$GENERATOR" --config attribution.toml ) || {
  echo "fail: generator exited non-zero on first invocation" >&2
  exit 1
}

block_body="$(awk '/BEGIN AUTO-GENERATED binaries/,/END AUTO-GENERATED binaries/' \
              "$fixture_root/ACKNOWLEDGEMENTS.md")"

if ! printf '%s' "$block_body" | grep -qE 'OpenSSH.*9\.6p1.*BSD-3-Clause'; then
  echo "fail: block missing the OpenSSH row" >&2
  echo "body: $block_body" >&2
  exit 1
fi
if ! printf '%s' "$block_body" | grep -qE 'FFmpeg.*7\.0.*LGPL-2\.1-or-later'; then
  echo "fail: block missing the FFmpeg row" >&2
  echo "body: $block_body" >&2
  exit 1
fi
# Source column carried through.
if ! printf '%s' "$block_body" | grep -q 'https://www.openssh.com/'; then
  echo "fail: block missing the OpenSSH source URL" >&2
  echo "body: $block_body" >&2
  exit 1
fi

# Determinism: sorted by name ascending → FFmpeg before OpenSSH.
ff_line=$(printf '%s' "$block_body" | grep -nE 'FFmpeg' | head -1 | cut -d: -f1)
os_line=$(printf '%s' "$block_body" | grep -nE 'OpenSSH' | head -1 | cut -d: -f1)
if [ -z "$ff_line" ] || [ -z "$os_line" ] || [ "$ff_line" -ge "$os_line" ]; then
  echo "fail: binaries not sorted ascending by name (FFmpeg=$ff_line OpenSSH=$os_line)" >&2
  exit 1
fi

# Inline comment must not corrupt the value: Mosh's version is exactly
# "1.4.0", not "1.4.0  # latest…".
if ! printf '%s' "$block_body" | grep -qE 'Mosh.*\| 1\.4\.0 \|.*GPL-3\.0-or-later'; then
  echo "fail: Mosh row mis-parsed (inline comment leaked into the value?)" >&2
  echo "body: $block_body" >&2
  exit 1
fi
# A literal `|` in a cell must be escaped so it can't break the table.
if ! printf '%s' "$block_body" | grep -qF 'https://mosh.org/ \| mirror'; then
  echo "fail: literal pipe in the source cell was not escaped" >&2
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
echo "ok scenario 1: round-trip render listed both binaries sorted, preserved hand-curated content"

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
echo "ATTRIB-004 bundled-binaries render tests passed: 3/3 scenarios green."
