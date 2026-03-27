#!/bin/bash
# Local Review Pre-commit Hook (Council Gate guardrail)
#
# This hook is an OPTIONAL GUARDRAIL, not the primary review interface.
# The primary review workflow is explicit Council sessions via:
#   /local-review-council or .claude/council/council-session.sh
#
# When enabled, this hook fires on git commit and reminds the agent
# to check Council session state before proceeding. It does NOT block
# commits or replace Council as the review workflow.
#
# Config:
#   CLAUDE_LOCAL_REVIEW_PRECOMMIT=false (default: off)
#   Set to "true" to enable guardrail reminders before commit.

set -euo pipefail

# Guardrail is OFF by default — explicit sessions are the primary review
if [[ "${CLAUDE_LOCAL_REVIEW_PRECOMMIT:-false}" != "true" ]]; then
    exit 0
fi

# Read hook input from positional argument
HOOK_INPUT="${1:-}"

# Parse tool input from hook JSON
COMMAND=$(echo "$HOOK_INPUT" | jq -r '.tool_input.command // empty' 2>/dev/null)

if [[ -z "$COMMAND" ]]; then
    exit 0
fi

# Only trigger on git commit commands (not amend or --no-verify)
if [[ ! "$COMMAND" =~ (^|&&|;)[[:space:]]*git[[:space:]]+commit ]] || [[ "$COMMAND" =~ --amend ]] || [[ "$COMMAND" =~ --no-verify ]]; then
    exit 0
fi

# Check if there are staged changes
STAGED_DIFF=$(git diff --cached --stat 2>/dev/null)
if [[ -z "$STAGED_DIFF" ]]; then
    exit 0
fi

# Check for an active Council session
SESSIONS_DIR="${CLAUDE_PROJECT_DIR:-.}/.claude/council/sessions"
if [[ -d "$SESSIONS_DIR" ]]; then
    LATEST=$(ls -t "$SESSIONS_DIR"/council-*.json 2>/dev/null | head -1)
    if [[ -n "$LATEST" ]]; then
        STATUS=$(jq -r '.status // "unknown"' "$LATEST" 2>/dev/null || echo "unknown")
        SESSION_ID=$(jq -r '.id // "unknown"' "$LATEST" 2>/dev/null || echo "unknown")
        if [[ "$STATUS" == "active" ]]; then
            echo "[Council] Active review session found (${SESSION_ID}, status: ${STATUS}). Consider closing it before committing." >&2
        fi
    else
        echo "[Council] No Council session found. Consider running /local-review-council before committing." >&2
    fi
fi

exit 0
