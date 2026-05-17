#!/usr/bin/env bash
# eslint coexistence runner — ADOPT-006.
set -euo pipefail

case "${1:-}" in
  --print-fixture) echo "fixtures/typescript"; exit 0 ;;
  --run-against) shift; target_dir="${1:?dir required}" ;;
  *) echo "usage: $0 (--print-fixture | --run-against <dir>)" >&2; exit 2 ;;
esac

if ! command -v eslint >/dev/null 2>&1 && ! command -v npx >/dev/null 2>&1; then
  exit 200
fi

cd "${target_dir}"
if command -v eslint >/dev/null 2>&1; then
  exec eslint .
else
  exec npx --no-install eslint .
fi
