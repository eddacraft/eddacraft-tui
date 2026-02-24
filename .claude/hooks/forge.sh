#!/bin/bash
# Forge pre-commit hook — intercepts git commit, launches review negotiation
# Triggers on: PreToolUse (Bash) matching git commit commands
# Requires: CLAUDE_FORGE_ENABLED=true
#
# Config:
#   CLAUDE_FORGE_ENABLED=false            - Master toggle (default: off)
#   CLAUDE_FORGE_MAX_ROUNDS=3             - Max negotiation rounds
#   CLAUDE_FORGE_AUTO_DEFER_NITS=true     - Auto-defer nit findings without negotiation

set -e

# --- Guard: Forge must be explicitly enabled ---
if [[ "${CLAUDE_FORGE_ENABLED:-false}" != "true" ]]; then
    exit 0
fi

# --- Guard: Only process Bash tool calls ---
if [[ "$CLAUDE_TOOL_NAME" != "Bash" ]]; then
    exit 0
fi

# --- Parse the command from tool input ---
COMMAND=$(echo "$CLAUDE_TOOL_INPUT" | jq -r '.command // empty' 2>/dev/null)

# --- Guard: Only trigger on git commit (not amend, not --no-verify) ---
if [[ ! "$COMMAND" =~ (^|&&|;)[[:space:]]*git[[:space:]]+commit ]] || \
   [[ "$COMMAND" =~ --amend ]] || \
   [[ "$COMMAND" =~ --no-verify ]]; then
    exit 0
fi

# --- Guard: Must have staged changes ---
STAGED_DIFF_STAT=$(git diff --cached --stat 2>/dev/null)
if [[ -z "$STAGED_DIFF_STAT" ]]; then
    exit 0
fi

# --- Capture the staged diff ---
STAGED_DIFF=$(git diff --cached 2>/dev/null)

# Guard: Skip very large diffs (>100KB) — not practical to review inline
DIFF_SIZE=${#STAGED_DIFF}
if [[ $DIFF_SIZE -gt 102400 ]]; then
    echo '{"decision":"allow","reason":"Forge: diff too large (>100KB), skipping review"}'
    exit 0
fi

# --- Derive session identifiers ---
FORGE_HASH=$(echo -n "$(date +%s)-$$-${STAGED_DIFF_STAT}" | sha256sum | cut -c1-12)
FORGE_LOG_DIR="${CLAUDE_PROJECT_DIR:-.}/.claude/logs"
FORGE_LOG="${FORGE_LOG_DIR}/forge-${FORGE_HASH}.md"
SIGNAL_DIR="${CLAUDE_PROJECT_DIR:-.}/.claude/agent-bus/signals"
SIGNAL_FILE="${SIGNAL_DIR}/forge-${FORGE_HASH}.json"

mkdir -p "$FORGE_LOG_DIR" "$SIGNAL_DIR"

# --- Configuration ---
MAX_ROUNDS="${CLAUDE_FORGE_MAX_ROUNDS:-3}"
AUTO_DEFER_NITS="${CLAUDE_FORGE_AUTO_DEFER_NITS:-true}"
STARTED_AT=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

# --- Staged file list (for scoped review context) ---
STAGED_FILES=$(git diff --cached --name-only 2>/dev/null | tr '\n' ', ' | sed 's/,$//')

# --- Write the diff to a temp file for the reviewer ---
DIFF_FILE=$(mktemp)
echo "$STAGED_DIFF" > "$DIFF_FILE"
trap "rm -f '$DIFF_FILE'" EXIT

# --- Initialize negotiation signal file ---
cat > "$SIGNAL_FILE" << SIGNAL_EOF
{
  "id": "${FORGE_HASH}",
  "topic": "Pre-commit review of staged changes",
  "participants": ["session", "forge-reviewer"],
  "status": "in_progress",
  "round": 1,
  "maxRounds": ${MAX_ROUNDS},
  "autoDeferNits": ${AUTO_DEFER_NITS},
  "diffFile": "${DIFF_FILE}",
  "stagedFiles": "${STAGED_FILES}",
  "history": [],
  "startedAt": "${STARTED_AT}",
  "updatedAt": "${STARTED_AT}"
}
SIGNAL_EOF

# --- Initialize forge report ---
cat > "$FORGE_LOG" << LOG_EOF
# Forge Report: ${FORGE_HASH}

**Started:** ${STARTED_AT}
**Files:** ${STAGED_FILES}
**Diff size:** ${DIFF_SIZE} bytes
**Max rounds:** ${MAX_ROUNDS}
**Auto-defer nits:** ${AUTO_DEFER_NITS}

## Staged Diff Stats

\`\`\`
${STAGED_DIFF_STAT}
\`\`\`

## Negotiation

_Pending — negotiation will be logged by the forge-reviewer agent._
LOG_EOF

# --- Block the commit and instruct Claude to run negotiation ---
# The hook returns a JSON block message that tells Claude Code to:
#   1. Spawn the forge-reviewer agent with the diff
#   2. Run negotiation rounds
#   3. Apply fixes and re-stage
#   4. File deferred findings
#   5. Then re-attempt the commit
cat << BLOCK_EOF
{
  "decision": "block",
  "reason": "Forge pre-commit review activated.\n\nReview the staged diff with the forge-reviewer agent before committing.\n\n**Instructions:**\n1. Spawn a forge-reviewer subagent (Task tool, subagent_type: forge-reviewer) with this context:\n   - Signal file: ${SIGNAL_FILE}\n   - Diff file: ${DIFF_FILE}\n   - Forge hash: ${FORGE_HASH}\n2. The forge-reviewer will produce structured findings.\n3. For each finding, decide: fix (edit + re-stage), dismiss (with reasoning), or defer (file as issue).\n4. Critical and major findings MUST be fixed — they are not dismissable.\n5. If CLAUDE_FORGE_AUTO_DEFER_NITS is true, nit findings are auto-deferred.\n6. After all findings are resolved, update the signal file status and re-run the commit.\n7. Append outcomes to the forge report at: ${FORGE_LOG}\n\nMax ${MAX_ROUNDS} negotiation rounds. After round ${MAX_ROUNDS}, all remaining findings are deferred."
}
BLOCK_EOF

exit 0
