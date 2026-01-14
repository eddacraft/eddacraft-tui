#!/bin/bash
# On-Stop Hook - Runs when Claude finishes responding
# Can be used for notifications, logging, or continuation decisions

set -euo pipefail

# Log completion time
LOG_DIR="${CLAUDE_PROJECT_DIR:-$(pwd)}/.claude/logs"
mkdir -p "$LOG_DIR"

TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%S%z")
STOP_REASON="${CLAUDE_STOP_REASON:-unknown}"

# Log the stop event
echo "{\"timestamp\": \"$TIMESTAMP\", \"event\": \"stop\", \"reason\": \"$STOP_REASON\"}" >> "$LOG_DIR/session.log"

# Optional: Desktop notification (if available)
if command -v notify-send &> /dev/null; then
    notify-send "Claude Code" "Task completed" --urgency=low 2>/dev/null || true
fi

# Optional: Sound notification (if available)
if command -v paplay &> /dev/null && [[ -f /usr/share/sounds/freedesktop/stereo/complete.oga ]]; then
    paplay /usr/share/sounds/freedesktop/stereo/complete.oga 2>/dev/null || true
fi

exit 0
