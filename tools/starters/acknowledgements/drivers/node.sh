#!/usr/bin/env bash
# Node ecosystem driver. Invoked by generate-acknowledgements.sh with
# two arguments: the block's resolved JSON config and a path where
# rendered markdown should be written.
#
# Block config schema (Node):
#   {
#     "name": "node",
#     "ecosystem": "node",
#     "manifest_path":   "absolute path to package.json",
#     "node_allow_path": "absolute path to licences.node-allow.txt",
#     "prod_only":       true (default) | false,
#     "exclude":         "comma-separated package@version list" (optional)
#   }
#
# Driver-author contract (kept here for reviewer convenience — same
# four rules as drivers/rust.sh):
#   1. Preflight  — verify required tool + state; actionable error on stderr; non-zero exit
#   2. Render     — deterministic markdown sorted by package name@version
#   3. Strict     — reject disallowed licences before render (ATTRIB-012)
#   4. No side effects on the splice target — write only to the
#      <output-temp-path> argument

set -euo pipefail

if [ $# -ne 2 ]; then
  echo "drivers/node.sh: expected 2 arguments (block-config-json, output-temp-path), got $#" >&2
  exit 2
fi

config_json="$1"
output_path="$2"

# ── Required block keys ──────────────────────────────────────────────
manifest_path="$(printf '%s' "$config_json" | jq -er '.manifest_path // empty')" || {
  echo "drivers/node.sh: block is missing required key 'manifest_path'" >&2
  exit 1
}
node_allow_path="$(printf '%s' "$config_json" | jq -er '.node_allow_path // empty')" || {
  echo "drivers/node.sh: block is missing required key 'node_allow_path'" >&2
  exit 1
}

# ── Optional block keys ──────────────────────────────────────────────
# `prod_only` defaults to true (devDependencies stay out of consumer
# ACKNOWLEDGEMENTS by default; consumers wanting devtools opt in by
# setting `prod_only = false` explicitly — that's the ATTRIB-015
# Anvil-devtools path).
prod_only="$(printf '%s' "$config_json" | jq -r '.prod_only // true')"
# `exclude` is forwarded raw to license-checker --excludePackages;
# license-checker expects semicolon-separated `package@version`
# entries. Consumers using globs would need a future driver extension.
exclude="$(printf '%s' "$config_json" | jq -r '.exclude // empty')"

# ── Preflight ────────────────────────────────────────────────────────
# Driver-author contract rule 1: actionable error if any required
# tool or state is missing.
if ! command -v jq >/dev/null 2>&1; then
  echo "drivers/node.sh: jq not installed (required to parse the block-config-json argument)" >&2
  exit 1
fi
if ! command -v license-checker >/dev/null 2>&1; then
  echo "drivers/node.sh: license-checker not installed. Install via npm:" >&2
  echo "  npm install --save-dev license-checker" >&2
  echo "  # or globally: npm install -g license-checker" >&2
  exit 1
fi

if [ ! -f "$manifest_path" ]; then
  echo "drivers/node.sh: manifest_path does not exist: $manifest_path" >&2
  exit 1
fi
if [ ! -f "$node_allow_path" ]; then
  echo "drivers/node.sh: node_allow_path does not exist: $node_allow_path" >&2
  echo "  copy tools/starters/acknowledgements/licences.node-allow.txt.template to your project root" >&2
  echo "  and run tools/starters/acknowledgements/expand-licences.sh to populate it." >&2
  exit 1
fi

manifest_dir="$(cd "$(dirname "$manifest_path")" && pwd)"

# Walk up from the manifest looking for node_modules. license-checker
# resolves through workspace-root node_modules in pnpm/npm-workspace
# layouts, so accepting any ancestor handles both single-repo and
# monorepo shapes.
nm_search="$manifest_dir"
found_nm=""
while [ "$nm_search" != "/" ]; do
  if [ -d "$nm_search/node_modules" ]; then
    found_nm="$nm_search/node_modules"
    break
  fi
  nm_search="$(dirname "$nm_search")"
done
if [ -z "$found_nm" ]; then
  echo "drivers/node.sh: no node_modules found at $manifest_dir or any ancestor." >&2
  echo "  run npm install / pnpm install / yarn install at the workspace root first." >&2
  exit 1
fi

# ── Read allow-list (one semicolon-joined SPDX line) ─────────────────
# `licences.node-allow.txt` carries comment lines (#...) outside the
# markers + the marker lines themselves + one data line between the
# markers. license-checker --onlyAllow takes that data line verbatim.
allow_line="$(grep -v '^#' "$node_allow_path" | grep -v '^[[:space:]]*$' | head -1)"
if [ -z "$allow_line" ]; then
  echo "drivers/node.sh: $node_allow_path is empty between the BEGIN/END markers." >&2
  echo "  run tools/starters/acknowledgements/expand-licences.sh to populate it." >&2
  exit 1
fi

# ── Compose license-checker argv ─────────────────────────────────────
lc_args=( --start "$manifest_dir" --excludePrivatePackages )
if [ "$prod_only" = "true" ]; then
  lc_args+=( --production )
fi
if [ -n "$exclude" ]; then
  lc_args+=( --excludePackages "$exclude" )
fi

# ── Strict gate — must run BEFORE render ─────────────────────────────
# license-checker --onlyAllow exits non-zero on the first disallowed
# licence. Capture stderr so we can attach the allow-list + fix hint
# to the error report.
strict_err="$(mktemp)"
trap 'rm -f "$strict_err"' EXIT
if ! license-checker "${lc_args[@]}" --onlyAllow "$allow_line" >/dev/null 2>"$strict_err"; then
  echo "drivers/node.sh: license-checker --onlyAllow rejected one or more dependencies." >&2
  echo "  allow-list (from $node_allow_path):" >&2
  echo "    $allow_line" >&2
  echo "  license-checker output:" >&2
  sed 's/^/    /' "$strict_err" >&2
  echo "  fix: add the missing licence to licences.toml + rerun expand-licences.sh," >&2
  echo "    or remove the offending package." >&2
  exit 1
fi

# ── Render — license-checker --json piped through jq ─────────────────
# license-checker's JSON keys are `name@version` (unscoped) or
# `@scope/name@version` (scoped). Split on `@`, rejoin all but the
# last segment as the name to handle both shapes deterministically.
# Sort by the original key (ascending) for byte-stable output.
license-checker "${lc_args[@]}" --json | jq -r '
  to_entries
  | sort_by(.key)
  | (
      "| Package | Version | License | Repository |",
      "|---|---|---|---|",
      (
        .[] |
        (.key | split("@")) as $segs |
        ($segs | length) as $n |
        ($segs | .[: $n - 1] | join("@")) as $name |
        ($segs | .[$n - 1]) as $version |
        "| " + $name +
        " | " + $version +
        " | " + (.value.licenses // "UNKNOWN" | tostring) +
        " | " + (.value.repository // "—") +
        " |"
      )
    )
' >"$output_path"

if [ ! -s "$output_path" ]; then
  echo "drivers/node.sh: render produced an empty file; refusing to let the dispatcher splice an empty block." >&2
  exit 1
fi
