#!/bin/sh
# Anvil pre-push hook
# Runs quality gates before push

# Check for ANVIL_SKIP_HOOKS environment variable
if [ -n "$ANVIL_SKIP_HOOKS" ]; then
  echo "Anvil: Skipping hooks (ANVIL_SKIP_HOOKS is set)"
  exit 0
fi

# Find plan files in the repository
PLAN_FILES=$(find . \( -name "*.md" -path "*/planning/*" \) -o -name "*-plan.md" -o -name "*-prd.md" 2>/dev/null | head -5)

if [ -n "$PLAN_FILES" ]; then
  echo "Anvil: Running quality gates..."

  GATE_FAILED=0
  while IFS= read -r file; do
    if [ -f "$file" ]; then
      echo "  Checking: $file"
      if ! anvil gate "$file" 2>/dev/null; then
        echo "  [FAIL] Gate failed: $file"
        echo ""
        echo "Run 'anvil gate $file' to see details."
        echo "To bypass, set ANVIL_SKIP_HOOKS=1"
        GATE_FAILED=1
        break
      fi
    fi
  done <<EOF
$PLAN_FILES
EOF

  if [ "$GATE_FAILED" = "1" ]; then
    exit 1
  fi

  echo "  [OK] All gates passed"
fi

exit 0
