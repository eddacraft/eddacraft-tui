#!/bin/sh
# Anvil pre-commit hook
# Validates planning documents before commit

# Source logging library if available
PROJECT_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
_LOG_LIB="${PROJECT_ROOT}/.claude/hooks/lib/log.sh"
if [ -f "$_LOG_LIB" ]; then
  export ANVIL_LOG_TAG="anvil:pre-commit"
  # shellcheck source=../../../.claude/hooks/lib/log.sh
  . "$_LOG_LIB"
fi

type log_enter >/dev/null 2>&1 && log_enter "$@"
type log_debug >/dev/null 2>&1 && log_debug "hook=pre-commit"

# Find modified plan files
PLAN_FILES=$(git diff --cached --name-only --diff-filter=ACM | grep -E '\.(md|yaml|yml|json)$' || true)
type log_debug >/dev/null 2>&1 && log_debug "plan_files_count=$(echo "$PLAN_FILES" | grep -c . 2>/dev/null || echo 0)"

if [ -n "$PLAN_FILES" ]; then
  echo "Anvil: Validating planning documents..."
  type log_info >/dev/null 2>&1 && log_info "validating planning documents"
  FAILED=0

  echo "$PLAN_FILES" | while IFS= read -r file; do
    type log_trace >/dev/null 2>&1 && log_trace "validating: $file"
    if anvil validate "$file" --quiet 2>/dev/null; then
      echo "  [OK] $file"
      type log_debug >/dev/null 2>&1 && log_debug "validated OK: $file"
    else
      echo "  [FAIL] $file"
      type log_debug >/dev/null 2>&1 && log_debug "validated FAIL: $file"
      FAILED=1
    fi
  done

  if [ "$FAILED" -ne 0 ]; then
    echo ""
    echo "Commit blocked: one or more plan files failed validation."
    echo "Run 'anvil validate <file>' to see details."
    type log_exit >/dev/null 2>&1 && log_exit 1
    exit 1
  fi

  type log_info >/dev/null 2>&1 && log_info "planning document validation complete"
fi

type log_exit >/dev/null 2>&1 && log_exit 0
exit 0
