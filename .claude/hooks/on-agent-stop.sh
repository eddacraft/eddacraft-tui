#!/usr/bin/env bash
# Anvil subagent stop hook: record lightweight local agent events.

set -euo pipefail

PROJECT_DIR="${CLAUDE_PROJECT_DIR:-$(pwd)}"
LOG_DIR="$PROJECT_DIR/.claude/logs"
mkdir -p "$LOG_DIR"

TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
SOURCE_AGENT="${CLAUDE_AGENT_NAME:-unknown}"
AGENT_TRIGGERS="${CLAUDE_AGENT_TRIGGERS:-false}"
AGENT_OUTPUT="${CLAUDE_TOOL_OUTPUT:-}"

jq -n --arg ts "$TIMESTAMP" --arg agent "$SOURCE_AGENT" \
  '{timestamp: $ts, event: "agent_stop", agent: $agent}' >> "$LOG_DIR/session.log"

if [[ "$AGENT_TRIGGERS" != "true" ]]; then
  exit 0
fi

if [[ -z "$AGENT_OUTPUT" ]]; then
  while IFS= read -r -t 0.1 line; do
    AGENT_OUTPUT="${AGENT_OUTPUT}${AGENT_OUTPUT:+$'\n'}$line"
  done || true
fi

if [[ "$AGENT_OUTPUT" != *TRIGGER:* ]]; then
  exit 0
fi

QUEUE_FILE="$LOG_DIR/agent-queue.json"
LOCK_FILE="$LOG_DIR/.queue.lock"
EPOCH=$(date +%s)

(
  flock -w 5 200 || exit 0
  [[ -f "$QUEUE_FILE" ]] || echo '{"triggers":[],"lastUpdated":"'$TIMESTAMP'"}' > "$QUEUE_FILE"

  trigger_array="[]"
  counter=0
  while IFS= read -r trigger_line; do
    [[ -z "$trigger_line" ]] && continue
    agent_name=$(printf '%s\n' "$trigger_line" | cut -d: -f2)
    context=$(printf '%s\n' "$trigger_line" | cut -d: -f3-)
    trigger_array=$(printf '%s\n' "$trigger_array" | jq \
      --arg id "trg-$EPOCH-$counter" \
      --arg trigger "$agent_name" \
      --arg source "$SOURCE_AGENT" \
      --arg context "$context" \
      --arg timestamp "$TIMESTAMP" \
      '. += [{id: $id, trigger: $trigger, source: $source, context: $context, priority: "queued", timestamp: $timestamp, status: "pending"}]')
    counter=$((counter + 1))
  done < <(printf '%s\n' "$AGENT_OUTPUT" | grep -oE 'TRIGGER:[a-zA-Z0-9_-]+(:.*)?' || true)

  tmp_file="$QUEUE_FILE.tmp.$$"
  jq --argjson new_triggers "$trigger_array" --arg ts "$TIMESTAMP" \
    '.triggers += $new_triggers | .lastUpdated = $ts' "$QUEUE_FILE" > "$tmp_file"
  mv "$tmp_file" "$QUEUE_FILE"
) 200>"$LOCK_FILE"

exit 0
