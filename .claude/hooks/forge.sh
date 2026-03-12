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
COMMAND=$(echo "$CLAUDE_TOOL_INPUT" | jq -r '.command // empty' 2>/dev/null || echo '')

# If we can't parse a command, explicitly allow the operation
if [[ -z "$COMMAND" ]]; then
    echo '{"decision":"allow","reason":"Forge: unable to parse command from CLAUDE_TOOL_INPUT"}'
    exit 0
fi

# --- Guard: Only trigger on git commit (not amend, not --no-verify) ---
# Use word-boundary-aware matching to avoid false positives from commit
# messages that happen to contain these flag names as substrings.
if [[ ! "$COMMAND" =~ (^|&&|;)[[:space:]]*git[[:space:]]+commit ]] || \
   [[ "$COMMAND" =~ (^|[[:space:]])--amend([[:space:];]|$) ]] || \
   [[ "$COMMAND" =~ (^|[[:space:]])--no-verify([[:space:];]|$) ]]; then
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
# Include epoch, PID ($$), and diff stat to ensure unique hashes even when
# the same files are committed in quick succession from different processes.
HASH_INPUT="$(date +%s)-$$-${STAGED_DIFF_STAT}"
if command -v shasum >/dev/null 2>&1; then
    FORGE_HASH=$(printf '%s' "$HASH_INPUT" | shasum -a 256 | awk '{print substr($1,1,12)}')
elif command -v sha256sum >/dev/null 2>&1; then
    FORGE_HASH=$(printf '%s' "$HASH_INPUT" | sha256sum | awk '{print substr($1,1,12)}')
elif command -v md5 >/dev/null 2>&1; then
    FORGE_HASH=$(printf '%s' "$HASH_INPUT" | md5 | awk '{print substr($1,1,12)}')
elif command -v cksum >/dev/null 2>&1; then
    FORGE_HASH=$(printf '%s' "$HASH_INPUT" | cksum | awk '{printf "%012d", $1}')
else
    FORGE_HASH=$(printf '%s%s' "$HASH_INPUT" "$$" | tr -cd '[:alnum:]' | cut -c1-12)
fi
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
# Build as JSON array for structured signal file; comma-separated for report
STAGED_FILES_CSV=$(git diff --cached --name-only 2>/dev/null | tr '\n' ', ' | sed 's/,$//')
STAGED_FILES_JSON=$(git diff --cached --name-only 2>/dev/null | jq -R . | jq -s '.')

# --- Write the diff to a persistent file for the reviewer ---
# NOTE: Do NOT use a temp file with EXIT trap — the hook exits before the
# forge-reviewer agent can read it. Persist under agent-bus/diffs/ and let
# the orchestration command clean up after the session completes.
DIFF_DIR="${CLAUDE_PROJECT_DIR:-.}/.claude/agent-bus/diffs"
mkdir -p "$DIFF_DIR"
DIFF_FILE="${DIFF_DIR}/forge-${FORGE_HASH}.diff"
printf '%s\n' "$STAGED_DIFF" > "$DIFF_FILE"

# Clean up stale diff files older than 7 days to prevent accumulation
find "$DIFF_DIR" -name "forge-*.diff" -mtime +7 -delete 2>/dev/null || true

# --- Initialize negotiation signal file ---
# Use jq for safe JSON construction — avoids shell injection from filenames
# or diff stat containing quotes/special characters.
jq -n \
  --arg id "$FORGE_HASH" \
  --arg diffFile "$DIFF_FILE" \
  --argjson stagedFiles "$STAGED_FILES_JSON" \
  --arg startedAt "$STARTED_AT" \
  --arg maxRounds "$MAX_ROUNDS" \
  --arg autoDeferNits "$AUTO_DEFER_NITS" \
  '{
    id: $id,
    topic: "Pre-commit review of staged changes",
    participants: ["session", "forge-reviewer"],
    status: "in_progress",
    round: 1,
    maxRounds: ($maxRounds | tonumber? // 3),
    autoDeferNits: ($autoDeferNits == "true"),
    diffFile: $diffFile,
    stagedFiles: $stagedFiles,
    history: [],
    startedAt: $startedAt,
    updatedAt: $startedAt
  }' > "$SIGNAL_FILE"

# --- Initialize forge report ---
# Use a function to safely write the Markdown report, avoiding unquoted
# interpolation of git-derived values (filenames may contain backticks, etc.).
{
  printf '# Forge Report: %s\n\n' "$FORGE_HASH"
  printf '**Started:** %s\n' "$STARTED_AT"
  printf '**Files:** %s\n' "$STAGED_FILES_CSV"
  printf '**Diff size:** %s bytes\n' "$DIFF_SIZE"
  printf '**Max rounds:** %s\n' "$MAX_ROUNDS"
  printf '**Auto-defer nits:** %s\n\n' "$AUTO_DEFER_NITS"
  printf '## Staged Diff Stats\n\n```\n'
  printf '%s\n' "$STAGED_DIFF_STAT"
  printf '```\n\n## Negotiation\n\n'
  printf '_Pending — negotiation will be logged by the forge-reviewer agent._\n'
} > "$FORGE_LOG"

# --- Block the commit and instruct Claude to run negotiation ---
# The hook returns a JSON block message that tells Claude Code to:
#   1. Spawn the forge-reviewer agent with the diff
#   2. Run negotiation rounds
#   3. Apply fixes and re-stage
#   4. File deferred findings
#   5. Then re-attempt the commit
jq -n \
  --arg signalFile "$SIGNAL_FILE" \
  --arg diffFile "$DIFF_FILE" \
  --arg forgeHash "$FORGE_HASH" \
  --arg maxRounds "$MAX_ROUNDS" \
  --arg forgeLog "$FORGE_LOG" \
  '{
    decision: "block",
    reason: ("Forge pre-commit review activated.\n\nReview the staged diff with the forge-reviewer agent before committing.\n\n**Instructions:**\n1. Spawn a forge-reviewer subagent (Task tool, subagent_type: forge-reviewer) with this context:\n   - Signal file: " + $signalFile + "\n   - Diff file: " + $diffFile + "\n   - Forge hash: " + $forgeHash + "\n2. The forge-reviewer will produce structured findings.\n3. For each finding, decide: fix (edit + re-stage), dismiss (with reasoning), or defer (file as issue).\n4. Critical and major findings MUST be fixed — they are not dismissible.\n5. If CLAUDE_FORGE_AUTO_DEFER_NITS is true, nit findings are auto-deferred.\n6. After all findings are resolved, update the signal file status and re-run the commit.\n7. Append outcomes to the forge report at: " + $forgeLog + "\n\nMax " + $maxRounds + " negotiation rounds. After round " + $maxRounds + ", all remaining findings are deferred.")
  }'

exit 0
