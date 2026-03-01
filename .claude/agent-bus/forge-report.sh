#!/bin/bash
# forge-report.sh — Append structured content to a Forge report
#
# Usage:
#   forge-report.sh <forge-hash> round-start <round> <agent>
#   forge-report.sh <forge-hash> findings <round> <json-findings>
#   forge-report.sh <forge-hash> responses <round> <json-responses>
#   forge-report.sh <forge-hash> round-summary <round> <outcome>
#   forge-report.sh <forge-hash> deferred <json-deferred-findings>
#   forge-report.sh <forge-hash> complete <outcome> <total-rounds>
#
# The report file is at .claude/logs/forge-{hash}.md (created by forge.sh hook)

set -e

FORGE_HASH="$1"
ACTION="$2"
shift 2

REPORT_DIR="${CLAUDE_PROJECT_DIR:-.}/.claude/logs"
REPORT_FILE="${REPORT_DIR}/forge-${FORGE_HASH}.md"

if [[ ! -f "$REPORT_FILE" ]]; then
    echo "Error: Report file not found: ${REPORT_FILE}" >&2
    exit 1
fi

TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

case "$ACTION" in
    round-start)
        ROUND="$1"
        AGENT="$2"
        cat >> "$REPORT_FILE" << EOF

---

### Round ${ROUND}: ${AGENT}

**Time:** ${TIMESTAMP}

EOF
        ;;

    findings)
        ROUND="$1"
        FINDINGS_JSON="$2"
        # Parse JSON findings into a markdown table
        FINDING_COUNT=$(echo "$FINDINGS_JSON" | jq 'length' 2>/dev/null || echo "0")
        NITS=$(echo "$FINDINGS_JSON" | jq '[.[] | select(.severity == "nit")] | length' 2>/dev/null || echo "0")
        AUTO_DEFERRED=$(echo "$FINDINGS_JSON" | jq '[.[] | select(.status == "auto-deferred")] | length' 2>/dev/null || echo "0")

        cat >> "$REPORT_FILE" << EOF
**Findings:** ${FINDING_COUNT} (${NITS} nits, ${AUTO_DEFERRED} auto-deferred)

| ID | File | Severity | Category | Description | Status |
| -- | ---- | -------- | -------- | ----------- | ------ |
EOF
        # Append each finding as a table row
        echo "$FINDINGS_JSON" | jq -r '.[] | "| \(.id) | `\(.file):\(.line)` | \(.severity) | \(.category) | \(.description) | \(.status // "pending") |"' 2>/dev/null >> "$REPORT_FILE" || true
        echo "" >> "$REPORT_FILE"
        ;;

    responses)
        ROUND="$1"
        RESPONSES_JSON="$2"

        cat >> "$REPORT_FILE" << EOF
#### Author Responses (Round ${ROUND})

| Finding | Action | Reasoning |
| ------- | ------ | --------- |
EOF
        echo "$RESPONSES_JSON" | jq -r '.[] | "| \(.findingId) | \(.action) | \(.reasoning // "-") |"' 2>/dev/null >> "$REPORT_FILE" || true
        echo "" >> "$REPORT_FILE"
        ;;

    round-summary)
        ROUND="$1"
        OUTCOME="$2"

        cat >> "$REPORT_FILE" << EOF
**Round ${ROUND} outcome:** ${OUTCOME}

EOF
        ;;

    deferred)
        DEFERRED_JSON="$1"
        DEFERRED_COUNT=$(echo "$DEFERRED_JSON" | jq 'length' 2>/dev/null || echo "0")

        cat >> "$REPORT_FILE" << EOF
## Deferred Findings

**Count:** ${DEFERRED_COUNT}

| ID | File | Severity | Category | Description | Filed as |
| -- | ---- | -------- | -------- | ----------- | -------- |
EOF
        echo "$DEFERRED_JSON" | jq -r '.[] | "| \(.id) | `\(.file):\(.line)` | \(.severity) | \(.category) | \(.description) | \(.issueUrl // "pending") |"' 2>/dev/null >> "$REPORT_FILE" || true
        echo "" >> "$REPORT_FILE"
        ;;

    complete)
        OUTCOME="$1"
        TOTAL_ROUNDS="$2"

        cat >> "$REPORT_FILE" << EOF

---

## Summary

**Outcome:** ${OUTCOME}
**Rounds:** ${TOTAL_ROUNDS}
**Completed:** ${TIMESTAMP}
EOF

        # Clean up stale diff and signal files older than 7 days
        DIFF_DIR="${CLAUDE_PROJECT_DIR:-.}/.claude/agent-bus/diffs"
        SIGNAL_DIR="${CLAUDE_PROJECT_DIR:-.}/.claude/agent-bus/signals"
        if [[ -d "$DIFF_DIR" ]]; then
            find "$DIFF_DIR" -name "forge-*.diff" -mtime +7 -delete 2>/dev/null || true
        fi
        if [[ -d "$SIGNAL_DIR" ]]; then
            find "$SIGNAL_DIR" -name "forge-*.json" -mtime +7 -delete 2>/dev/null || true
        fi
        ;;

    *)
        echo "Error: Unknown action '${ACTION}'" >&2
        echo "Usage: forge-report.sh <hash> {round-start|findings|responses|round-summary|deferred|complete}" >&2
        exit 1
        ;;
esac
