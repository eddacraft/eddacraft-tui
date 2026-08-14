#!/usr/bin/env bash
# Python ecosystem driver. Invoked by generate-acknowledgements.sh with
# two arguments: the block's resolved JSON config and a path where
# rendered markdown should be written.
#
# Block config schema (Python):
#   {
#     "name": "python",
#     "ecosystem": "python",
#     "venv_path":         "absolute path to a consumer-supplied pre-built virtualenv",
#     "python_allow_path": "absolute path to licences.python-allow.txt"
#   }
#
# Driver-author contract (same four rules as the other drivers):
#   1. Preflight  — verify required tool + state; actionable error on stderr; non-zero exit
#   2. Strict     — reject disallowed licences (pip-licenses --allow-only) BEFORE render
#   3. Render     — deterministic markdown (pip-licenses --format markdown --order name)
#   4. No side effects on the splice target — write only to the
#      <output-temp-path> argument
#
# Design: the kit ships no Python tooling opinions (no uv/poetry/pdm).
# The consumer builds their venv with their own installer and includes
# `pip-licenses` in it; this driver runs that venv's pip-licenses so the
# attribution reflects exactly the consumer's environment. pip-licenses
# self-excludes its own dependency chain (pip-licenses/prettytable/...),
# so the rendered block lists only the consumer's real dependencies.

set -euo pipefail

if [ $# -ne 2 ]; then
  echo "drivers/python.sh: expected 2 arguments (block-config-json, output-temp-path), got $#" >&2
  exit 2
fi

config_json="$1"
output_path="$2"

# ── Tool preflight (jq first — the config parse below needs it) ──────
if ! command -v jq >/dev/null 2>&1; then
  echo "drivers/python.sh: jq not installed (required to parse the block-config-json argument)" >&2
  exit 1
fi

# ── Required block keys ──────────────────────────────────────────────
venv_path="$(printf '%s' "$config_json" | jq -er '.venv_path // empty')" || {
  echo "drivers/python.sh: block is missing required key 'venv_path'" >&2
  exit 1
}
python_allow_path="$(printf '%s' "$config_json" | jq -er '.python_allow_path // empty')" || {
  echo "drivers/python.sh: block is missing required key 'python_allow_path'" >&2
  exit 1
}

# ── State preflight ──────────────────────────────────────────────────
if [ ! -d "$venv_path" ]; then
  echo "drivers/python.sh: venv_path is not a directory: $venv_path" >&2
  echo "  build your virtualenv first (e.g. python -m venv / uv venv / poetry env)," >&2
  echo "  install your dependencies + pip-licenses into it, then point venv_path at it." >&2
  exit 1
fi

pip_licenses="$venv_path/bin/pip-licenses"
if [ ! -x "$pip_licenses" ]; then
  echo "drivers/python.sh: pip-licenses not found in the venv at $pip_licenses" >&2
  echo "  the consumer-supplied venv must contain pip-licenses; install it with your" >&2
  echo "  installer (e.g. 'uv pip install pip-licenses' / 'pip install pip-licenses')." >&2
  exit 1
fi

if [ ! -f "$python_allow_path" ]; then
  echo "drivers/python.sh: python_allow_path does not exist: $python_allow_path" >&2
  echo "  copy tools/starters/acknowledgements/licences.python-allow.txt.template to your project root" >&2
  echo "  and run tools/starters/acknowledgements/expand-licences.sh to populate it." >&2
  exit 1
fi

# ── Read allow-list (one semicolon-joined SPDX line) ─────────────────
# `|| true`: grep exits 1 when the file is all comments/blank (no data
# line), which under `set -e`/`pipefail` would abort before the helpful
# emptiness check below.
allow_line="$(grep -v '^#' "$python_allow_path" | grep -v '^[[:space:]]*$' | head -1 || true)"
if [ -z "$allow_line" ]; then
  echo "drivers/python.sh: $python_allow_path is empty between the BEGIN/END markers." >&2
  echo "  run tools/starters/acknowledgements/expand-licences.sh to populate it." >&2
  exit 1
fi

# pip-licenses reports Trove/classifier names, not SPDX. Expand the
# SPDX allow-list with Python-only aliases so a licences.toml of
# Apache-2.0 accepts "Apache Software License". Node/Go/Rust are
# untouched — the table lives beside this driver.
alias_file="$(cd "$(dirname "$0")" && pwd)/python-license-aliases.txt"
allow_for_tool="$allow_line"
if [ -f "$alias_file" ]; then
  while IFS="$(printf '\t')" read -r spdx alias || [ -n "${spdx:-}" ]; do
    case "$spdx" in
      '' | \#*) continue ;;
    esac
    [ -z "${alias:-}" ] && continue
    case ";$allow_line;" in
      *";$spdx;"*) allow_for_tool="$allow_for_tool;$alias" ;;
    esac
  done <"$alias_file"
fi

# ── Strict gate — must run BEFORE render ─────────────────────────────
# pip-licenses --allow-only exits non-zero on the first package whose
# licence is not in the list. Capture stderr to attach the allow-list +
# fix hint to the error report.
strict_err="$(mktemp)"
trap 'rm -f "$strict_err"' EXIT
if ! "$pip_licenses" --allow-only "$allow_for_tool" >/dev/null 2>"$strict_err"; then
  echo "drivers/python.sh: pip-licenses --allow-only rejected one or more dependencies." >&2
  echo "  allow-list (from $python_allow_path, SPDX):" >&2
  echo "    $allow_line" >&2
  echo "  pip-licenses output:" >&2
  sed 's/^/    /' "$strict_err" >&2
  echo "  fix: if pip-licenses printed an SPDX id, add it to licences.toml" >&2
  echo "    and rerun expand-licences.sh. If it printed a classifier name" >&2
  echo "    whose SPDX is already allowed, add an unambiguous alias row" >&2
  echo "    (SPDX<TAB>classifier) to drivers/python-license-aliases.txt." >&2
  echo "    Otherwise remove or replace the offending dependency." >&2
  echo "  do not alias generic names that collapse distinct licences" >&2
  echo "    (plain 'BSD License', generic GPLv3 / LGPLv3)." >&2
  exit 1
fi

# ── Render — pip-licenses markdown, ordered for determinism ──────────
render_err="$(mktemp)"
trap 'rm -f "$strict_err" "$render_err"' EXIT
if ! "$pip_licenses" --format markdown --order name >"$output_path" 2>"$render_err"; then
  echo "drivers/python.sh: pip-licenses render failed." >&2
  sed 's/^/    /' "$render_err" >&2
  exit 1
fi

# pip-licenses always emits the two markdown header rows. A render with
# no data rows means the venv has no attributable dependencies (only the
# self-excluded pip-licenses tool chain) — surface the actionable error
# rather than letting the dispatcher splice an empty block.
data_rows="$(tail -n +3 "$output_path" | grep -c '^|' || true)"
if [ "$data_rows" -eq 0 ]; then
  echo "drivers/python.sh: no installed dependencies found in the venv at $venv_path." >&2
  echo "  install your project's dependencies into the venv with your installer first" >&2
  echo "  (pip-licenses excludes its own tool chain, so a venv with only pip-licenses is empty)." >&2
  exit 1
fi
