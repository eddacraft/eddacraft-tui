#!/usr/bin/env bash
# Regenerate the auto-generated attribution block inside a target markdown
# file (default: ACKNOWLEDGEMENTS.md).
#
# This is a parameterised, portable generator: every project-specific value
# (manifest path, template paths, marker output, fix-it command) lives in
# the consumer repo's `attribution.toml`. The script itself carries no
# hard-coded paths.
#
# Wraps `cargo about generate` against a single Cargo manifest and splices
# the result between BEGIN/END marker comments inside the target file.
# Hand-edited content above and below the marker is preserved verbatim.
#
# Usage:
#   generate-acknowledgements.sh                     # write target file in place
#   generate-acknowledgements.sh --check             # verify without writing; exit 1 on drift
#   generate-acknowledgements.sh --output <path>     # write to <path> instead of in place
#   generate-acknowledgements.sh --config <path>     # explicit attribution.toml location
#
# `--check` and `--output` are mutually exclusive.
#
# By default the script discovers `attribution.toml` by walking from the
# caller's CWD upward; `--config` overrides discovery.
#
# Exit codes:
#   0  success / no drift
#   1  drift detected, missing markers, empty output, missing tool, or bad config
#   2  CLI argument error

set -euo pipefail

mode="write"
target_override=""
config_override=""

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
      case "$2" in
        /*) target_override="$2" ;;
        *)  target_override="$PWD/$2" ;;
      esac
      shift 2
      ;;
    --config)
      if [ -z "${2:-}" ]; then
        echo "error: --config requires a path argument" >&2
        exit 2
      fi
      case "$2" in
        /*) config_override="$2" ;;
        *)  config_override="$PWD/$2" ;;
      esac
      shift 2
      ;;
    -h|--help)
      sed -n '2,30p' "$0"
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      exit 2
      ;;
  esac
done

if [ "$mode" = "check" ] && [ -n "$target_override" ]; then
  echo "error: --check and --output are mutually exclusive" >&2
  exit 2
fi

# --- Locate attribution.toml --------------------------------------------------
#
# Discovery walks upward from CWD looking for `attribution.toml`. The file's
# parent directory becomes the project root all relative paths are resolved
# against.

if [ -n "$config_override" ]; then
  if [ ! -f "$config_override" ]; then
    echo "error: --config path does not exist: $config_override" >&2
    exit 1
  fi
  config_path="$config_override"
else
  search="$PWD"
  config_path=""
  while [ "$search" != "/" ]; do
    if [ -f "$search/attribution.toml" ]; then
      config_path="$search/attribution.toml"
      break
    fi
    search="$(dirname "$search")"
  done
  if [ -z "$config_path" ]; then
    echo "error: attribution.toml not found in CWD or any parent directory" >&2
    echo "  copy tools/starters/acknowledgements/attribution.toml.example to your repo root and edit." >&2
    exit 1
  fi
fi

project_root="$(cd "$(dirname "$config_path")" && pwd)"

# --- Read attribution.toml ---------------------------------------------------
#
# Minimal TOML reader: pulls top-level string keys from the [project] and
# [rust] tables. Comments and blank lines are skipped; values may be
# quoted with single or double quotes. The schema is intentionally narrow
# so `grep`+`sed` parsing is safe.

read_toml_value() {
  # Args: <table> <key>; emits the unquoted value or empty.
  local table="$1"
  local key="$2"
  awk -v table="$table" -v key="$key" '
    BEGIN { in_table = 0 }
    /^[[:space:]]*#/ { next }
    /^[[:space:]]*$/ { next }
    /^[[:space:]]*\[/ {
      gsub(/[[:space:]\[\]]/, "", $0)
      in_table = ($0 == table)
      next
    }
    in_table {
      line = $0
      sub(/[[:space:]]*#.*$/, "", line)
      n = index(line, "=")
      if (n == 0) next
      k = substr(line, 1, n - 1)
      v = substr(line, n + 1)
      gsub(/^[[:space:]]+|[[:space:]]+$/, "", k)
      gsub(/^[[:space:]]+|[[:space:]]+$/, "", v)
      if (k == key) {
        gsub(/^["'\'']|["'\'']$/, "", v)
        print v
        exit
      }
    }
  ' "$config_path"
}

require_value() {
  local table="$1"
  local key="$2"
  local val
  val="$(read_toml_value "$table" "$key")"
  if [ -z "$val" ]; then
    echo "error: attribution.toml is missing required key [$table].$key" >&2
    exit 1
  fi
  printf '%s' "$val"
}

resolve_path() {
  # Resolve a config-relative path against $project_root, leaving absolute
  # paths untouched.
  case "$1" in
    /*) printf '%s' "$1" ;;
    *)  printf '%s/%s' "$project_root" "$1" ;;
  esac
}

manifest_path="$(resolve_path "$(require_value rust manifest_path)")"
template_path="$(resolve_path "$(require_value rust template_path)")"
config_path_about="$(resolve_path "$(require_value rust config_path)")"
target_default="$(resolve_path "$(require_value project target_path)")"
fixit_command="$(require_value project fixit_command)"
marker_begin="$(read_toml_value project marker_begin)"
marker_end="$(read_toml_value project marker_end)"
marker_begin="${marker_begin:-<!-- BEGIN AUTO-GENERATED -->}"
marker_end="${marker_end:-<!-- END AUTO-GENERATED -->}"

# `splice_input` is the file with the BEGIN/END markers we splice into.
# It must already exist (the kit's bootstrap produces it from
# ACKNOWLEDGEMENTS.md.template). `output_path` is where the spliced
# result is written — when --output is not given, output_path == splice_input
# (overwrite-in-place). When --output IS given, output_path differs from
# splice_input and need not pre-exist.
splice_input="$target_default"
if [ -z "$target_override" ]; then
  output_path="$target_default"
else
  output_path="$target_override"
fi

# --- Preflight ----------------------------------------------------------------

if ! command -v cargo-about >/dev/null 2>&1; then
  echo "cargo-about not installed. Install the version pinned by your project (see CI), e.g.:" >&2
  echo "  cargo install cargo-about --locked --version <CARGO_ABOUT_VERSION>" >&2
  exit 1
fi

for f in "$manifest_path" "$template_path" "$config_path_about" "$splice_input"; do
  if [ ! -f "$f" ]; then
    echo "error: required file does not exist: $f" >&2
    exit 1
  fi
done

# Marker-count gate. Bare `grep -c` greps the entire literal string, so we
# can compare against exactly 1 BEGIN and 1 END marker without having to
# escape regex metacharacters in the configured marker.
begin_count=$(grep -cF "$marker_begin" "$splice_input" || true)
end_count=$(grep -cF "$marker_end" "$splice_input" || true)
if [ "$begin_count" != "1" ] || [ "$end_count" != "1" ]; then
  echo "error: $splice_input must contain exactly one BEGIN and one END marker." >&2
  echo "  '$marker_begin' count: $begin_count (expected 1)" >&2
  echo "  '$marker_end' count: $end_count (expected 1)" >&2
  exit 1
fi

# --- Generate ----------------------------------------------------------------

tmp_generated=""
tmp_output=""
trap 'rm -f "${tmp_generated:-}" "${tmp_output:-}"' EXIT
tmp_generated="$(mktemp)"
# Create the splice-output temp file in the same directory as the final
# output_path so the closing `mv` is a same-filesystem rename (atomic).
# `mktemp` would otherwise default to $TMPDIR (often /tmp), which can be
# on a different filesystem and silently degrade `mv` to copy+delete —
# breaking the atomic-write guarantee documented in the kit README.
output_dir="$(cd "$(dirname "$output_path")" && pwd)"
tmp_output="$(mktemp "$output_dir/.generate-acknowledgements.tmp.XXXXXX")"

# Run cargo-about from the directory containing about.toml so it picks up
# the config without an explicit flag (cargo-about looks beside the cwd by
# default for `about.toml`).
about_dir="$(cd "$(dirname "$config_path_about")" && pwd)"
(
  cd "$about_dir"
  cargo about generate "$template_path" \
    --manifest-path "$manifest_path" \
    -o "$tmp_generated"
)

if [ ! -s "$tmp_generated" ]; then
  echo "error: cargo-about produced an empty file; refusing to clobber $output_path" >&2
  exit 1
fi

awk -v gen="$tmp_generated" -v begin="$marker_begin" -v end="$marker_end" '
  BEGIN { in_block = 0 }
  index($0, begin) {
    print
    while ((getline line < gen) > 0) print line
    in_block = 1
    next
  }
  index($0, end) {
    in_block = 0
    print
    next
  }
  !in_block { print }
' "$splice_input" > "$tmp_output"

if [ "$mode" = "check" ]; then
  if ! diff -u "$splice_input" "$tmp_output"; then
    echo "" >&2
    echo "$splice_input is out of date." >&2
    echo "Run: $fixit_command" >&2
    exit 1
  fi
else
  mv "$tmp_output" "$output_path"
  # Disable trap removal of $tmp_output since it's been moved.
  trap 'rm -f "${tmp_generated:-}"' EXIT
  echo "Updated $output_path"
fi
