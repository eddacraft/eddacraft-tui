#!/usr/bin/env bash
# Regenerate auto-generated attribution blocks inside a target markdown
# file (default: ACKNOWLEDGEMENTS.md).
#
# This is a parameterised, portable dispatcher. Every project-specific
# value (manifests, tool selection, marker output, fix-it command)
# lives in the consumer repo's `attribution.toml`. The dispatcher
# routes each declared block to an ecosystem-specific driver under
# `drivers/<ecosystem>.sh`.
#
# Schema (canonical):
#
#   [project]
#   target_path   = "ACKNOWLEDGEMENTS.md"
#   fixit_command = "tools/starters/acknowledgements/generate-acknowledgements.sh"
#   # marker_begin / marker_end optional global overrides
#
#   [[blocks]]
#   name      = "rust"             # required (kebab-case when non-empty)
#   ecosystem = "rust"             # required; must match drivers/<ecosystem>.sh
#   # ecosystem-specific keys: manifest_path, template_path, config_path, …
#
# Back-compat shim: a consumer with the legacy flat `[rust]` table
# (no `[[blocks]]` entries) is treated as if it declared a single
# unnamed block (`name = ""`, `ecosystem = "rust"`). Markers for the
# unnamed block omit the name suffix (`<!-- BEGIN AUTO-GENERATED -->`).
# Mixing flat `[rust]` and `[[blocks]]` in one file is a hard error.
#
# Usage:
#   generate-acknowledgements.sh                     # write target file in place
#   generate-acknowledgements.sh --check             # verify without writing; exit 1 on drift
#   generate-acknowledgements.sh --output <path>     # write to <path> instead of in place
#   generate-acknowledgements.sh --config <path>     # explicit attribution.toml location
#
# `--check` and `--output` are mutually exclusive.
#
# Discovery: walks from CWD upward for `attribution.toml`. `--config`
# overrides discovery. Drivers are looked up at
# `${ATTRIB_DRIVERS_DIR:-<script-dir>/drivers}/<ecosystem>.sh`; the
# env var override is intended for tests, not production consumers.
#
# Exit codes:
#   0  success / no drift
#   1  drift detected, missing markers, empty output, missing tool, bad config, driver failure
#   2  CLI argument error

set -euo pipefail

# ── CLI parsing ──────────────────────────────────────────────────────

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
      sed -n '2,45p' "$0"
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

# ── Tool preflight ───────────────────────────────────────────────────

if ! command -v jq >/dev/null 2>&1; then
  echo "error: jq not installed (required for block-config JSON handoff to drivers)" >&2
  exit 1
fi

# ── Locate attribution.toml ──────────────────────────────────────────

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

# Resolve $0 through symlinks before taking dirname: a consumer may
# expose the dispatcher via a symlink (e.g. ~/.local/bin/), and a bare
# `dirname "$0"` would then point at the link's directory, where
# `drivers/` does not exist. Plain `readlink` (no -f) keeps this
# portable to macOS, which lacked `readlink -f` before 12.3.
script_path="$0"
link_hops=0
while [ -L "$script_path" ]; do
  link_hops=$((link_hops + 1))
  if [ "$link_hops" -gt 40 ]; then
    echo "error: too many symlink levels resolving $0 (circular symlink?)" >&2
    exit 1
  fi
  link_target="$(readlink "$script_path")"
  case "$link_target" in
    /*) script_path="$link_target" ;;
    *)  script_path="$(dirname "$script_path")/$link_target" ;;
  esac
done
script_dir="$(cd "$(dirname "$script_path")" && pwd)"
drivers_dir="${ATTRIB_DRIVERS_DIR:-$script_dir/drivers}"

# ── TOML helpers ─────────────────────────────────────────────────────
# The schema we accept is narrow on purpose: a small number of named
# scalar tables ([project], [rust]) plus an array-of-tables [[blocks]]
# carrying string-valued keys. We do not try to be a general TOML
# parser — kits adopting this script benefit from the simplicity.

# read_scalar() extracts kvs from a scalar table `[name]` and emits
# `key=value` lines on stdout. Values are unquoted (single or double
# quotes stripped). Lines with no `=` are ignored.
read_scalar() {
  local table="$1"
  awk -v table="$table" '
    BEGIN { in_table = 0 }
    /^[[:space:]]*#/ { next }
    /^[[:space:]]*$/ { next }
    /^[[:space:]]*\[\[/ {            # array-of-tables marker
      in_table = 0
      next
    }
    /^[[:space:]]*\[/ {               # scalar table marker
      header = $0
      gsub(/[[:space:]\[\]]/, "", header)
      in_table = (header == table)
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
      gsub(/^["'\'']|["'\'']$/, "", v)
      print k "=" v
    }
  ' "$config_path"
}

# count_array_entries() counts `[[name]]` occurrences (one per array entry).
count_array_entries() {
  local name="$1"
  awk -v name="$name" '
    /^[[:space:]]*\[\[/ {
      header = $0
      gsub(/[[:space:]\[\]]/, "", header)
      if (header == name) count++
    }
    END { print count + 0 }
  ' "$config_path"
}

# read_array_entry() extracts the i-th `[[name]]` entry's kvs as
# `key=value` lines. i is 0-indexed.
read_array_entry() {
  local name="$1"
  local index="$2"
  awk -v name="$name" -v target="$index" '
    BEGIN { current = -1; in_entry = 0 }
    /^[[:space:]]*#/ { next }
    /^[[:space:]]*$/ { next }
    /^[[:space:]]*\[\[/ {
      header = $0
      gsub(/[[:space:]\[\]]/, "", header)
      if (header == name) {
        current++
        in_entry = (current == target)
      } else {
        in_entry = 0
      }
      next
    }
    /^[[:space:]]*\[/ {                # scalar table closes any open array entry
      in_entry = 0
      next
    }
    in_entry {
      line = $0
      sub(/[[:space:]]*#.*$/, "", line)
      n = index(line, "=")
      if (n == 0) next
      k = substr(line, 1, n - 1)
      v = substr(line, n + 1)
      gsub(/^[[:space:]]+|[[:space:]]+$/, "", k)
      gsub(/^[[:space:]]+|[[:space:]]+$/, "", v)
      gsub(/^["'\'']|["'\'']$/, "", v)
      print k "=" v
    }
  ' "$config_path"
}

# scalar_table_present() returns 0 if `[name]` is declared at all,
# even with no keys; 1 otherwise. Used for shim/mixed-schema detection.
scalar_table_present() {
  local name="$1"
  awk -v name="$name" '
    /^[[:space:]]*\[\[/ { next }
    /^[[:space:]]*\[/ {
      header = $0
      gsub(/[[:space:]\[\]]/, "", header)
      if (header == name) { found = 1; exit }
    }
    END { exit (found ? 0 : 1) }
  ' "$config_path"
}

# resolve_path() turns a config-relative path into an absolute one,
# leaving already-absolute paths untouched.
resolve_path() {
  case "$1" in
    /*) printf '%s' "$1" ;;
    *)  printf '%s/%s' "$project_root" "$1" ;;
  esac
}

# kv_get() pulls the value for key `$2` out of a `key=value` lines
# blob `$1` (stdin-style strings), or empty if absent.
kv_get() {
  local blob="$1"
  local key="$2"
  printf '%s\n' "$blob" | awk -F= -v k="$key" '
    $1 == k { sub(/^[^=]*=/, ""); print; exit }
  '
}

# kv_to_json() turns a `key=value` lines blob into a JSON object on
# one line. Values are emitted as strings (no type coercion; this
# kit accepts string-valued keys only for now). Compact output so
# downstream `while read -r` loops can consume one block per line.
kv_to_json() {
  local blob="$1"
  printf '%s\n' "$blob" | jq -Rsc '
    split("\n")
    | map(select(length > 0))
    | map(split("=") | {(.[0]): (.[1:] | join("="))})
    | add // {}
  '
}

# ── Project-level keys + marker defaults ────────────────────────────

project_kvs="$(read_scalar project)"
target_default_rel="$(kv_get "$project_kvs" target_path)"
fixit_command="$(kv_get "$project_kvs" fixit_command)"
marker_begin="$(kv_get "$project_kvs" marker_begin)"
marker_end="$(kv_get "$project_kvs" marker_end)"

if [ -z "$target_default_rel" ]; then
  echo "error: attribution.toml is missing required key [project].target_path" >&2
  exit 1
fi
if [ -z "$fixit_command" ]; then
  echo "error: attribution.toml is missing required key [project].fixit_command" >&2
  exit 1
fi
marker_begin="${marker_begin:-<!-- BEGIN AUTO-GENERATED -->}"
marker_end="${marker_end:-<!-- END AUTO-GENERATED -->}"

target_default="$(resolve_path "$target_default_rel")"
splice_input="$target_default"
if [ -z "$target_override" ]; then
  output_path="$target_default"
else
  output_path="$target_override"
fi

if [ ! -f "$splice_input" ]; then
  echo "error: target_path does not exist: $splice_input" >&2
  exit 1
fi

# ── Resolve blocks (with back-compat shim) ──────────────────────────

blocks_count="$(count_array_entries blocks)"
has_flat_rust=0
if scalar_table_present rust; then has_flat_rust=1; fi

if [ "$has_flat_rust" -eq 1 ] && [ "$blocks_count" -gt 0 ]; then
  echo "error: attribution.toml mixes flat [rust] and [[blocks]] schemas." >&2
  echo "  pick one: either keep the legacy flat [rust] table, OR migrate to [[blocks]] entries." >&2
  echo "  the two schemas are mutually exclusive to avoid silent precedence rules." >&2
  exit 1
fi

# RESOLVED_BLOCKS holds one JSON object per block, newline-separated.
# Each object includes "name" + "ecosystem" + every ecosystem-specific
# key the consumer declared. Order matches the source file (or, for
# the shim path, a single unnamed block).
RESOLVED_BLOCKS=""

if [ "$has_flat_rust" -eq 1 ]; then
  # Back-compat shim: synthesise a single unnamed block from [rust].
  rust_kvs="$(read_scalar rust)"
  if [ -z "$rust_kvs" ]; then
    echo "error: [rust] table is empty; nothing to synthesise into a back-compat block." >&2
    exit 1
  fi
  shim_block_json="$(kv_to_json "$rust_kvs" | jq -c --arg name "" --arg ecosystem "rust" '. + {name: $name, ecosystem: $ecosystem}')"
  RESOLVED_BLOCKS="$shim_block_json"
elif [ "$blocks_count" -gt 0 ]; then
  seen_names=""
  i=0
  while [ "$i" -lt "$blocks_count" ]; do
    entry_kvs="$(read_array_entry blocks "$i")"
    entry_json="$(kv_to_json "$entry_kvs")"
    name="$(printf '%s' "$entry_json" | jq -r '.name // ""')"
    ecosystem="$(printf '%s' "$entry_json" | jq -r '.ecosystem // ""')"
    if [ -z "$name" ]; then
      echo "error: [[blocks]] entry #$((i+1)) is missing required key 'name'." >&2
      exit 1
    fi
    if [ -z "$ecosystem" ]; then
      echo "error: [[blocks]] entry '$name' is missing required key 'ecosystem'." >&2
      exit 1
    fi
    case "$seen_names" in
      *"|$name|"*)
        echo "error: duplicate block name '$name' in [[blocks]] — names must be unique within attribution.toml." >&2
        exit 1
        ;;
    esac
    seen_names="$seen_names|$name|"
    # Strict shape check on `name` and `ecosystem` BEFORE they are
    # substituted into filesystem paths or marker text. Rejects:
    #   - path-escape sequences (`..`, `/`) that could resolve outside
    #     the drivers/ directory (e.g. `ecosystem = "../expand-licences"`
    #     would run a sibling script instead of an ecosystem driver)
    #   - whitespace or shell metacharacters that confuse downstream
    #     consumers of the value
    # Kebab-case only (lowercase letters, digits, hyphens) keeps
    # markers unambiguous and matches the spec.
    case "$name" in
      *[!a-z0-9-]* | "" | -* | *- | *--*)
        echo "error: block name '$name' is not valid kebab-case (lowercase letters, digits, hyphens; no leading/trailing or doubled hyphens)." >&2
        exit 1
        ;;
    esac
    case "$ecosystem" in
      *[!a-z0-9-]* | "" | -* | *- | *--*)
        echo "error: ecosystem '$ecosystem' is not valid kebab-case (lowercase letters, digits, hyphens; no leading/trailing or doubled hyphens)." >&2
        exit 1
        ;;
    esac
    driver_script="$drivers_dir/$ecosystem.sh"
    if [ ! -x "$driver_script" ]; then
      echo "error: no driver for ecosystem '$ecosystem' (expected $driver_script to exist and be executable)." >&2
      exit 1
    fi
    if [ -z "$RESOLVED_BLOCKS" ]; then
      RESOLVED_BLOCKS="$entry_json"
    else
      RESOLVED_BLOCKS="$RESOLVED_BLOCKS"$'\n'"$entry_json"
    fi
    i=$((i + 1))
  done
else
  echo "error: attribution.toml declares no blocks." >&2
  echo "  add a [[blocks]] entry or the legacy flat [rust] table." >&2
  exit 1
fi

# ── Per-block marker computation ────────────────────────────────────

marker_for() {
  # Args: <name> <begin|end>; emits the composed marker text.
  local name="$1"
  local kind="$2"
  local base
  if [ "$kind" = "begin" ]; then
    base="$marker_begin"
  else
    base="$marker_end"
  fi
  if [ -z "$name" ]; then
    printf '%s' "$base"
    return
  fi
  # Insert " <name>" immediately before the closing HTML-comment
  # trailer (`-->`), with or without a preceding space, so both
  # `<!-- BEGIN AUTO-GENERATED -->` and `<!-- BEGIN AUTO-GENERATED-->`
  # produce well-formed `<!-- BEGIN AUTO-GENERATED <name> -->`.
  # Marker overrides that don't end with `-->` cannot be safely
  # suffixed (the dispatcher would emit text outside the comment
  # node, breaking the splice gate); fail loud rather than guess.
  case "$base" in
    *' -->')
      printf '%s %s -->' "${base%' -->'}" "$name"
      ;;
    *'-->')
      printf '%s %s -->' "${base%'-->'}" "$name"
      ;;
    *)
      echo "error: marker '$base' does not end with '-->'; per-block name suffix requires an HTML-comment trailer." >&2
      echo "  set [project].marker_begin / marker_end to comments ending in '-->' or use the back-compat shim (no [[blocks]] entries)." >&2
      exit 1
      ;;
  esac
}

# ── Splice loop ──────────────────────────────────────────────────────
# For each block:
#   1. Marker-count gate on the *current* working text (which may have
#      been mutated by a previous iteration's splice).
#   2. Run the ecosystem driver to a per-block temp output file.
#   3. Splice the driver output between the block's markers in the
#      working file.
# At the end, atomic mv working file → output_path (or --check diff).
#
# On any driver failure, abort before the mv: the on-disk target stays
# byte-identical.

working_dir="$(cd "$(dirname "$output_path")" && pwd)"
working_file="$(mktemp "$working_dir/.generate-acknowledgements.work.XXXXXX")"
tmp_driver_outputs_dir="$(mktemp -d)"
# Track per-block splice temps so they are cleaned even when awk fails
# mid-write (set -e exits before our explicit `rm`). Each loop
# iteration creates a fresh `spliced` file; the trap removes any that
# survive an abnormal exit.
splice_temps=""
trap 'rm -f "$working_file" $splice_temps; rm -rf "$tmp_driver_outputs_dir"' EXIT

cp "$splice_input" "$working_file"

block_idx=0
while IFS= read -r block_json; do
  [ -z "$block_json" ] && continue
  name="$(printf '%s' "$block_json" | jq -r '.name')"
  ecosystem="$(printf '%s' "$block_json" | jq -r '.ecosystem')"

  begin_marker="$(marker_for "$name" begin)"
  end_marker="$(marker_for "$name" end)"

  # Per-block marker-count gate.
  begin_count="$(grep -cF "$begin_marker" "$working_file" || true)"
  end_count="$(grep -cF "$end_marker" "$working_file" || true)"
  if [ "$begin_count" != "1" ] || [ "$end_count" != "1" ]; then
    label="${name:-(unnamed)}"
    echo "error: $splice_input must contain exactly one BEGIN and one END marker for block '$label'." >&2
    echo "  '$begin_marker' count: $begin_count (expected 1)" >&2
    echo "  '$end_marker' count: $end_count (expected 1)" >&2
    exit 1
  fi

  # Resolve ecosystem-specific paths (string values only) against project_root.
  resolved_json="$(printf '%s' "$block_json" | jq -c --arg root "$project_root" '
    to_entries
    | map(
        if (.value | type) == "string" and (.key | endswith("_path"))
        then .value = (if (.value | startswith("/")) then .value else ($root + "/" + .value) end)
        else .
        end
      )
    | from_entries
  ')"

  driver_script="$drivers_dir/$ecosystem.sh"
  driver_output="$tmp_driver_outputs_dir/block-$block_idx.md"

  if ! "$driver_script" "$resolved_json" "$driver_output"; then
    echo "" >&2
    echo "error: driver for ecosystem '$ecosystem' (block '${name:-(unnamed)}') exited non-zero." >&2
    echo "  on-disk target $output_path was not modified." >&2
    exit 1
  fi

  if [ ! -s "$driver_output" ]; then
    echo "error: driver for ecosystem '$ecosystem' (block '${name:-(unnamed)}') produced an empty file; refusing to clobber the block." >&2
    exit 1
  fi

  # Splice driver_output between begin_marker and end_marker in working_file.
  spliced="$(mktemp "$working_dir/.generate-acknowledgements.splice.XXXXXX")"
  splice_temps="$splice_temps $spliced"
  awk -v gen="$driver_output" -v begin="$begin_marker" -v end="$end_marker" '
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
  ' "$working_file" > "$spliced"
  mv "$spliced" "$working_file"

  block_idx=$((block_idx + 1))
done <<< "$RESOLVED_BLOCKS"

# ── Drift check or atomic write ─────────────────────────────────────

if [ "$mode" = "check" ]; then
  if ! diff -u "$splice_input" "$working_file"; then
    echo "" >&2
    echo "$splice_input is out of date." >&2
    echo "Run: $fixit_command" >&2
    exit 1
  fi
else
  mv "$working_file" "$output_path"
  # Suppress the trap's removal of the now-moved working file; the
  # driver-outputs temp dir is still owned by the trap.
  trap 'rm -rf "$tmp_driver_outputs_dir"' EXIT
  echo "Updated $output_path"
fi
