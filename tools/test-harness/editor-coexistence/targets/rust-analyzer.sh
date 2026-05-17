#!/usr/bin/env bash
# rust-analyzer coexistence runner — ADOPT-006.
#
# Verifies the rust-analyzer binary is installed and that a `cargo check`
# pass against the fixture completes while `anvil watch` runs on the same
# tree. `cargo check` is what rust-analyzer drives internally during
# indexing, so it exercises the same file-I/O pattern that real editor
# use would — without depending on LSP wire-protocol stability across
# rust-analyzer versions. Exits 200 when the binary is absent so the
# harness records a skip.

set -euo pipefail

case "${1:-}" in
  --print-fixture) echo "fixtures/rust"; exit 0 ;;
  --run-against) shift; target_dir="${1:?dir required}" ;;
  *) echo "usage: $0 (--print-fixture | --run-against <dir>)" >&2; exit 2 ;;
esac

if ! command -v rust-analyzer >/dev/null 2>&1; then
  exit 200
fi
if ! command -v cargo >/dev/null 2>&1; then
  exit 200
fi

rust-analyzer --version
cd "${target_dir}"
exec cargo check --quiet --offline
