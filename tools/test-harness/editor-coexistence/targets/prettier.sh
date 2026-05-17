#!/usr/bin/env bash
# prettier coexistence runner — ADOPT-006.
set -euo pipefail

case "${1:-}" in
  --print-fixture) echo "fixtures/typescript"; exit 0 ;;
  --run-against) shift; target_dir="${1:?dir required}" ;;
  *) echo "usage: $0 (--print-fixture | --run-against <dir>)" >&2; exit 2 ;;
esac

if ! command -v prettier >/dev/null 2>&1 && ! command -v npx >/dev/null 2>&1; then
  exit 200
fi

cd "${target_dir}"
if command -v prettier >/dev/null 2>&1; then
  exec prettier --check .
else
  exec npx --no-install prettier --check .
fi
