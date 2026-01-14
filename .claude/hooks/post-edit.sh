#!/bin/bash
# Post-Edit Hook - Quality checks after file modifications
# Runs linting, formatting, and type checking based on file type

set -euo pipefail

TOOL_INPUT="${1:-}"

# Extract file path from tool input (JSON) - using portable sed instead of grep -P
FILE_PATH=$(printf '%s\n' "$TOOL_INPUT" | sed -n 's/.*"file_path"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' 2>/dev/null || echo "")

if [[ -z "$FILE_PATH" ]]; then
    exit 0
fi

# Get file extension
EXT="${FILE_PATH##*.}"

# Project root detection
PROJECT_DIR="${CLAUDE_PROJECT_DIR:-$(pwd)}"

# Function to check if command exists
command_exists() {
    command -v "$1" &> /dev/null
}

# Run checks based on file type
case "$EXT" in
    ts|tsx)
        # TypeScript files
        if [[ -f "$PROJECT_DIR/node_modules/.bin/tsc" ]]; then
            cd "$PROJECT_DIR"
            npx tsc --noEmit "$FILE_PATH" 2>&1 || true
        fi
        if [[ -f "$PROJECT_DIR/node_modules/.bin/eslint" ]]; then
            npx eslint "$FILE_PATH" --fix 2>&1 || true
        fi
        ;;
    js|jsx)
        # JavaScript files
        if [[ -f "$PROJECT_DIR/node_modules/.bin/eslint" ]]; then
            cd "$PROJECT_DIR"
            npx eslint "$FILE_PATH" --fix 2>&1 || true
        fi
        ;;
    py)
        # Python files
        if command_exists ruff; then
            ruff check "$FILE_PATH" --fix 2>&1 || true
            ruff format "$FILE_PATH" 2>&1 || true
        elif command_exists black; then
            black "$FILE_PATH" 2>&1 || true
        fi
        if command_exists mypy; then
            mypy "$FILE_PATH" --ignore-missing-imports 2>&1 || true
        fi
        ;;
    go)
        # Go files
        if command_exists gofmt; then
            gofmt -w "$FILE_PATH" 2>&1 || true
        fi
        if command_exists golint; then
            golint "$FILE_PATH" 2>&1 || true
        fi
        ;;
    rs)
        # Rust files
        if command_exists rustfmt; then
            rustfmt "$FILE_PATH" 2>&1 || true
        fi
        ;;
    json)
        # JSON files - validate syntax
        if command_exists jq; then
            jq empty "$FILE_PATH" 2>&1 || echo "JSON syntax error in $FILE_PATH"
        fi
        ;;
    yaml|yml)
        # YAML files
        if command_exists yamllint; then
            yamllint "$FILE_PATH" 2>&1 || true
        fi
        ;;
esac

exit 0
