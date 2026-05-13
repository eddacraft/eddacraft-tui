#!/usr/bin/env bash
# Anvil TDD guard: opt-in checks before Claude edits source files.

set -euo pipefail

TOOL_INPUT="${1:-}"
if [[ -z "$TOOL_INPUT" && ! -t 0 ]]; then
  TOOL_INPUT=$(cat)
fi
PROJECT_DIR="${CLAUDE_PROJECT_DIR:-$(pwd)}"

FILE_PATH=$(printf '%s\n' "$TOOL_INPUT" | sed -n 's/.*"file_path"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' | head -n1)
if [[ -z "$FILE_PATH" ]]; then
  exit 0
fi

case "$FILE_PATH" in
  "$PROJECT_DIR"/*) REL_PATH="${FILE_PATH#"$PROJECT_DIR"/}" ;;
  *) REL_PATH="$FILE_PATH" ;;
esac

EXT="${FILE_PATH##*.}"
case "$EXT" in
  ts|tsx|js|jsx|rs) ;;
  *) exit 0 ;;
esac

if printf '%s\n' "$REL_PATH" | grep -qiE '(^|/)(__tests__|tests?)/|\.(test|spec)\.|_test\.rs$'; then
  exit 0
fi

TDD_STRICT="${CLAUDE_TDD_STRICT:-false}"
RUN_TESTS="${CLAUDE_TDD_RUN_TESTS:-false}"

test_evidence_exists() {
  local dir base stem
  dir=$(dirname "$REL_PATH")
  base=$(basename "$REL_PATH")
  stem="${base%.*}"

  case "$EXT" in
    ts|tsx|js|jsx)
      [[ -f "$PROJECT_DIR/$dir/$stem.test.$EXT" ]] && return 0
      [[ -f "$PROJECT_DIR/$dir/$stem.spec.$EXT" ]] && return 0
      [[ -f "$PROJECT_DIR/$dir/__tests__/$stem.test.$EXT" ]] && return 0
      [[ -f "$PROJECT_DIR/$dir/__tests__/$stem.spec.$EXT" ]] && return 0
      ;;
    rs)
      grep -q '#\[cfg(test)\]' "$FILE_PATH" 2>/dev/null && return 0
      [[ -d "$PROJECT_DIR/crates" && -d "$PROJECT_DIR/tests" ]] && return 0
      ;;
  esac

  return 1
}

if [[ "$TDD_STRICT" == "true" ]] && ! test_evidence_exists; then
  echo "{\"decision\":\"block\",\"reason\":\"Anvil TDD guard: no nearby test evidence for $REL_PATH. Add or update tests first, or set CLAUDE_TDD_STRICT=false.\"}" >&2
  exit 2
fi

if [[ "$RUN_TESTS" == "true" ]]; then
  cd "$PROJECT_DIR"
  case "$EXT" in
    ts|tsx|js|jsx)
      pnpm exec vitest related "$REL_PATH" --run 2>&1 || {
        echo "{\"decision\":\"block\",\"reason\":\"Anvil TDD guard: related Vitest tests failed for $REL_PATH.\"}" >&2
        exit 2
      }
      ;;
    rs)
      cargo test --workspace 2>&1 || {
        echo "{\"decision\":\"block\",\"reason\":\"Anvil TDD guard: Rust tests failed.\"}" >&2
        exit 2
      }
      ;;
  esac
fi

exit 0
