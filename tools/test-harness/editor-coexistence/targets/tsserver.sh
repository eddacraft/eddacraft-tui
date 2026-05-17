#!/usr/bin/env bash
# tsserver coexistence runner — ADOPT-006.
#
# Uses `tsc --noEmit` as the headless equivalent of opening the fixture
# in an editor with tsserver attached. Exits 200 if the TypeScript
# compiler is not actually resolvable, even when `npx` is on PATH.

set -euo pipefail

case "${1:-}" in
  --print-fixture) echo "fixtures/typescript"; exit 0 ;;
  --run-against) shift; target_dir="${1:?dir required}" ;;
  *) echo "usage: $0 (--print-fixture | --run-against <dir>)" >&2; exit 2 ;;
esac

cmd=()
if command -v tsc >/dev/null 2>&1; then
  cmd=(tsc)
elif command -v npx >/dev/null 2>&1 && npx --no-install tsc --version >/dev/null 2>&1; then
  cmd=(npx --no-install tsc)
else
  exit 200
fi

cd "${target_dir}"
exec "${cmd[@]}" --noEmit
