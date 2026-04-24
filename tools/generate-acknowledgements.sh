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
#
# `--check` and `--output` are mutually exclusive.

set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"

target=""
mode="write"

while [ $# -gt 0 ]; do
  case "$1" in
    --check)
      mode="check"
      shift
      ;;
    --output)
      if [ -z "${2:-}" ]; then
        echo "error: --output requires a path argument" >&2
        exit 2
      fi
      # Resolve before `cd` so relative paths honour the caller's CWD.
      case "$2" in
        /*) target="$2" ;;
        *)  target="$PWD/$2" ;;
      esac
      shift 2
      ;;
    *)
      echo "unknown argument: $1" >&2
      exit 2
      ;;
  esac
done

if [ "$mode" = "check" ] && [ -n "$target" ]; then
  echo "error: --check and --output are mutually exclusive" >&2
  exit 2
fi

if [ -z "$target" ]; then
  target="$repo_root/ACKNOWLEDGEMENTS.md"
fi

cd "$repo_root"

if ! command -v cargo-about >/dev/null 2>&1; then
  echo "cargo-about not installed. Install the version pinned in .github/workflows/rust.yml:" >&2
  echo "  cargo install cargo-about --locked --version <CARGO_ABOUT_VERSION>" >&2
  exit 1
fi

# Fail fast if the source file is missing the markers — otherwise the awk
# splice below would silently produce a file identical to the input, and
# `--check` would report "all good" while regeneration never actually happened.
begin_count=$(grep -c '<!-- BEGIN AUTO-GENERATED -->' ACKNOWLEDGEMENTS.md || true)
end_count=$(grep -c '<!-- END AUTO-GENERATED -->' ACKNOWLEDGEMENTS.md || true)
if [ "$begin_count" != "1" ] || [ "$end_count" != "1" ]; then
  echo "error: ACKNOWLEDGEMENTS.md must contain exactly one BEGIN and one END marker." >&2
  echo "  <!-- BEGIN AUTO-GENERATED --> count: $begin_count (expected 1)" >&2
  echo "  <!-- END AUTO-GENERATED -->   count: $end_count (expected 1)" >&2
  exit 1
fi

tmp_generated=""
tmp_output=""
trap 'rm -f "${tmp_generated:-}" "${tmp_output:-}"' EXIT
tmp_generated="$(mktemp)"
tmp_output="$(mktemp)"

# Scoped to crates/anvil-cli so cargo-about walks only the anvil binary's
# runtime dependency graph. Dev-deps (insta, criterion, wiremock, ...) are
# not linked into the shipped binary and are excluded from attribution.
cargo about generate about.hbs \
  --manifest-path crates/anvil-cli/Cargo.toml \
  -o "$tmp_generated"

# Guard against a silent empty-output accepting into the file.
if [ ! -s "$tmp_generated" ]; then
  echo "error: cargo-about produced an empty file; refusing to clobber ACKNOWLEDGEMENTS.md" >&2
  exit 1
fi

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
