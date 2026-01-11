#!/bin/sh
# Anvil pre-commit hook
# Validates planning documents before commit

# Find modified plan files
PLAN_FILES=$(git diff --cached --name-only --diff-filter=ACM | grep -E '\.(md|yaml|yml|json)$' || true)

if [ -n "$PLAN_FILES" ]; then
  echo "Anvil: Validating planning documents..."

  for file in $PLAN_FILES; do
    if anvil validate "$file" --quiet 2>/dev/null; then
      echo "  ✓ $file"
    fi
  done
fi

exit 0
