#!/bin/bash
# Receive Messages Utility
# Retrieves messages addressed to a specific agent from the file-based store
#
# Usage:
#   ./receive-messages.sh <agent-name> [options]
#
# Options:
#   --type <type>      Filter by message type (finding, question, recommendation, alert)
#   --from <agent>     Filter by sender agent
#   --priority <level> Filter by priority (low, medium, high, critical)
#   --since <time>     Only messages after this ISO timestamp
#   --limit <n>        Maximum number of messages to return (default: 50)
#   --unread           Only show messages not yet marked as read
#   --mark-read        Mark returned messages as read
#   --format <fmt>     Output format: json (default), summary
#
# Examples:
#   ./receive-messages.sh code-reviewer
#   ./receive-messages.sh architect --type alert --priority high
#   ./receive-messages.sh security-analyst --from code-reviewer --since 2024-01-01T00:00:00Z
#   ./receive-messages.sh debugger --format summary --limit 10

set -euo pipefail

# Default values
AGENT=""
FILTER_TYPE=""
FILTER_FROM=""
FILTER_PRIORITY=""
FILTER_SINCE=""
LIMIT=50
UNREAD_ONLY=false
MARK_READ=false
FORMAT="json"

# Parse arguments
if [[ $# -lt 1 ]]; then
    echo "Usage: receive-messages.sh <agent-name> [options]"
    echo "Run with --help for more options"
    exit 1
fi

AGENT="$1"
shift

while [[ $# -gt 0 ]]; do
    case "$1" in
        --type)
            FILTER_TYPE="$2"
            shift 2
            ;;
        --from)
            FILTER_FROM="$2"
            shift 2
            ;;
        --priority)
            FILTER_PRIORITY="$2"
            shift 2
            ;;
        --since)
            FILTER_SINCE="$2"
            shift 2
            ;;
        --limit)
            LIMIT="$2"
            shift 2
            ;;
        --unread)
            UNREAD_ONLY=true
            shift
            ;;
        --mark-read)
            MARK_READ=true
            shift
            ;;
        --format)
            FORMAT="$2"
            shift 2
            ;;
        -h|--help)
            echo "Usage: receive-messages.sh <agent-name> [options]"
            echo ""
            echo "Options:"
            echo "  --type <type>      Filter by message type"
            echo "  --from <agent>     Filter by sender agent"
            echo "  --priority <level> Filter by priority"
            echo "  --since <time>     Only messages after this ISO timestamp"
            echo "  --limit <n>        Maximum messages to return (default: 50)"
            echo "  --unread           Only show unread messages"
            echo "  --mark-read        Mark returned messages as read"
            echo "  --format <fmt>     Output format: json (default), summary"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Validate agent name
if [[ -z "$AGENT" ]]; then
    echo "Error: Agent name is required"
    exit 1
fi

# Message state is project-local, never skill-local: <project>/.agent-bus,
# with the project root resolved via git (falling back to pwd). Override the
# location with AGENT_BUS_DIR.
PROJECT_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
MESSAGES_DIR="${AGENT_BUS_DIR:-$PROJECT_ROOT/.agent-bus}"
READ_MARKER_DIR="$MESSAGES_DIR/.read-markers"

# Build jq filter
JQ_FILTER='select(.to == $agent)'

if [[ -n "$FILTER_TYPE" ]]; then
    JQ_FILTER="$JQ_FILTER | select(.type == \"$FILTER_TYPE\")"
fi

if [[ -n "$FILTER_FROM" ]]; then
    JQ_FILTER="$JQ_FILTER | select(.from == \"$FILTER_FROM\")"
fi

if [[ -n "$FILTER_PRIORITY" ]]; then
    JQ_FILTER="$JQ_FILTER | select(.priority == \"$FILTER_PRIORITY\")"
fi

if [[ -n "$FILTER_SINCE" ]]; then
    JQ_FILTER="$JQ_FILTER | select(.timestamp >= \"$FILTER_SINCE\")"
fi

# Function to read messages from the canonical file store.
# OPTIMIZED: Use streaming jq and early termination
get_from_files() {
    local messages_file="$MESSAGES_DIR/${AGENT}.jsonl"

    if [[ ! -f "$messages_file" ]]; then
        echo "[]"
        return 0
    fi

    # OPTIMIZATION: Check file size first - use streaming for large files
    local file_size
    file_size=$(stat -f%z "$messages_file" 2>/dev/null || stat -c%s "$messages_file" 2>/dev/null || echo "0")

    local messages

    # For small files (<100KB), use simple approach
    if [[ "$file_size" -lt 102400 ]]; then
        messages=$(jq -s --arg agent "$AGENT" "[.[] | $JQ_FILTER] | .[-$LIMIT:]" "$messages_file" 2>/dev/null || echo "[]")
    else
        # OPTIMIZATION: For large files, use tail + streaming to only process recent messages
        # Read last 500 lines max (most recent), then filter
        messages=$(tail -n 500 "$messages_file" | jq -s --arg agent "$AGENT" "[.[] | $JQ_FILTER] | .[-$LIMIT:]" 2>/dev/null || echo "[]")
    fi

    # Handle unread filter
    if [[ "$UNREAD_ONLY" == "true" ]]; then
        mkdir -p "$READ_MARKER_DIR"
        local read_file="$READ_MARKER_DIR/${AGENT}.read"
        if [[ -f "$read_file" ]]; then
            # OPTIMIZATION: Convert read IDs to object for O(1) has() lookup
            messages=$(jq --slurpfile ids "$read_file" '($ids[0] | map({(.): true}) | add // {}) as $lookup | [.[] | select(($lookup | has(.id)) | not)]' <<< "$messages" 2>/dev/null || echo "$messages")
        fi
    fi

    echo "$messages"
}

# Get messages from the canonical file store
MESSAGES=$(get_from_files)

# Mark as read if requested
if [[ "$MARK_READ" == "true" ]]; then
    mkdir -p "$READ_MARKER_DIR"
    read_file="$READ_MARKER_DIR/${AGENT}.read"

    # OPTIMIZATION: Single jq call to extract IDs and merge
    if [[ -f "$read_file" ]]; then
        # Merge existing + new in single jq invocation
        echo "$MESSAGES" | jq --slurpfile existing "$read_file" '[.[].id] + $existing[0] | unique' > "${read_file}.tmp"
        mv "${read_file}.tmp" "$read_file"
    else
        # No existing file, just write new IDs
        echo "$MESSAGES" | jq '[.[].id]' > "$read_file"
    fi
fi

# OPTIMIZATION: Format output and get count in single jq call where possible
case "$FORMAT" in
    json)
        COUNT=$(echo "$MESSAGES" | jq 'length')
        echo "$MESSAGES" | jq .
        echo "# Found $COUNT message(s) for $AGENT" >&2
        ;;
    summary)
        echo "$MESSAGES" | jq -r '.[] | "\(.timestamp | split("T")[0]) [\(.priority | ascii_upcase)] \(.from) -> \(.to): \(.type) - \(.payload | tostring | .[0:80])"'
        ;;
    *)
        echo "Error: Unknown format '$FORMAT'. Use: json, summary"
        exit 1
        ;;
esac

exit 0
