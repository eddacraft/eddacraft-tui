#!/usr/bin/env bash
# Anvil post-edit hook: cheap, file-scoped formatting after Claude edits.

set -euo pipefail

if [[ "${CLAUDE_POST_EDIT_LINT:-true}" == "false" ]]; then
  exit 0
fi

TOOL_INPUT="${1:-}"
if [[ -z "$TOOL_INPUT" && ! -t 0 ]]; then
  TOOL_INPUT=$(cat)
fi
PROJECT_DIR="${CLAUDE_PROJECT_DIR:-$(pwd)}"

FILE_PATH=$(printf '%s\n' "$TOOL_INPUT" | sed -n 's/.*"file_path"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' | head -n1)
if [[ -z "$FILE_PATH" || ! -f "$FILE_PATH" ]]; then
  exit 0
fi

case "$FILE_PATH" in
  "$PROJECT_DIR"/*) REL_PATH="${FILE_PATH#"$PROJECT_DIR"/}" ;;
  *) REL_PATH="$FILE_PATH" ;;
esac

case "$(basename "$REL_PATH")" in
  Cargo.lock|pnpm-lock.yaml|package-lock.json)
    exit 0
    ;;
esac

run_oxfmt() {
  cd "$PROJECT_DIR"
  if [[ -x "$PROJECT_DIR/node_modules/.bin/oxfmt" ]]; then
    "$PROJECT_DIR/node_modules/.bin/oxfmt" --write "$REL_PATH" 2>&1 || true
  elif command -v oxfmt >/dev/null 2>&1; then
    oxfmt --write "$REL_PATH" 2>&1 || true
  fi
}

case "${FILE_PATH##*.}" in
  rs)
    if command -v rustfmt >/dev/null 2>&1; then
      rustfmt "$FILE_PATH" 2>&1 || true
    fi
    ;;
  js|jsx|ts|tsx|json|jsonc|md|mdx|css|html|yaml|yml)
    run_oxfmt
    ;;
esac

exit 0
