#!/bin/sh
# Anvil pre-push hook
# Runs quality gates before push

# Source logging library if available
PROJECT_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
_LOG_LIB="${PROJECT_ROOT}/.claude/hooks/lib/log.sh"
if [ -f "$_LOG_LIB" ]; then
  export ANVIL_LOG_TAG="anvil:pre-push"
  # shellcheck source=../../../.claude/hooks/lib/log.sh
  . "$_LOG_LIB"
fi

type log_enter >/dev/null 2>&1 && log_enter "$@"
type log_debug >/dev/null 2>&1 && log_debug "hook=pre-push"

# Check for ANVIL_SKIP_HOOKS environment variable
if [ -n "$ANVIL_SKIP_HOOKS" ]; then
  echo "Anvil: Skipping hooks (ANVIL_SKIP_HOOKS is set)"
  type log_info >/dev/null 2>&1 && log_info "skipping hooks (ANVIL_SKIP_HOOKS=${ANVIL_SKIP_HOOKS})"
  type log_exit >/dev/null 2>&1 && log_exit 0
  exit 0
fi

# Find plan files in the repository
PLAN_FILES=$(find . \( -name "*.md" -path "*/planning/*" \) -o -name "*-plan.md" -o -name "*-prd.md" 2>/dev/null | head -5)
PLAN_COUNT=$(echo "$PLAN_FILES" | grep -c . 2>/dev/null || echo "0")
type log_debug >/dev/null 2>&1 && log_debug "plan_files_found=${PLAN_COUNT}"

if [ -n "$PLAN_FILES" ]; then
  echo "Anvil: Running quality gates..."
  type log_info >/dev/null 2>&1 && log_info "running quality gates on ${PLAN_COUNT} file(s)"

  GATE_FAILED=0
  while IFS= read -r file; do
    if [ -f "$file" ]; then
      echo "  Checking: $file"
      type log_debug >/dev/null 2>&1 && log_debug "running gate on: $file"
      if ! anvil gate "$file" 2>/dev/null; then
        echo "  [FAIL] Gate failed: $file"
        echo ""
        echo "Run 'anvil gate $file' to see details."
        echo "To bypass, set ANVIL_SKIP_HOOKS=1"
        type log_error >/dev/null 2>&1 && log_error "gate failed: $file"
        GATE_FAILED=1
        break
      fi
      type log_debug >/dev/null 2>&1 && log_debug "gate passed: $file"
    fi
  done <<EOF
$PLAN_FILES
EOF

  if [ "$GATE_FAILED" = "1" ]; then
    type log_exit >/dev/null 2>&1 && log_exit 1
    exit 1
  fi

  echo "  [OK] All gates passed"
  type log_info >/dev/null 2>&1 && log_info "all quality gates passed"
fi

type log_exit >/dev/null 2>&1 && log_exit 0
exit 0
