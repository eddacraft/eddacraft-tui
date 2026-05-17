#!/usr/bin/env bash
# pyright coexistence runner — ADOPT-006.
set -euo pipefail

case "${1:-}" in
  --print-fixture) echo "fixtures/python"; exit 0 ;;
  --run-against) shift; target_dir="${1:?dir required}" ;;
  *) echo "usage: $0 (--print-fixture | --run-against <dir>)" >&2; exit 2 ;;
esac

if ! command -v pyright >/dev/null 2>&1; then
  exit 200
fi

cd "${target_dir}"
exec pyright .
