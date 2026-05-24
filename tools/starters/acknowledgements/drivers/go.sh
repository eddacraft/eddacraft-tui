#!/usr/bin/env bash
# Go ecosystem driver. Invoked by generate-acknowledgements.sh with
# two arguments: the block's resolved JSON config and a path where
# rendered markdown should be written.
#
# Block config schema (Go):
#   {
#     "name": "go",
#     "ecosystem": "go",
#     "module_path":   "absolute path to the package/binary dir to walk (e.g. ./cmd/anvil)",
#     "go_allow_path": "absolute path to licences.go-allow.txt",
#     "template_path": "absolute path to a go-licenses report template (optional;
#                       defaults to the kit's templates/go-licenses.tmpl)"
#   }
#
# Driver-author contract (same four rules as drivers/rust.sh + node.sh):
#   1. Preflight  — verify required tool + state; actionable error on stderr; non-zero exit
#   2. Strict     — reject disallowed licences (go-licenses check) BEFORE render
#   3. Render     — deterministic markdown sorted by module path
#   4. No side effects on the splice target — write only to the
#      <output-temp-path> argument
#
# Determinism note: go-licenses' license *URL* discovery makes network
# calls and degrades to "Unknown" offline, so the rendered block carries
# only the module import path + SPDX licence name (both classified
# locally, network-free). The import path is itself the canonical source
# location for a Go module, so no URL column is needed.

set -euo pipefail

if [ $# -ne 2 ]; then
  echo "drivers/go.sh: expected 2 arguments (block-config-json, output-temp-path), got $#" >&2
  exit 2
fi

config_json="$1"
output_path="$2"

# ── Required + optional block keys ───────────────────────────────────
module_path="$(printf '%s' "$config_json" | jq -er '.module_path // empty')" || {
  echo "drivers/go.sh: block is missing required key 'module_path'" >&2
  exit 1
}
go_allow_path="$(printf '%s' "$config_json" | jq -er '.go_allow_path // empty')" || {
  echo "drivers/go.sh: block is missing required key 'go_allow_path'" >&2
  exit 1
}
template_path="$(printf '%s' "$config_json" | jq -r '.template_path // empty')"

# ── Tool preflight ───────────────────────────────────────────────────
# Each driver checks its own deps so direct invocation (tests, scripts)
# gives the same actionable error rather than a `command not found`.
if ! command -v jq >/dev/null 2>&1; then
  echo "drivers/go.sh: jq not installed (required to parse the block-config-json argument)" >&2
  exit 1
fi
if ! command -v go >/dev/null 2>&1; then
  echo "drivers/go.sh: go not installed (required to resolve the module graph)" >&2
  exit 1
fi
if ! command -v go-licenses >/dev/null 2>&1; then
  echo "drivers/go.sh: go-licenses not installed. Install the version pinned by your project (see CI), e.g.:" >&2
  echo "  go install github.com/google/go-licenses@<GO_LICENSES_VERSION>" >&2
  exit 1
fi

# ── State preflight ──────────────────────────────────────────────────
if [ ! -d "$module_path" ]; then
  echo "drivers/go.sh: module_path is not a directory: $module_path" >&2
  exit 1
fi
if [ ! -f "$go_allow_path" ]; then
  echo "drivers/go.sh: go_allow_path does not exist: $go_allow_path" >&2
  echo "  copy tools/starters/acknowledgements/licences.go-allow.txt.template to your project root" >&2
  echo "  and run tools/starters/acknowledgements/expand-licences.sh to populate it." >&2
  exit 1
fi

# Default template ships with the kit.
driver_dir="$(cd "$(dirname "$0")" && pwd)"
if [ -z "$template_path" ]; then
  template_path="$driver_dir/../templates/go-licenses.tmpl"
fi
if [ ! -f "$template_path" ]; then
  echo "drivers/go.sh: template_path does not exist: $template_path" >&2
  exit 1
fi

# Walk up from module_path for go.mod — that directory is the module
# root go-licenses must run from.
mod_root="$module_path"
while [ "$mod_root" != "/" ] && [ ! -f "$mod_root/go.mod" ]; do
  mod_root="$(dirname "$mod_root")"
done
if [ ! -f "$mod_root/go.mod" ]; then
  echo "drivers/go.sh: no go.mod found at $module_path or any ancestor." >&2
  echo "  the module_path must point inside a Go module; run 'go mod download' so the" >&2
  echo "  module cache is populated before generating attribution." >&2
  exit 1
fi

# Main module path — ignored so the project's own packages are not
# attributed to itself.
main_module="$(cd "$mod_root" && go list -m 2>/dev/null | head -1)"
if [ -z "$main_module" ]; then
  echo "drivers/go.sh: 'go list -m' produced no main module path in $mod_root." >&2
  echo "  run 'go mod download' / ensure the module cache is populated." >&2
  exit 1
fi

# ── Read allow-list (one comma-joined SPDX line) ─────────────────────
# `licences.go-allow.txt` carries the marker lines + one data line
# between them. go-licenses --allowed_licenses takes that line verbatim.
allow_line="$(grep -v '^#' "$go_allow_path" | grep -v '^[[:space:]]*$' | head -1)"
if [ -z "$allow_line" ]; then
  echo "drivers/go.sh: $go_allow_path is empty between the BEGIN/END markers." >&2
  echo "  run tools/starters/acknowledgements/expand-licences.sh to populate it." >&2
  exit 1
fi

# ── Strict gate — must run BEFORE render ─────────────────────────────
# go-licenses check exits non-zero on the first disallowed licence.
# Capture stderr so we can attach the allow-list + fix hint to the
# error report.
strict_err="$(mktemp)"
trap 'rm -f "$strict_err"' EXIT
if ! ( cd "$mod_root" && go-licenses check "$module_path" \
         --ignore "$main_module" \
         --allowed_licenses="$allow_line" ) >/dev/null 2>"$strict_err"; then
  echo "drivers/go.sh: go-licenses check rejected one or more dependencies." >&2
  echo "  allow-list (from $go_allow_path):" >&2
  echo "    $allow_line" >&2
  echo "  go-licenses output:" >&2
  sed 's/^/    /' "$strict_err" >&2
  echo "  fix: add the missing licence to licences.toml + rerun expand-licences.sh," >&2
  echo "    or remove/replace the offending dependency." >&2
  exit 1
fi

# ── Render ───────────────────────────────────────────────────────────
# The template emits one `| <module> | <licence> |` row per third-party
# library. Sort the rows for byte-stable output, then prepend the table
# header. go-licenses' own ordering is graph-derived and not guaranteed
# stable, so sorting here is what makes --check deterministic.
render_err="$(mktemp)"
trap 'rm -f "$strict_err" "$render_err"' EXIT
rows="$( ( cd "$mod_root" && go-licenses report "$module_path" \
            --ignore "$main_module" \
            --template "$template_path" ) 2>"$render_err" \
          | grep -v '^[[:space:]]*$' | LC_ALL=C sort )" || {
  echo "drivers/go.sh: go-licenses report failed." >&2
  sed 's/^/    /' "$render_err" >&2
  exit 1
}

if [ -z "$rows" ]; then
  echo "drivers/go.sh: go-licenses report produced no third-party dependencies for $module_path." >&2
  echo "  if this module genuinely has no third-party deps, omit the go block from attribution.toml." >&2
  exit 1
fi

{
  echo "| Module | License |"
  echo "|---|---|"
  printf '%s\n' "$rows"
} >"$output_path"

if [ ! -s "$output_path" ]; then
  echo "drivers/go.sh: render produced an empty file; refusing to let the dispatcher splice an empty block." >&2
  exit 1
fi
