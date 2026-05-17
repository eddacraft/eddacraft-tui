#!/usr/bin/env bash
# prettier coexistence runner — ADOPT-006.
set -euo pipefail

case "${1:-}" in
  --print-fixture) echo "fixtures/typescript"; exit 0 ;;
  --run-against) shift; target_dir="${1:?dir required}" ;;
  *) echo "usage: $0 (--print-fixture | --run-against <dir>)" >&2; exit 2 ;;
esac

cmd=()
if command -v prettier >/dev/null 2>&1; then
  cmd=(prettier)
elif command -v npx >/dev/null 2>&1 && npx --no-install prettier --version >/dev/null 2>&1; then
  cmd=(npx --no-install prettier)
else
  exit 200
fi

cd "${target_dir}"
exec "${cmd[@]}" --check .
