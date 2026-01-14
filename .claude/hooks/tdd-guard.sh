#!/bin/bash
# TDD Guard Hook - Enforces test-driven development practices
# Blocks implementation changes if tests don't exist or aren't passing
#
# Environment Variables:
#   CLAUDE_TDD_STRICT    - Set to "true" to block edits when no test file exists (default: false)
#   CLAUDE_TDD_RUN_TESTS - Set to "true" to run related tests before allowing edits (default: false)
#
# Configure these in .claude/settings.json under "env" section:

set -euo pipefail

TOOL_INPUT="${1:-}"
PROJECT_DIR="${CLAUDE_PROJECT_DIR:-$(pwd)}"

# Extract file path from tool input (portable sed instead of grep -P)
FILE_PATH=$(printf '%s\n' "$TOOL_INPUT" | sed -n 's/.*"file_path"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' 2>/dev/null | head -n1)
if [[ -z "$FILE_PATH" ]]; then
    FILE_PATH=""
fi

if [[ -z "$FILE_PATH" ]]; then
    exit 0
fi

# Skip non-source files
EXT="${FILE_PATH##*.}"
case "$EXT" in
    md|json|yaml|yml|toml|lock|log|txt|env|gitignore)
        exit 0
        ;;
esac

# Skip test files themselves
if echo "$FILE_PATH" | grep -qiE '(test|spec|_test|\.test\.)'; then
    exit 0
fi

# Skip configuration files
BASENAME=$(basename "$FILE_PATH")
case "$BASENAME" in
    *.config.*|*.conf.*|setup.*|jest.*|vitest.*|tsconfig.*|package.*)
        exit 0
        ;;
esac

# Determine test file pattern based on language
get_test_patterns() {
    local src_file="$1"
    local dir=$(dirname "$src_file")
    local base=$(basename "$src_file")
    local name="${base%.*}"
    local ext="${base##*.}"

    case "$ext" in
        ts|tsx|js|jsx)
            echo "$dir/$name.test.$ext"
            echo "$dir/$name.spec.$ext"
            echo "$dir/__tests__/$name.test.$ext"
            echo "$dir/__tests__/$name.spec.$ext"
            ;;
        py)
            echo "$dir/test_$name.py"
            echo "$dir/${name}_test.py"
            echo "tests/test_$name.py"
            ;;
        go)
            echo "$dir/${name}_test.go"
            ;;
        rs)
            # Rust uses inline tests or tests/ directory
            echo ""
            ;;
    esac
}

# Check if any test file exists
TEST_EXISTS=false
while IFS= read -r pattern; do
    [[ -z "$pattern" ]] && continue
    if [[ -f "$PROJECT_DIR/$pattern" ]]; then
        TEST_EXISTS=true
        break
    fi
done < <(get_test_patterns "${FILE_PATH#$PROJECT_DIR/}")

# TDD Mode check (can be disabled via env var)
TDD_STRICT="${CLAUDE_TDD_STRICT:-false}"

if [[ "$TDD_STRICT" == "true" ]] && [[ "$TEST_EXISTS" == "false" ]]; then
    echo "{\"decision\": \"block\", \"reason\": \"TDD Guard: No test file found for $FILE_PATH. Write tests first!\"}" >&2
    exit 2
fi

# If tests exist, optionally run them
if [[ "$TEST_EXISTS" == "true" ]] && [[ "${CLAUDE_TDD_RUN_TESTS:-false}" == "true" ]]; then
    cd "$PROJECT_DIR"

    case "$EXT" in
        ts|tsx|js|jsx)
            if [[ -f "node_modules/.bin/jest" ]]; then
                npx jest --findRelatedTests "$FILE_PATH" --passWithNoTests 2>&1 || {
                    echo "{\"decision\": \"block\", \"reason\": \"TDD Guard: Related tests are failing\"}" >&2
                    exit 2
                }
            fi
            ;;
        py)
            if command -v pytest &> /dev/null; then
                pytest -x --tb=short 2>&1 || {
                    echo "{\"decision\": \"block\", \"reason\": \"TDD Guard: Tests are failing\"}" >&2
                    exit 2
                }
            fi
            ;;
    esac
fi

exit 0
