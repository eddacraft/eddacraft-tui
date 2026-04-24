#!/usr/bin/env bash
# Regenerate the auto-generated attribution block inside ACKNOWLEDGEMENTS.md.
#
# Runs `cargo about generate` scoped to the shipping `anvil` binary
# (crates/anvil-cli/Cargo.toml) so dev-only dependencies do NOT appear in
# the third-party licence attribution. The generated block is spliced in
# between the BEGIN/END AUTO-GENERATED marker comments; hand-edited
# content above the marker is preserved.
#
# Usage:
#   tools/generate-acknowledgements.sh           # overwrite ACKNOWLEDGEMENTS.md in place
#   tools/generate-acknowledgements.sh --check   # verify without writing; exit 1 on drift
#   tools/generate-acknowledgements.sh --output <path>   # write to <path> instead of in place

set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root"

target="ACKNOWLEDGEMENTS.md"
mode="write"

while [ $# -gt 0 ]; do
  case "$1" in
    --check)
      mode="check"
      shift
      ;;
    --output)
      target="$2"
      shift 2
      ;;
    *)
      echo "unknown argument: $1" >&2
      exit 2
      ;;
  esac
done

if ! command -v cargo-about >/dev/null 2>&1; then
  echo "cargo-about not installed. Install the version pinned in .github/workflows/rust.yml:" >&2
  echo "  cargo install cargo-about --locked --version <CARGO_ABOUT_VERSION>" >&2
  exit 1
fi

tmp_generated="$(mktemp)"
tmp_output="$(mktemp)"
trap 'rm -f "$tmp_generated" "$tmp_output"' EXIT

# Scoped to crates/anvil-cli so cargo-about walks only the anvil binary's
# runtime dependency graph. Dev-deps (insta, criterion, wiremock, ...) are
# not linked into the shipped binary and are excluded from attribution.
cargo about generate about.hbs \
  --manifest-path crates/anvil-cli/Cargo.toml \
  -o "$tmp_generated"

# Splice the generated block between the markers. Everything before
# BEGIN AUTO-GENERATED and after END AUTO-GENERATED is preserved verbatim.
awk -v gen="$tmp_generated" '
  BEGIN { in_block = 0 }
  /<!-- BEGIN AUTO-GENERATED -->/ {
    print
    while ((getline line < gen) > 0) print line
    in_block = 1
    next
  }
  /<!-- END AUTO-GENERATED -->/ {
    in_block = 0
    print
    next
  }
  !in_block { print }
' ACKNOWLEDGEMENTS.md > "$tmp_output"

if [ "$mode" = "check" ]; then
  if ! diff -u ACKNOWLEDGEMENTS.md "$tmp_output"; then
    echo "" >&2
    echo "ACKNOWLEDGEMENTS.md is out of date." >&2
    echo "Run: pnpm run licenses:generate" >&2
    exit 1
  fi
else
  mv "$tmp_output" "$target"
  echo "Updated $target"
fi
