#!/bin/bash
# Session Start Hook - Initializes development context
# Sets up environment and loads project-specific configurations

set -euo pipefail

PROJECT_DIR="${CLAUDE_PROJECT_DIR:-$(pwd)}"
LOG_DIR="$PROJECT_DIR/.claude/logs"
mkdir -p "$LOG_DIR"

TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%S%z")

# Log session start
echo "{\"timestamp\": \"$TIMESTAMP\", \"event\": \"session_start\", \"project\": \"$PROJECT_DIR\"}" >> "$LOG_DIR/session.log"

# Check for required tools and report status
check_tool() {
    local tool=$1
    if command -v "$tool" &> /dev/null; then
        echo "  [OK] $tool"
    else
        echo "  [MISSING] $tool"
    fi
}

echo "=== Development Environment Check ==="
echo "Project: $PROJECT_DIR"
echo ""
echo "Build Tools:"
check_tool "node"
check_tool "npm"
check_tool "yarn"
check_tool "pnpm"
check_tool "python"
check_tool "pip"
check_tool "go"
check_tool "cargo"

echo ""
echo "Version Control:"
check_tool "git"
check_tool "gh"

echo ""
echo "Quality Tools:"
check_tool "eslint"
check_tool "prettier"
check_tool "ruff"
check_tool "mypy"

# Check for project-specific config files
echo ""
echo "Project Configuration:"
[[ -f "$PROJECT_DIR/package.json" ]] && echo "  [FOUND] package.json"
[[ -f "$PROJECT_DIR/tsconfig.json" ]] && echo "  [FOUND] tsconfig.json"
[[ -f "$PROJECT_DIR/pyproject.toml" ]] && echo "  [FOUND] pyproject.toml"
[[ -f "$PROJECT_DIR/Cargo.toml" ]] && echo "  [FOUND] Cargo.toml"
[[ -f "$PROJECT_DIR/go.mod" ]] && echo "  [FOUND] go.mod"
[[ -f "$PROJECT_DIR/Makefile" ]] && echo "  [FOUND] Makefile"
[[ -f "$PROJECT_DIR/docker-compose.yml" ]] && echo "  [FOUND] docker-compose.yml"

# Git status summary
if [[ -d "$PROJECT_DIR/.git" ]]; then
    echo ""
    echo "Git Status:"
    cd "$PROJECT_DIR"
    BRANCH=$(git branch --show-current 2>/dev/null || echo "unknown")
    CHANGES=$(git status --porcelain 2>/dev/null | wc -l)
    echo "  Branch: $BRANCH"
    echo "  Uncommitted changes: $CHANGES"
fi

echo ""
echo "=== Session Ready ==="

exit 0
