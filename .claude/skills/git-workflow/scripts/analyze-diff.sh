#!/bin/bash
# Git diff analysis helper script
# Analyzes staged changes and provides suggestions for commit type and scope

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}=== Git Diff Analysis ===${NC}\n"

# Check if there are staged changes
if ! git diff --cached --quiet 2>/dev/null; then
    echo -e "${GREEN}✓ Staged changes detected${NC}\n"
else
    echo -e "${RED}✗ No staged changes found${NC}"
    echo "Run: git add <files> to stage changes"
    exit 1
fi

# Show stats
echo -e "${BLUE}Changed Files:${NC}"
git diff --cached --stat

echo -e "\n${BLUE}File Changes Summary:${NC}"
ADDED=$(git diff --cached --numstat | awk '{added+=$1} END {print added}')
REMOVED=$(git diff --cached --numstat | awk '{removed+=$2} END {print removed}')
FILES=$(git diff --cached --name-only | wc -l)

echo "Files changed: ${FILES}"
echo -e "Lines added: ${GREEN}+${ADDED}${NC}"
echo -e "Lines removed: ${RED}-${REMOVED}${NC}"

# Analyze file types
echo -e "\n${BLUE}File Types:${NC}"
git diff --cached --name-only | sed 's/.*\.//' | sort | uniq -c | sort -rn

# Suggest commit type based on changes
echo -e "\n${BLUE}Suggested Commit Type:${NC}"

# Check for new files
NEW_FILES=$(git diff --cached --diff-filter=A --name-only | wc -l)
DELETED_FILES=$(git diff --cached --diff-filter=D --name-only | wc -l)
MODIFIED_FILES=$(git diff --cached --diff-filter=M --name-only | wc -l)

# Detect patterns
HAS_TEST_CHANGES=$(git diff --cached --name-only | grep -E "(test|spec)\." | wc -l)
HAS_DOC_CHANGES=$(git diff --cached --name-only | grep -E "\.(md|txt|rst)$" | wc -l)
HAS_CONFIG_CHANGES=$(git diff --cached --name-only | grep -E "(config|\.json|\.ya?ml|\.toml|\.env)$" | wc -l)
HAS_BUILD_CHANGES=$(git diff --cached --name-only | grep -E "(package\.json|Cargo\.toml|go\.mod|requirements\.txt|Dockerfile|Makefile)" | wc -l)

# Analysis logic
if [ "$HAS_DOC_CHANGES" -gt 0 ] && [ "$MODIFIED_FILES" -eq "$HAS_DOC_CHANGES" ]; then
    echo -e "  ${GREEN}docs${NC} - Only documentation files changed"
elif [ "$HAS_TEST_CHANGES" -gt 0 ] && [ "$MODIFIED_FILES" -eq "$HAS_TEST_CHANGES" ]; then
    echo -e "  ${GREEN}test${NC} - Only test files changed"
elif [ "$HAS_CONFIG_CHANGES" -gt 0 ] || [ "$HAS_BUILD_CHANGES" -gt 0 ]; then
    echo -e "  ${GREEN}chore${NC} - Configuration or build files changed"
elif [ "$NEW_FILES" -gt "$MODIFIED_FILES" ]; then
    echo -e "  ${GREEN}feat${NC} - Mostly new files (likely new feature)"
elif [ "$DELETED_FILES" -gt 0 ]; then
    echo -e "  ${GREEN}refactor${NC} or ${GREEN}chore${NC} - Files deleted"
else
    echo -e "  ${GREEN}feat${NC} - New functionality (if adding capability)"
    echo -e "  ${GREEN}fix${NC} - Bug fix (if correcting behavior)"
    echo -e "  ${GREEN}refactor${NC} - Code restructuring (if no behavior change)"
    echo -e "  ${GREEN}perf${NC} - Performance improvement"
fi

# Suggest scope based on file paths
echo -e "\n${BLUE}Suggested Scope (based on file paths):${NC}"
git diff --cached --name-only | sed 's/\/.*//g' | sort | uniq -c | sort -rn | head -5 | while read count dir; do
    if [ "$dir" != "." ]; then
        echo "  ${GREEN}${dir}${NC} (${count} files)"
    fi
done

# Check for common patterns
echo -e "\n${BLUE}Pattern Detection:${NC}"

# API changes
if git diff --cached --name-only | grep -qE "(api|endpoint|route|controller)"; then
    echo -e "  ${YELLOW}•${NC} API changes detected - consider scope: ${GREEN}api${NC}"
fi

# UI changes
if git diff --cached --name-only | grep -qE "(component|view|page|ui|frontend)"; then
    echo -e "  ${YELLOW}•${NC} UI changes detected - consider scope: ${GREEN}ui${NC}"
fi

# Database changes
if git diff --cached --name-only | grep -qE "(migration|model|schema|db|database)"; then
    echo -e "  ${YELLOW}•${NC} Database changes detected - consider scope: ${GREEN}db${NC}"
fi

# Auth changes
if git diff --cached --name-only | grep -qE "(auth|login|session|token)"; then
    echo -e "  ${YELLOW}•${NC} Authentication changes detected - consider scope: ${GREEN}auth${NC}"
fi

# Check for breaking changes indicators
echo -e "\n${BLUE}Breaking Change Indicators:${NC}"
BREAKING_FOUND=0

# Check for removed functions/exports
if git diff --cached | grep -E "^-.*export (function|class|const|let|var)" > /dev/null 2>&1; then
    echo -e "  ${RED}⚠${NC} Removed exports detected - may be breaking"
    BREAKING_FOUND=1
fi

# Check for changed function signatures
if git diff --cached | grep -E "^[-+].*(function|const.*=.*\()" | grep -v test > /dev/null 2>&1; then
    echo -e "  ${YELLOW}⚠${NC} Function signatures changed - review for breaking changes"
    BREAKING_FOUND=1
fi

# Check for API endpoint changes
if git diff --cached | grep -E "^[-+].*\.(get|post|put|delete|patch)\(" > /dev/null 2>&1; then
    echo -e "  ${YELLOW}⚠${NC} API endpoints modified - review for breaking changes"
    BREAKING_FOUND=1
fi

if [ "$BREAKING_FOUND" -eq 0 ]; then
    echo -e "  ${GREEN}✓${NC} No obvious breaking changes detected"
fi

# Check for common issues
echo -e "\n${BLUE}Potential Issues:${NC}"
ISSUES_FOUND=0

# Check for console.log
if git diff --cached | grep -E "^\+.*console\.(log|debug|info)" > /dev/null 2>&1; then
    echo -e "  ${YELLOW}⚠${NC} console.log statements added - remove before commit"
    ISSUES_FOUND=1
fi

# Check for debugger
if git diff --cached | grep -E "^\+.*debugger" > /dev/null 2>&1; then
    echo -e "  ${RED}⚠${NC} debugger statement added - remove before commit"
    ISSUES_FOUND=1
fi

# Check for TODO/FIXME
if git diff --cached | grep -E "^\+.*(TODO|FIXME|XXX|HACK)" > /dev/null 2>&1; then
    echo -e "  ${YELLOW}⚠${NC} TODO/FIXME comments added - track in issue tracker"
    ISSUES_FOUND=1
fi

# Check for large files
LARGE_FILES=$(git diff --cached --numstat | awk '$1 > 500 || $2 > 500 {print $3}')
if [ ! -z "$LARGE_FILES" ]; then
    echo -e "  ${YELLOW}⚠${NC} Large file changes (>500 lines):"
    echo "$LARGE_FILES" | sed 's/^/    /'
    ISSUES_FOUND=1
fi

if [ "$ISSUES_FOUND" -eq 0 ]; then
    echo -e "  ${GREEN}✓${NC} No obvious issues detected"
fi

# Generate suggested commit message
echo -e "\n${BLUE}=== Suggested Commit Message ===${NC}\n"

# Try to intelligently determine type and scope
TYPE="feat"
SCOPE=""

if [ "$HAS_DOC_CHANGES" -gt 0 ] && [ "$MODIFIED_FILES" -eq "$HAS_DOC_CHANGES" ]; then
    TYPE="docs"
elif [ "$HAS_TEST_CHANGES" -gt 0 ] && [ "$MODIFIED_FILES" -eq "$HAS_TEST_CHANGES" ]; then
    TYPE="test"
elif [ "$HAS_CONFIG_CHANGES" -gt 0 ] || [ "$HAS_BUILD_CHANGES" -gt 0 ]; then
    TYPE="chore"
fi

# Get most common directory as scope
COMMON_DIR=$(git diff --cached --name-only | sed 's/\/.*//g' | grep -v "^\." | sort | uniq -c | sort -rn | head -1 | awk '{print $2}')
if [ ! -z "$COMMON_DIR" ] && [ "$COMMON_DIR" != "." ]; then
    SCOPE="($COMMON_DIR)"
fi

echo -e "${GREEN}${TYPE}${SCOPE}: <subject>${NC}"
echo ""
echo "<optional body>"
echo ""
echo "<optional footer>"

echo -e "\n${BLUE}=== Example ===${NC}\n"
echo -e "${GREEN}${TYPE}${SCOPE}: add user authentication${NC}"
echo ""
echo "Implement OAuth2 authentication flow with Google provider."
echo "Includes token refresh and profile fetching."
echo ""
echo "Closes #123"

echo -e "\n${BLUE}=== Next Steps ===${NC}\n"
echo "1. Review the changes: git diff --cached"
echo "2. Write commit message following Conventional Commits"
echo "3. Commit: git commit -m \"type(scope): subject\""
echo ""
echo "See .claude/skills/git-workflow/commit-patterns.md for detailed examples"

exit 0
