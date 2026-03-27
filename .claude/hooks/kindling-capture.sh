#!/bin/bash
# Kindling Capture Hook
# Captures agent messages and negotiation data to Kindling
#
# Usage:
#   ./kindling-capture.sh message '{"from":"agent1","to":"agent2",...}'
#   ./kindling-capture.sh negotiation '{"id":"neg-123",...}'
#
# Requires: @kindling/cli to be installed and linked

set -euo pipefail

CAPTURE_TYPE="${1:-}"
PAYLOAD="${2:-}"

if [[ -z "$CAPTURE_TYPE" || -z "$PAYLOAD" ]]; then
    echo "Usage: kindling-capture.sh <message|negotiation> '<json_payload>'"
    exit 1
fi

# Check if kindling CLI is available
if ! command -v kindling &> /dev/null; then
    # Fallback to file-based storage if Kindling not installed
    LOG_DIR="${CLAUDE_PROJECT_DIR:-$(pwd)}/.claude/agent-bus/fallback"
    mkdir -p "$LOG_DIR"

    TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%S%z")

    case "$CAPTURE_TYPE" in
        message)
            echo "{\"timestamp\": \"$TIMESTAMP\", \"type\": \"agent_message\", \"data\": $PAYLOAD}" \
                >> "$LOG_DIR/messages.jsonl"
            ;;
        negotiation)
            # Extract negotiation ID from payload
            NEG_ID=$(echo "$PAYLOAD" | grep -oP '"id"\s*:\s*"\K[^"]+' || echo "unknown")
            echo "$PAYLOAD" > "$LOG_DIR/negotiation-${NEG_ID}.json"
            ;;
        *)
            echo "Unknown capture type: $CAPTURE_TYPE"
            exit 1
            ;;
    esac

    echo "Captured to fallback storage (Kindling not available)"
    exit 0
fi

# Use Kindling CLI to capture
case "$CAPTURE_TYPE" in
    message)
        # Capture as agent_message observation
        kindling capture \
            --type "agent_message" \
            --data "$PAYLOAD" \
            2>/dev/null || {
                echo "Failed to capture message to Kindling"
                exit 1
            }
        ;;
    negotiation)
        # Capture negotiation state
        # Create or update capsule for this negotiation
        NEG_ID=$(echo "$PAYLOAD" | grep -oP '"id"\s*:\s*"\K[^"]+' || echo "unknown")
        STATUS=$(echo "$PAYLOAD" | grep -oP '"status"\s*:\s*"\K[^"]+' || echo "in_progress")

        if [[ "$STATUS" == "in_progress" ]]; then
            # Start or update negotiation capsule
            kindling capsule open \
                --type "negotiation" \
                --id "$NEG_ID" \
                --data "$PAYLOAD" \
                2>/dev/null || true
        else
            # Close negotiation capsule with outcome
            kindling capsule close \
                --id "$NEG_ID" \
                --data "$PAYLOAD" \
                2>/dev/null || true
        fi
        ;;
    *)
        echo "Unknown capture type: $CAPTURE_TYPE"
        exit 1
        ;;
esac

echo "Captured to Kindling"
exit 0
