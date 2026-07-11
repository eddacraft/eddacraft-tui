#!/usr/bin/env bash
# Anvil session-start hook: record local session context and surface tool state.

set -euo pipefail

PROJECT_DIR="${CLAUDE_PROJECT_DIR:-$(pwd)}"
LOG_DIR="$PROJECT_DIR/.claude/logs"
mkdir -p "$LOG_DIR"

TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
if command -v jq >/dev/null 2>&1; then
  jq -n --arg ts "$TIMESTAMP" --arg project "$PROJECT_DIR" \
    '{timestamp: $ts, event: "session_start", project: $project}' >> "$LOG_DIR/session.log"
else
  printf '{"timestamp":"%s","event":"session_start","project":"%s"}\n' \
    "$TIMESTAMP" "$PROJECT_DIR" >> "$LOG_DIR/session.log"
fi

check_tool() {
  if command -v "$1" >/dev/null 2>&1; then
    printf '  [OK] %s\n' "$1"
  else
    printf '  [MISSING] %s\n' "$1"
  fi
}

printf '=== Anvil Session Check ===\n'
printf 'Project: %s\n\n' "$PROJECT_DIR"

printf 'Required tools:\n'
check_tool node
check_tool pnpm
check_tool cargo
check_tool rustfmt
check_tool git
check_tool gh
check_tool jq

printf '\nOptional policy/docs tools:\n'
check_tool opa
check_tool regal
check_tool markdownlint

if [[ -d "$PROJECT_DIR/.git" ]]; then
  cd "$PROJECT_DIR"
  printf '\nGit status:\n'
  printf '  Branch: %s\n' "$(git branch --show-current 2>/dev/null || printf unknown)"
  printf '  Uncommitted changes: %s\n' "$(git status --porcelain 2>/dev/null | wc -l | tr -d ' ')"
fi


# Surface pending continuous-improvement notes (shared across worktrees).
if [[ -f "$PROJECT_DIR/scripts/ci-log/status.mjs" ]] && command -v node >/dev/null 2>&1; then
  printf '\nContinuous improvement log:\n'
  if status_out="$(cd "$PROJECT_DIR" && node scripts/ci-log/status.mjs 2>/dev/null)"; then
    printf '%s\n' "$status_out" | sed -n 's/^/  /p' | head -n 12
  else
    printf '  (ci-log:status unavailable)\n'
  fi
fi

printf '\nValidation commands:\n'
printf '  pnpm format:check && pnpm lint:check && pnpm typecheck && pnpm test\n'
printf '  cargo test --workspace\n'
printf '=== Session Ready ===\n'

exit 0
