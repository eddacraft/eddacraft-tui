#!/bin/bash
# Send Message Utility
# Sends structured messages between agents via a file-based store, with an
# optional forward to an external observation store (AGENT_BUS_OBSERVER)
#
# Usage:
#   ./send-message.sh --from <agent> --to <agent> --type <type> --payload-file <path>
#   ./send-message.sh --from <agent> --to <agent> --type <type> --payload '<json>'
#   ./send-message.sh -f architect -t security-analyst -T finding -p '{"issue":"SQL injection"}'
#
# Message Types: finding, question, recommendation, alert
# Priority (optional): low, medium (default), high, critical
#
# Examples:
#   ./send-message.sh --from code-reviewer --to architect --type finding \
#     --payload '{"file":"src/auth.ts","issue":"Missing input validation"}'
#
#   ./send-message.sh -f security-analyst -t code-reviewer -T alert -P critical \
#     -p '{"vulnerability":"XSS","severity":"high"}'

set -euo pipefail

# Default values
FROM=""
TO=""
TYPE=""
PAYLOAD="{}"
PAYLOAD_FILE=""
PAYLOAD_SET="false"
PRIORITY="medium"
NEGOTIATION_ID=""

# Parse arguments
while [[ $# -gt 0 ]]; do
    case "$1" in
        -f|--from)
            FROM="$2"
            shift 2
            ;;
        -t|--to)
            TO="$2"
            shift 2
            ;;
        -T|--type)
            TYPE="$2"
            shift 2
            ;;
        -p|--payload)
            PAYLOAD="$2"
            PAYLOAD_SET="true"
            shift 2
            ;;
        --payload-file)
            PAYLOAD_FILE="$2"
            shift 2
            ;;
        -P|--priority)
            PRIORITY="$2"
            shift 2
            ;;
        -n|--negotiation-id)
            NEGOTIATION_ID="$2"
            shift 2
            ;;
        -h|--help)
            echo "Usage: send-message.sh --from <agent> --to <agent> --type <type> (--payload-file <path> | --payload '<json>')"
            echo ""
            echo "Options:"
            echo "  -f, --from           Sending agent name"
            echo "  -t, --to             Receiving agent name"
            echo "  -T, --type           Message type: finding, question, recommendation, alert"
            echo "  -p, --payload        JSON payload with message content"
            echo "      --payload-file   Read the JSON payload from a file (preferred)"
            echo "  -P, --priority       Priority: low, medium (default), high, critical"
            echo "  -n, --negotiation-id Optional link to negotiation session"
            echo "  -h, --help           Show this help"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Validate required fields
if [[ -z "$FROM" || -z "$TO" || -z "$TYPE" ]]; then
    echo "Error: --from, --to, and --type are required"
    echo "Run with --help for usage"
    exit 1
fi

if [[ "$PAYLOAD_SET" == "true" && -n "$PAYLOAD_FILE" ]]; then
    echo "Error: use only one of --payload or --payload-file"
    exit 1
fi

if [[ -n "$PAYLOAD_FILE" ]]; then
    if [[ ! -f "$PAYLOAD_FILE" || ! -r "$PAYLOAD_FILE" ]]; then
        echo "Error: --payload-file must name a readable file"
        exit 1
    fi
    PAYLOAD="$(< "$PAYLOAD_FILE")"
fi

# Validate message type
case "$TYPE" in
    finding|question|recommendation|alert)
        ;;
    *)
        echo "Error: Invalid type '$TYPE'. Must be: finding, question, recommendation, alert"
        exit 1
        ;;
esac

# Validate priority
case "$PRIORITY" in
    low|medium|high|critical)
        ;;
    *)
        echo "Error: Invalid priority '$PRIORITY'. Must be: low, medium, high, critical"
        exit 1
        ;;
esac

# Validate payload is valid JSON
if ! echo "$PAYLOAD" | jq . > /dev/null 2>&1; then
    echo "Error: Payload must be valid JSON"
    exit 1
fi

# Generate message ID and timestamp
MESSAGE_ID="msg-$(date +%s)-$(head -c 6 /dev/urandom | xxd -p)"
TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

# Build the message JSON
MESSAGE=$(jq -n \
    --arg id "$MESSAGE_ID" \
    --arg from "$FROM" \
    --arg to "$TO" \
    --arg type "$TYPE" \
    --arg priority "$PRIORITY" \
    --arg timestamp "$TIMESTAMP" \
    --arg negotiationId "$NEGOTIATION_ID" \
    --argjson payload "$PAYLOAD" \
    '{
        id: $id,
        from: $from,
        to: $to,
        type: $type,
        priority: $priority,
        payload: $payload,
        timestamp: $timestamp
    } + (if $negotiationId != "" then {negotiationId: $negotiationId} else {} end)'
)

# Message state is project-local, never skill-local: <project>/.agent-bus,
# with the project root resolved via git (falling back to pwd). Override the
# location with AGENT_BUS_DIR.
PROJECT_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
MESSAGES_DIR="${AGENT_BUS_DIR:-$PROJECT_ROOT/.agent-bus}"

# Always write to file-based storage for local querying
mkdir -p "$MESSAGES_DIR"

# Write message to recipient-specific file for easy querying
RECIPIENT_FILE="$MESSAGES_DIR/${TO}.jsonl"
echo "$MESSAGE" >> "$RECIPIENT_FILE"

# Also write to a global messages log
echo "$MESSAGE" >> "$MESSAGES_DIR/all-messages.jsonl"

# Optionally forward to an external observation store for long-term
# persistence (non-blocking). Set AGENT_BUS_OBSERVER to an executable that
# accepts: <observer> message <json>. The file store above is canonical;
# this is purely additive and never fails a send.
if [[ -n "${AGENT_BUS_OBSERVER:-}" && -x "${AGENT_BUS_OBSERVER}" ]]; then
    "${AGENT_BUS_OBSERVER}" message "$MESSAGE" 2>/dev/null || true
fi

# Human-readable confirmation to stderr; bare message id to stdout so
# callers can capture it.
echo "Message sent: $MESSAGE_ID" >&2
echo "$MESSAGE_ID"
exit 0
