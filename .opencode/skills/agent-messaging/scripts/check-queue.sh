#!/bin/bash
# Check Queue Utility
# Inspects pending agent triggers and manages the queue
#
# Usage:
#   ./check-queue.sh                    # Show all pending triggers
#   ./check-queue.sh --priority immediate  # Only immediate priority
#   ./check-queue.sh --pop              # Get and remove next trigger
#   ./check-queue.sh --mark-done <id>   # Mark a trigger as completed
#   ./check-queue.sh --clear            # Clear all triggers
#   ./check-queue.sh --enqueue --trigger <agent> --source <agent> \
#     [--context <text>] [--priority <immediate|queued>]   # Append a trigger
#
# Output Format:
#   --format json (default) | summary | count

set -euo pipefail

# Default values
PRIORITY_FILTER=""
POP_MODE=false
MARK_DONE_ID=""
CLEAR_MODE=false
ENQUEUE_MODE=false
TRIGGER_AGENT=""
SOURCE_AGENT=""
CONTEXT=""
FORMAT="json"

# Parse arguments
while [[ $# -gt 0 ]]; do
    case "$1" in
        --priority)
            PRIORITY_FILTER="$2"
            shift 2
            ;;
        --pop)
            POP_MODE=true
            shift
            ;;
        --mark-done)
            MARK_DONE_ID="$2"
            shift 2
            ;;
        --clear)
            CLEAR_MODE=true
            shift
            ;;
        --enqueue)
            ENQUEUE_MODE=true
            shift
            ;;
        --trigger)
            TRIGGER_AGENT="$2"
            shift 2
            ;;
        --source)
            SOURCE_AGENT="$2"
            shift 2
            ;;
        --context)
            CONTEXT="$2"
            shift 2
            ;;
        --format)
            FORMAT="$2"
            shift 2
            ;;
        -h|--help)
            echo "Usage: check-queue.sh [options]"
            echo ""
            echo "Options:"
            echo "  --priority <level>  Filter by priority (immediate, queued);"
            echo "                      with --enqueue, sets the new trigger's priority"
            echo "  --pop               Get and remove next pending trigger"
            echo "  --mark-done <id>    Mark a specific trigger as completed"
            echo "  --clear             Clear all triggers from queue"
            echo "  --enqueue           Append a pending trigger to the queue"
            echo "  --trigger <agent>   Agent to trigger (required with --enqueue)"
            echo "  --source <agent>    Emitting agent (required with --enqueue)"
            echo "  --context <text>    Context passed to the triggered agent"
            echo "  --format <fmt>      Output format: json (default), summary, count"
            echo "  -h, --help          Show this help"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Trigger-queue state is project-local, never skill-local: <project>/.agent-bus,
# with the project root resolved via git (falling back to pwd). Override the
# location with AGENT_BUS_DIR.
PROJECT_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
QUEUE_DIR="${AGENT_BUS_DIR:-$PROJECT_ROOT/.agent-bus}"
QUEUE_FILE="$QUEUE_DIR/agent-queue.json"
LOCK_FILE="$QUEUE_DIR/.queue.lock"

# Initialize queue if it doesn't exist
if [[ ! -f "$QUEUE_FILE" ]]; then
    mkdir -p "$QUEUE_DIR"
    TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
    echo '{"triggers":[],"lastUpdated":"'"$TIMESTAMP"'"}' > "$QUEUE_FILE"
fi

# OPTIMIZATION: Helper function for atomic file updates
atomic_update() {
    local new_content="$1"
    local tmp_file="${QUEUE_FILE}.tmp.$$"
    echo "$new_content" > "$tmp_file"
    mv "$tmp_file" "$QUEUE_FILE"
}

# Clear mode - with flock for consistency
if [[ "$CLEAR_MODE" == "true" ]]; then
    (
        flock -w 5 200 || exit 1
        TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
        atomic_update '{"triggers":[],"lastUpdated":"'"$TIMESTAMP"'"}'
    ) 200>"$LOCK_FILE"
    echo "Queue cleared"
    exit 0
fi

# Mark done mode - with atomic update
if [[ -n "$MARK_DONE_ID" ]]; then
    (
        flock -w 5 200 || exit 1
        TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
        updated=$(jq --arg id "$MARK_DONE_ID" --arg ts "$TIMESTAMP" '
            .triggers = [.triggers[] | if .id == $id then .status = "completed" else . end] |
            .lastUpdated = $ts
        ' "$QUEUE_FILE")
        atomic_update "$updated"
    ) 200>"$LOCK_FILE"
    echo "Marked $MARK_DONE_ID as completed"
    exit 0
fi

# Enqueue mode - append a pending agentTrigger record (see references/schema.json)
if [[ "$ENQUEUE_MODE" == "true" ]]; then
    if [[ -z "$TRIGGER_AGENT" || -z "$SOURCE_AGENT" ]]; then
        echo "Error: --trigger and --source are required with --enqueue"
        exit 1
    fi
    TRIGGER_PRIORITY="${PRIORITY_FILTER:-queued}"
    case "$TRIGGER_PRIORITY" in
        immediate|queued)
            ;;
        *)
            echo "Error: Invalid priority '$TRIGGER_PRIORITY'. Must be: immediate, queued"
            exit 1
            ;;
    esac
    TRIGGER_ID="trig-$(date +%s)-$(head -c 4 /dev/urandom | od -An -tx1 | tr -d ' \n')"
    (
        flock -w 5 200 || exit 1
        TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
        updated=$(jq \
            --arg id "$TRIGGER_ID" \
            --arg trigger "$TRIGGER_AGENT" \
            --arg source "$SOURCE_AGENT" \
            --arg context "$CONTEXT" \
            --arg priority "$TRIGGER_PRIORITY" \
            --arg ts "$TIMESTAMP" '
            .triggers += [{
                id: $id,
                trigger: $trigger,
                source: $source,
                priority: $priority,
                status: "pending",
                timestamp: $ts
            } + (if $context != "" then {context: $context} else {} end)] |
            .lastUpdated = $ts
        ' "$QUEUE_FILE")
        atomic_update "$updated"
    ) 200>"$LOCK_FILE"
    echo "Enqueued trigger: $TRIGGER_ID" >&2
    echo "$TRIGGER_ID"
    exit 0
fi

# Build jq filter for pending triggers
JQ_FILTER='select(.status == "pending")'
if [[ -n "$PRIORITY_FILTER" ]]; then
    JQ_FILTER="$JQ_FILTER | select(.priority == \"$PRIORITY_FILTER\")"
fi

# Pop mode - get next trigger and mark as processing
# OPTIMIZATION: Single jq call to get trigger + update status
if [[ "$POP_MODE" == "true" ]]; then
    (
        flock -w 5 200 || exit 1

        # OPTIMIZATION: Single jq call to find, sort, get first, and prepare update
        result=$(jq -c "
            ([.triggers[] | $JQ_FILTER] | sort_by(if .priority == \"immediate\" then 0 else 1 end) | .[0]) as \$next |
            if \$next then
                {
                    trigger: \$next,
                    updated: {
                        triggers: [.triggers[] | if .id == \$next.id then .status = \"processing\" else . end],
                        lastUpdated: \"$(date -u +"%Y-%m-%dT%H:%M:%SZ")\"
                    }
                }
            else
                {trigger: null, updated: null}
            end
        " "$QUEUE_FILE")

        next_trigger=$(echo "$result" | jq -c '.trigger')

        if [[ "$next_trigger" == "null" ]]; then
            echo "No pending triggers"
            exit 0
        fi

        # Write updated queue using atomic_update helper
        atomic_update "$(echo "$result" | jq -c '.updated')"

        # Output the trigger
        echo "$next_trigger" | jq .
    ) 200>"$LOCK_FILE"
    exit 0
fi

# Regular query mode
# OPTIMIZATION: Single jq call for all stats
case "$FORMAT" in
    json)
        jq "[.triggers[] | $JQ_FILTER]" "$QUEUE_FILE"
        ;;
    summary)
        jq -r "[.triggers[] | $JQ_FILTER] | .[] | \"[\(.priority | ascii_upcase)] \(.trigger) from \(.source): \(.context | .[0:60])\"" "$QUEUE_FILE"
        ;;
    count)
        # OPTIMIZATION: Single jq call for count + breakdown
        jq -r "
            [.triggers[] | $JQ_FILTER] |
            {
                total: length,
                immediate: [.[] | select(.priority == \"immediate\")] | length,
                queued: [.[] | select(.priority == \"queued\")] | length
            } |
            \"\(.total) pending trigger(s)\" +
            (if .immediate > 0 then \"\n  - \(.immediate) immediate\" else \"\" end) +
            (if .queued > 0 then \"\n  - \(.queued) queued\" else \"\" end)
        " "$QUEUE_FILE"
        ;;
    *)
        echo "Error: Unknown format '$FORMAT'. Use: json, summary, count"
        exit 1
        ;;
esac

exit 0
