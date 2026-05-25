#!/usr/bin/env bash
# Bundled-binaries ecosystem driver. Invoked by generate-acknowledgements.sh
# with two arguments: the block's resolved JSON config and a path where
# rendered markdown should be written.
#
# Attributes third-party binaries that are NOT a package manager's
# dependencies — OpenSSH, Mosh, FFmpeg, … — from a hand-maintained TOML
# inventory. Unlike the language drivers there is no upstream licence
# tool: the curator records each binary's licence by hand, so the
# driver's "strict" step is field validation (every entry must declare a
# name + SPDX licence) rather than a scanner gate.
#
# Block config schema (bundled-binaries):
#   {
#     "name": "binaries",
#     "ecosystem": "bundled-binaries",
#     "inventory_path": "absolute path to bundled-binaries.toml"
#   }
#
# Inventory schema (bundled-binaries.toml) — array of tables:
#   [[binary]]
#   name    = "OpenSSH"            # required
#   spdx    = "BSD-3-Clause"       # required (arbitrary SPDX expression)
#   version = "9.6p1"             # optional
#   source  = "https://…"         # optional
#   # unknown keys (e.g. maintainer notes) are tolerated and ignored
#
# Driver-author contract:
#   1. Preflight  — jq + inventory file present; actionable error otherwise
#   2. Validate   — every entry has name + spdx; name the offender otherwise
#   3. Render     — deterministic markdown sorted by binary name
#   4. No side effects on the splice target — write only to <output-temp-path>

set -euo pipefail

if [ $# -ne 2 ]; then
  echo "drivers/bundled-binaries.sh: expected 2 arguments (block-config-json, output-temp-path), got $#" >&2
  exit 2
fi

config_json="$1"
output_path="$2"

# jq first — the config parse below needs it.
if ! command -v jq >/dev/null 2>&1; then
  echo "drivers/bundled-binaries.sh: jq not installed (required to parse the block-config-json argument)" >&2
  exit 1
fi

inventory_path="$(printf '%s' "$config_json" | jq -er '.inventory_path // empty')" || {
  echo "drivers/bundled-binaries.sh: block is missing required key 'inventory_path'" >&2
  exit 1
}
# jq emits "" (exit 0) for a present-but-empty value; treat that as the
# same config error rather than letting it fall through to a blank-path
# "does not exist".
if [ -z "$inventory_path" ]; then
  echo "drivers/bundled-binaries.sh: block key 'inventory_path' is empty" >&2
  exit 1
fi

if [ ! -f "$inventory_path" ]; then
  echo "drivers/bundled-binaries.sh: inventory_path does not exist: $inventory_path" >&2
  echo "  copy tools/starters/acknowledgements/bundled-binaries.toml.example to your project" >&2
  echo "  and list each bundled third-party binary, or omit the block if you ship none." >&2
  exit 1
fi

# ── Parse + validate the inventory ───────────────────────────────────
# Emit one tab-separated record per [[binary]] entry: name, version,
# spdx, source. Validation (missing name/spdx) fails in awk naming the
# offender. Unknown keys are ignored. Minimal TOML — single-line basic
# string values only, matching the rest of the kit's parsers.
records="$(mktemp)"
parse_err="$(mktemp)"
trap 'rm -f "$records" "$parse_err"' EXIT

if ! awk '
  function strip(v) {
    sub(/^[^=]*=[[:space:]]*/, "", v)        # drop "key ="
    if (v ~ /^"/) {
      # Basic string: take the content between the opening quote and the
      # next quote, discarding any trailing inline `# comment`. (Single-
      # line basic strings only; escaped quotes are not expected in this
      # schema — names/versions/SPDX/URLs.)
      sub(/^"/, "", v)
      sub(/".*$/, "", v)
    } else {
      sub(/[[:space:]]*#.*$/, "", v)         # bare value: drop inline comment
      sub(/[[:space:]]+$/, "", v)            # and trailing whitespace
    }
    gsub(/\\"/, "\"", v)
    return v
  }
  function flush() {
    if (started) {
      if (name == "") {
        printf "drivers/bundled-binaries.sh: [[binary]] entry near line %d is missing required field '\''name'\''\n", start > "/dev/stderr"
        rc = 1
      } else if (spdx == "") {
        printf "drivers/bundled-binaries.sh: binary '\''%s'\'' (near line %d) is missing required field '\''spdx'\''\n", name, start > "/dev/stderr"
        rc = 1
      } else {
        printf "%s\t%s\t%s\t%s\n", name, version, spdx, source
      }
    }
    name = ""; version = ""; spdx = ""; source = ""; started = 0
  }
  BEGIN { rc = 0; started = 0 }
  /^[[:space:]]*#/ { next }
  /^[[:space:]]*$/ { next }
  /^[[:space:]]*\[\[binary\]\][[:space:]]*$/ { flush(); started = 1; start = NR; next }
  /^[[:space:]]*\[/ { flush(); next }   # any other table closes the entry
  started && /^[[:space:]]*name[[:space:]]*=/    { name = strip($0); next }
  started && /^[[:space:]]*version[[:space:]]*=/ { version = strip($0); next }
  started && /^[[:space:]]*spdx[[:space:]]*=/    { spdx = strip($0); next }
  started && /^[[:space:]]*source[[:space:]]*=/  { source = strip($0); next }
  END { flush(); exit rc }
' "$inventory_path" >"$records" 2>"$parse_err"; then
  cat "$parse_err" >&2
  echo "  fix the offending entry in $inventory_path." >&2
  exit 1
fi

if [ ! -s "$records" ]; then
  echo "drivers/bundled-binaries.sh: no [[binary]] entries found in $inventory_path." >&2
  echo "  add at least one [[binary]] entry, or omit the block from attribution.toml" >&2
  echo "  if this project ships no third-party binaries." >&2
  exit 1
fi

# ── Render — sorted by name for deterministic output ─────────────────
# Escape any literal `|` in a cell so a curated value can't break the
# markdown table. (The awk parser already guarantees single-line values,
# so no newline normalisation is needed.)
md_cell() { local s="$1"; printf '%s' "${s//|/\\|}"; }
{
  echo "| Binary | Version | License | Source |"
  echo "|---|---|---|---|"
  LC_ALL=C sort "$records" | while IFS=$'\t' read -r name version spdx source; do
    printf '| %s | %s | %s | %s |\n' \
      "$(md_cell "$name")" "$(md_cell "${version:-—}")" \
      "$(md_cell "$spdx")" "$(md_cell "${source:-—}")"
  done
} >"$output_path"

if [ ! -s "$output_path" ]; then
  echo "drivers/bundled-binaries.sh: render produced an empty file; refusing to let the dispatcher splice an empty block." >&2
  exit 1
fi
