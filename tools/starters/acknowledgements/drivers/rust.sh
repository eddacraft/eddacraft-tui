#!/usr/bin/env bash
# Rust ecosystem driver. Invoked by generate-acknowledgements.sh with
# two arguments: the block's resolved JSON config and a path where
# rendered markdown should be written.
#
# Block config schema (Rust):
#   {
#     "name": "rust",
#     "ecosystem": "rust",
#     "manifest_path": "absolute path to Cargo.toml",
#     "template_path": "absolute path to about.hbs",
#     "config_path":   "absolute path to about.toml"
#   }
#
# Driver-author contract (kept here for reviewer convenience):
#   1. Preflight  — verify required tool + state; actionable error on stderr; non-zero exit
#   2. Render     — deterministic markdown sorted/structured by the tool's own template
#   3. Strict     — reject disallowed / missing licences before render (ATTRIB-007)
#   4. No side effects on the splice target — write only to the
#      <output-temp-path> argument

set -euo pipefail

if [ $# -ne 2 ]; then
  echo "drivers/rust.sh: expected 2 arguments (block-config-json, output-temp-path), got $#" >&2
  exit 2
fi

config_json="$1"
output_path="$2"

# jq-extracted block keys. The dispatcher has already validated that
# `name` and `ecosystem` are present and non-empty; the rust driver
# requires its own ecosystem-specific trio.
manifest_path="$(printf '%s' "$config_json" | jq -er '.manifest_path // empty')" || {
  echo "drivers/rust.sh: block is missing required key 'manifest_path'" >&2
  exit 1
}
template_path="$(printf '%s' "$config_json" | jq -er '.template_path // empty')" || {
  echo "drivers/rust.sh: block is missing required key 'template_path'" >&2
  exit 1
}
config_path_about="$(printf '%s' "$config_json" | jq -er '.config_path // empty')" || {
  echo "drivers/rust.sh: block is missing required key 'config_path'" >&2
  exit 1
}

# ── Preflight ────────────────────────────────────────────────────────
# Driver-author contract rule 1: actionable error if any required
# tool is missing. The dispatcher also preflights `jq`, but each
# driver checks its own deps so direct invocation (tests, scripts)
# gives the same actionable error rather than a `command not found`.
if ! command -v jq >/dev/null 2>&1; then
  echo "drivers/rust.sh: jq not installed (required to parse the block-config-json argument)" >&2
  exit 1
fi
if ! command -v cargo-about >/dev/null 2>&1; then
  echo "drivers/rust.sh: cargo-about not installed. Install the version pinned by your project (see CI), e.g.:" >&2
  echo "  cargo install cargo-about --locked --version <CARGO_ABOUT_VERSION>" >&2
  exit 1
fi

for f in "$manifest_path" "$template_path" "$config_path_about"; do
  if [ ! -f "$f" ]; then
    echo "drivers/rust.sh: required file does not exist: $f" >&2
    exit 1
  fi
done

# ── Render ───────────────────────────────────────────────────────────
# Run cargo-about from the directory containing about.toml so it picks
# up the config without an explicit flag (cargo-about looks beside the
# cwd by default for `about.toml`).
#
# ATTRIB-007 strict-licence enforcement: `--fail` makes cargo-about
# exit non-zero when a workspace crate is missing the `license` (or
# `license-file`) field. Without it cargo-about emits a WARN and
# exits 0, and the crate silently drops out of the generated
# attribution. `--fail` puts the diagnostic at the canonical layer.
about_dir="$(cd "$(dirname "$config_path_about")" && pwd)"
(
  cd "$about_dir"
  cargo about generate "$template_path" \
    --manifest-path "$manifest_path" \
    --fail \
    -o "$output_path"
)

if [ ! -s "$output_path" ]; then
  echo "drivers/rust.sh: cargo-about produced an empty file; refusing to let the dispatcher splice an empty block" >&2
  exit 1
fi
