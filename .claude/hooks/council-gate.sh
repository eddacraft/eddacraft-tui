#!/bin/bash
# Council Gate — optional pre-commit policy hook
#
# Checks whether a valid Council review session exists for the current changes.
# Controlled by CLAUDE_COUNCIL_GATE (default: false).
#
# When enabled, this hook:
# - Fires on git commit commands (PreToolUse:Bash)
# - Checks for a converged Council session covering the staged files
# - Warns (or blocks) if no valid review exists
#
# Modes:
#   CLAUDE_COUNCIL_GATE=warn   — print reminder, allow commit
#   CLAUDE_COUNCIL_GATE=block  — block commit until Council review converges
#   CLAUDE_COUNCIL_GATE=false  — disabled (default)
#
# Hook output format: JSON with "decision" and optional "reason"

set -euo pipefail

# Check if this is a git commit command
TOOL_INPUT="${1:-}"
if ! echo "$TOOL_INPUT" | grep -qE 'git\s+commit'; then
    echo '{"decision":"allow"}'
    exit 0
fi

# Skip amends and no-verify
if echo "$TOOL_INPUT" | grep -qE '\-\-amend|\-\-no-verify'; then
    echo '{"decision":"allow"}'
    exit 0
fi

# Check toggle
GATE_MODE="${CLAUDE_COUNCIL_GATE:-false}"
if [[ "$GATE_MODE" == "false" || "$GATE_MODE" == "0" || -z "$GATE_MODE" ]]; then
    echo '{"decision":"allow"}'
    exit 0
fi

# Find project root
PROJECT_DIR="${CLAUDE_PROJECT_DIR:-$(git rev-parse --show-toplevel 2>/dev/null || pwd)}"
SESSIONS_DIR="$PROJECT_DIR/.claude/council/sessions"

# Get staged files for coverage check
STAGED_FILES=$(git diff --cached --name-only 2>/dev/null || echo "")
if [[ -z "$STAGED_FILES" ]]; then
    echo '{"decision":"allow"}'
    exit 0
fi

# Check for a converged/published session that covers the staged files
covers_staged=false
if [[ -d "$SESSIONS_DIR" ]]; then
    for f in "$SESSIONS_DIR"/*.json; do
        [[ -f "$f" ]] || continue
        status=$(jq -r '.status' "$f" 2>/dev/null || echo "")
        if [[ "$status" != "converged" && "$status" != "published" ]]; then
            continue
        fi

        # Reject stale sessions (older than 24 hours)
        updated_at=$(jq -r '.updatedAt // ""' "$f" 2>/dev/null)
        if [[ -n "$updated_at" ]]; then
            session_epoch=$(date -d "$updated_at" +%s 2>/dev/null || date -jf "%Y-%m-%dT%H:%M:%SZ" "$updated_at" +%s 2>/dev/null || echo 0)
            now_epoch=$(date +%s)
            age=$(( now_epoch - session_epoch ))
            max_age="${CLAUDE_COUNCIL_GATE_MAX_AGE:-86400}"
            if (( age > max_age )); then
                continue  # Skip stale session
            fi
        fi

        # Check target type and coverage
        target_type=$(jq -r '.target.type' "$f" 2>/dev/null || echo "")
        case "$target_type" in
            worktree|staged)
                # These targets review the full working set — accept if session exists
                covers_staged=true
                break
                ;;
            branch)
                # Branch review covers all changes on the branch — accept
                covers_staged=true
                break
                ;;
            files)
                # Check that all staged files are within the session's reviewed files
                reviewed_files=$(jq -r '.target.files[]?' "$f" 2>/dev/null || echo "")
                all_covered=true
                while IFS= read -r staged; do
                    [[ -z "$staged" ]] && continue
                    found=false
                    while IFS= read -r reviewed; do
                        [[ -z "$reviewed" ]] && continue
                        if [[ "$staged" == "$reviewed" || "$staged" == "$reviewed"/* ]]; then
                            found=true
                            break
                        fi
                    done <<< "$reviewed_files"
                    if ! $found; then
                        all_covered=false
                        break
                    fi
                done <<< "$STAGED_FILES"
                if $all_covered; then
                    covers_staged=true
                    break
                fi
                ;;
            commit)
                # Commit reviews are point-in-time — don't count for new staged changes
                ;;
        esac
    done
fi

if $covers_staged; then
    echo '{"decision":"allow"}'
    exit 0
fi

# No valid review found
if [[ "$GATE_MODE" == "block" ]]; then
    cat <<'BLOCK'
{
  "decision": "block",
  "reason": "No converged Council review session covers the staged files. Run `council` to review before committing, or set CLAUDE_COUNCIL_GATE=false to disable this check."
}
BLOCK
    exit 0
fi

# Warn mode (default for any truthy non-block value)
cat <<'WARN'
{
  "decision": "allow",
  "reason": "Reminder: No Council review session covers the staged files. Consider running `council` before committing."
}
WARN
exit 0
