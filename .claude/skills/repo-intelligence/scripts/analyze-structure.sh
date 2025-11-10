#!/bin/bash
# Repository structure analysis script
# Provides quick overview of repository organization and key metrics

set -e

# Colors
BLUE='\033[0;34m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${BLUE}=== Repository Structure Analysis ===${NC}\n"

# Project root info
if [ -f "package.json" ]; then
    PROJECT_NAME=$(cat package.json | grep '"name"' | head -1 | sed 's/.*"name": "\(.*\)".*/\1/')
    echo -e "${GREEN}Project:${NC} $PROJECT_NAME"
fi

# Detect tech stack
echo -e "\n${BLUE}Tech Stack:${NC}"

if [ -f "package.json" ]; then
    echo "  • Node.js/JavaScript/TypeScript"
    if grep -q "react" package.json; then
        echo "  • React"
    fi
    if grep -q "vue" package.json; then
        echo "  • Vue"
    fi
    if grep -q "express" package.json; then
        echo "  • Express"
    fi
fi

if [ -f "requirements.txt" ] || [ -f "pyproject.toml" ]; then
    echo "  • Python"
    if [ -f "manage.py" ]; then
        echo "  • Django"
    fi
    if grep -q "fastapi" requirements.txt 2>/dev/null || grep -q "fastapi" pyproject.toml 2>/dev/null; then
        echo "  • FastAPI"
    fi
fi

if [ -f "Cargo.toml" ]; then
    echo "  • Rust"
fi

if [ -f "go.mod" ]; then
    echo "  • Go"
fi

# Directory structure
echo -e "\n${BLUE}Directory Structure:${NC}"

# Find key directories
DIRS=("src" "lib" "app" "components" "services" "api" "routes" "models" "views" "tests" "test" "__tests__" "docs" "scripts")

for dir in "${DIRS[@]}"; do
    if [ -d "$dir" ]; then
        COUNT=$(find "$dir" -type f | wc -l)
        echo "  • $dir/ ($COUNT files)"
    fi
done

# File type analysis
echo -e "\n${BLUE}File Types:${NC}"

# Count by extension
find . -type f -not -path "*/node_modules/*" -not -path "*/.git/*" -not -path "*/dist/*" -not -path "*/build/*" |
  sed 's/.*\.//' |
  sort |
  uniq -c |
  sort -rn |
  head -10 |
  while read count ext; do
    echo "  • .$ext: $count files"
  done

# Code metrics
echo -e "\n${BLUE}Code Metrics:${NC}"

# Total lines of code (excluding common ignore patterns)
TOTAL_LOC=$(find . -type f \
  -not -path "*/node_modules/*" \
  -not -path "*/.git/*" \
  -not -path "*/dist/*" \
  -not -path "*/build/*" \
  -not -path "*/coverage/*" \
  \( -name "*.js" -o -name "*.ts" -o -name "*.tsx" -o -name "*.jsx" -o -name "*.py" -o -name "*.rs" -o -name "*.go" \) \
  -exec wc -l {} + 2>/dev/null | tail -1 | awk '{print $1}')

echo "  • Total LOC: $TOTAL_LOC"

# Count files
FILE_COUNT=$(find . -type f \
  -not -path "*/node_modules/*" \
  -not -path "*/.git/*" \
  -not -path "*/dist/*" \
  \( -name "*.js" -o -name "*.ts" -o -name "*.tsx" -o -name "*.jsx" -o -name "*.py" -o -name "*.rs" -o -name "*.go" \) | wc -l)

echo "  • Code files: $FILE_COUNT"

# Test files
TEST_COUNT=$(find . -type f \
  -not -path "*/node_modules/*" \
  \( -name "*.test.*" -o -name "*.spec.*" -o -path "*/tests/*" -o -path "*/__tests__/*" \) | wc -l)

echo "  • Test files: $TEST_COUNT"

# TODO/FIXME count
echo -e "\n${BLUE}Code Quality Indicators:${NC}"

TODO_COUNT=$(grep -r "TODO\|FIXME\|XXX\|HACK" --include="*.js" --include="*.ts" --include="*.tsx" --include="*.jsx" --include="*.py" --include="*.rs" --include="*.go" \
  --exclude-dir=node_modules --exclude-dir=.git --exclude-dir=dist --exclude-dir=build 2>/dev/null | wc -l)

echo "  • TODO/FIXME comments: $TODO_COUNT"

# Complexity indicators (files >300 lines)
LARGE_FILES=$(find . -type f \
  -not -path "*/node_modules/*" \
  -not -path "*/.git/*" \
  \( -name "*.js" -o -name "*.ts" -o -name "*.tsx" -o -name "*.jsx" -o -name "*.py" -o -name "*.rs" -o -name "*.go" \) \
  -exec wc -l {} + 2>/dev/null | awk '$1 > 300' | wc -l)

echo "  • Large files (>300 LOC): $LARGE_FILES"

# Documentation
echo -e "\n${BLUE}Documentation:${NC}"

if [ -f "README.md" ]; then
    README_LINES=$(wc -l < README.md)
    echo -e "  ${GREEN}✓${NC} README.md ($README_LINES lines)"
else
    echo -e "  ${YELLOW}✗${NC} README.md missing"
fi

if [ -d "docs" ]; then
    DOC_COUNT=$(find docs -name "*.md" | wc -l)
    echo -e "  ${GREEN}✓${NC} docs/ directory ($DOC_COUNT markdown files)"
fi

# ADRs
ADR_COUNT=$(find . -name "ADR-*.md" -o -name "adr-*.md" | wc -l)
if [ "$ADR_COUNT" -gt 0 ]; then
    echo -e "  ${GREEN}✓${NC} Architecture Decision Records: $ADR_COUNT"
else
    echo "  • No ADRs found"
fi

# Git info
if [ -d ".git" ]; then
    echo -e "\n${BLUE}Git Repository:${NC}"

    # Recent activity
    LAST_COMMIT=$(git log -1 --format="%ar")
    echo "  • Last commit: $LAST_COMMIT"

    # Contributors
    CONTRIBUTORS=$(git log --format='%an' | sort -u | wc -l)
    echo "  • Contributors: $CONTRIBUTORS"

    # Branches
    BRANCH_COUNT=$(git branch -r 2>/dev/null | wc -l)
    echo "  • Remote branches: $BRANCH_COUNT"

    # Commits last 30 days
    RECENT_COMMITS=$(git log --since="30 days ago" --oneline | wc -l)
    echo "  • Commits (last 30 days): $RECENT_COMMITS"
fi

# Dependencies
echo -e "\n${BLUE}Dependencies:${NC}"

if [ -f "package.json" ]; then
    PROD_DEPS=$(cat package.json | grep -A 999 '"dependencies"' | grep -B 999 '^  }' | grep ': "' | wc -l)
    DEV_DEPS=$(cat package.json | grep -A 999 '"devDependencies"' | grep -B 999 '^  }' | grep ': "' | wc -l)
    echo "  • npm production: $PROD_DEPS"
    echo "  • npm development: $DEV_DEPS"

    # Check for outdated
    if command -v npm &> /dev/null; then
        OUTDATED=$(npm outdated 2>/dev/null | wc -l)
        if [ "$OUTDATED" -gt 1 ]; then
            echo -e "  ${YELLOW}⚠${NC} Outdated packages: $((OUTDATED - 1))"
        fi
    fi
fi

if [ -f "requirements.txt" ]; then
    PY_DEPS=$(grep -v '^#' requirements.txt | grep -v '^$' | wc -l)
    echo "  • Python packages: $PY_DEPS"
fi

if [ -f "Cargo.toml" ]; then
    RUST_DEPS=$(grep -A 999 '^\[dependencies\]' Cargo.toml | grep -B 999 '^\[' | grep ' = ' | wc -l)
    echo "  • Rust crates: $RUST_DEPS"
fi

# Configuration files
echo -e "\n${BLUE}Configuration:${NC}"

[ -f ".eslintrc.js" ] || [ -f ".eslintrc.json" ] && echo -e "  ${GREEN}✓${NC} ESLint configured"
[ -f ".prettierrc" ] || [ -f ".prettierrc.json" ] && echo -e "  ${GREEN}✓${NC} Prettier configured"
[ -f "tsconfig.json" ] && echo -e "  ${GREEN}✓${NC} TypeScript configured"
[ -f ".editorconfig" ] && echo -e "  ${GREEN}✓${NC} EditorConfig present"
[ -f ".gitignore" ] && echo -e "  ${GREEN}✓${NC} .gitignore present"

# CI/CD
if [ -d ".github/workflows" ]; then
    WORKFLOW_COUNT=$(find .github/workflows -name "*.yml" -o -name "*.yaml" | wc -l)
    echo -e "  ${GREEN}✓${NC} GitHub Actions ($WORKFLOW_COUNT workflows)"
fi

[ -f ".gitlab-ci.yml" ] && echo -e "  ${GREEN}✓${NC} GitLab CI configured"
[ -f "Jenkinsfile" ] && echo -e "  ${GREEN}✓${NC} Jenkins configured"

# Testing
echo -e "\n${BLUE}Testing Setup:${NC}"

if [ -f "package.json" ]; then
    if grep -q "jest" package.json; then
        echo -e "  ${GREEN}✓${NC} Jest configured"
    fi
    if grep -q "vitest" package.json; then
        echo -e "  ${GREEN}✓${NC} Vitest configured"
    fi
    if grep -q "cypress" package.json; then
        echo -e "  ${GREEN}✓${NC} Cypress configured"
    fi
fi

if [ -f "pytest.ini" ] || grep -q "pytest" requirements.txt 2>/dev/null; then
    echo -e "  ${GREEN}✓${NC} pytest configured"
fi

# Summary
echo -e "\n${BLUE}=== Summary ===${NC}"
echo -e "Codebase: ${GREEN}$TOTAL_LOC${NC} lines across ${GREEN}$FILE_COUNT${NC} files"
echo -e "Tests: ${GREEN}$TEST_COUNT${NC} test files"
echo -e "Quality: ${YELLOW}$TODO_COUNT${NC} TODO items, ${YELLOW}$LARGE_FILES${NC} large files"

if [ "$TEST_COUNT" -gt 0 ] && [ "$FILE_COUNT" -gt 0 ]; then
    TEST_RATIO=$((TEST_COUNT * 100 / FILE_COUNT))
    if [ "$TEST_RATIO" -gt 50 ]; then
        echo -e "Test coverage: ${GREEN}Good${NC} (test to code file ratio: $TEST_RATIO%)"
    elif [ "$TEST_RATIO" -gt 20 ]; then
        echo -e "Test coverage: ${YELLOW}Moderate${NC} (test to code file ratio: $TEST_RATIO%)"
    else
        echo -e "Test coverage: ${YELLOW}Low${NC} (test to code file ratio: $TEST_RATIO%)"
    fi
fi

exit 0
