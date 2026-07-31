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
PROJECT_DIR=$(cd "$PROJECT_DIR" 2>/dev/null && pwd -P) || exit 0

FILE_PATH=$(printf '%s\n' "$TOOL_INPUT" | sed -n 's/.*"file_path"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' | head -n1)
if [[ -z "$FILE_PATH" ]]; then
  exit 0
fi
if [[ "$FILE_PATH" != /* ]]; then
  FILE_PATH="$PROJECT_DIR/$FILE_PATH"
fi
FILE_DIR=$(dirname "$FILE_PATH")
FILE_BASE=$(basename "$FILE_PATH")
FILE_DIR=$(cd "$FILE_DIR" 2>/dev/null && pwd -P) || exit 0
FILE_PATH="$FILE_DIR/$FILE_BASE"

case "$FILE_PATH" in
  "$PROJECT_DIR"/*) REL_PATH="${FILE_PATH#"$PROJECT_DIR"/}" ;;
  *) exit 0 ;;
esac

if [[ ! -f "$FILE_PATH" ]]; then
  exit 0
fi

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
    echo "oxfmt: using global (stale risk); run 'pnpm install' in worktree for local (CIB-032)" >&2
    oxfmt --write "$REL_PATH" 2>&1 || true
  fi
}

case "${FILE_PATH##*.}" in
  rs)
    # Format from the project root so the selected toolchain and
    # rustfmt.toml (edition 2024) apply. Bare rustfmt defaults to 2015.
    (
      cd "$PROJECT_DIR" || exit 0
      if command -v rustfmt >/dev/null 2>&1; then
        rustfmt -- "$REL_PATH" 2>&1 || true
      fi
    )
    ;;
  js|jsx|ts|tsx|json|jsonc|md|mdx|css|html|yaml|yml)
    run_oxfmt
    ;;
esac

exit 0
