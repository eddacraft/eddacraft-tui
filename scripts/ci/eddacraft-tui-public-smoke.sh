#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
MANIFEST="$ROOT/tests/public-crates/eddacraft-tui-json-pretext/Cargo.toml"

cargo test --manifest-path "$MANIFEST" --all-targets --locked
