#!/bin/bash
# Council Finding Management
# Add, transition, and query findings within a Council session.
#
# Usage:
#   council-finding.sh add <session-id> --severity <level> --category <cat> --description <desc> [options]
#   council-finding.sh resolve <session-id> <finding-id> --status <fixed|deferred|waived|dismissed> [--resolution <text>]
#   council-finding.sh list <session-id> [--status <open|fixed|...>] [--severity <level>]
#   council-finding.sh count <session-id>

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SESSIONS_DIR="$SCRIPT_DIR/sessions"

usage() {
    cat <<'EOF'
Council Finding Management

Commands:
  add       Add a new finding to a session
  resolve   Transition a finding to a resolution state
  list      List findings in a session
  count     Count findings by status and severity

add options:
  --severity <critical|major|minor|nit>
  --category <security|correctness|edge-case|performance|architecture|style|test-coverage|documentation>
  --description <text>
  --file <path>                     File path (relative to repo root)
  --line <number>                   Line number
  --suggestion <text>               Suggested fix
  --source <reviewer>               Reviewer agent name (default: default-reviewer)

resolve options:
  --status <fixed|deferred|waived|dismissed>
  --resolution <text>               How it was resolved

list options:
  --status <open|fixed|deferred|waived|dismissed>
  --severity <critical|major|minor|nit>

Examples:
  council-finding.sh add council-a1b2c3d4 --severity major --category security \
    --description "SQL injection in user query" --file src/db.ts --line 42
  council-finding.sh resolve council-a1b2c3d4 C-001 --status fixed --resolution "Parameterized query"
  council-finding.sh list council-a1b2c3d4 --status open
EOF
    exit 0
}

timestamp() {
    date -u +"%Y-%m-%dT%H:%M:%SZ"
}

next_finding_id() {
    local session_file="$1"
    local count
    count=$(jq '.findings | length' "$session_file")
    printf "C-%03d" $((count + 1))
}

cmd_add() {
    local session_id="$1"
    shift
    local session_file="$SESSIONS_DIR/${session_id}.json"

    if [[ ! -f "$session_file" ]]; then
        echo "Error: Session $session_id not found" >&2
        exit 1
    fi

    local severity="" category="" description="" file="" line="" suggestion="" source="default-reviewer"

    while [[ $# -gt 0 ]]; do
        case "$1" in
            --severity) severity="$2"; shift 2 ;;
            --category) category="$2"; shift 2 ;;
            --description) description="$2"; shift 2 ;;
            --file) file="$2"; shift 2 ;;
            --line) line="$2"; shift 2 ;;
            --suggestion) suggestion="$2"; shift 2 ;;
            --source) source="$2"; shift 2 ;;
            *) echo "Unknown option: $1"; exit 1 ;;
        esac
    done

    if [[ -z "$severity" || -z "$category" || -z "$description" ]]; then
        echo "Error: --severity, --category, and --description are required" >&2
        exit 1
    fi

    # Validate enums
    case "$severity" in
        critical|major|minor|nit) ;;
        *) echo "Error: Invalid severity '$severity'. Must be: critical, major, minor, nit" >&2; exit 1 ;;
    esac
    case "$category" in
        security|correctness|edge-case|performance|architecture|style|test-coverage|documentation) ;;
        *) echo "Error: Invalid category '$category'. Must be: security, correctness, edge-case, performance, architecture, style, test-coverage, documentation" >&2; exit 1 ;;
    esac

    local finding_id
    finding_id=$(next_finding_id "$session_file")
    local now
    now=$(timestamp)

    local finding
    finding=$(jq -n \
        --arg id "$finding_id" \
        --arg severity "$severity" \
        --arg category "$category" \
        --arg description "$description" \
        --arg file "$file" \
        --arg line "$line" \
        --arg suggestion "$suggestion" \
        --arg source "$source" \
        --arg now "$now" \
        '{
            id: $id,
            severity: $severity,
            category: $category,
            description: $description,
            source: $source,
            status: "open",
            createdAt: $now
        }
        + (if $file != "" then {file: $file} else {} end)
        + (if $line != "" then {line: ($line | tonumber)} else {} end)
        + (if $suggestion != "" then {suggestion: $suggestion} else {} end)')

    local updated
    updated=$(jq \
        --argjson finding "$finding" \
        --arg now "$now" \
        '.findings += [$finding]
         | .updatedAt = $now
         | .events += [{
             type: "finding_added",
             timestamp: $now,
             detail: ("Added finding \($finding.id): \($finding.description | .[0:80])"),
             data: {findingId: $finding.id}
           }]' \
        "$session_file")

    echo "$updated" > "${session_file}.tmp" && mv "${session_file}.tmp" "$session_file"
    echo "$finding_id"
}

cmd_resolve() {
    local session_id="$1"
    local finding_id="$2"
    shift 2

    local session_file="$SESSIONS_DIR/${session_id}.json"
    if [[ ! -f "$session_file" ]]; then
        echo "Error: Session $session_id not found" >&2
        exit 1
    fi

    local new_status="" resolution=""

    while [[ $# -gt 0 ]]; do
        case "$1" in
            --status) new_status="$2"; shift 2 ;;
            --resolution) resolution="$2"; shift 2 ;;
            *) echo "Unknown option: $1"; exit 1 ;;
        esac
    done

    if [[ -z "$new_status" ]]; then
        echo "Error: --status is required" >&2
        exit 1
    fi

    # Validate status enum
    case "$new_status" in
        fixed|deferred|waived|dismissed) ;;
        *) echo "Error: Invalid status '$new_status'. Must be: fixed, deferred, waived, dismissed" >&2; exit 1 ;;
    esac

    # Validate state transition
    local current_status
    current_status=$(jq -r --arg fid "$finding_id" '.findings[] | select(.id == $fid) | .status' "$session_file")

    if [[ -z "$current_status" ]]; then
        echo "Error: Finding $finding_id not found" >&2
        exit 1
    fi

    if [[ "$current_status" != "open" ]]; then
        echo "Error: Finding $finding_id is already $current_status (can only resolve open findings)" >&2
        exit 1
    fi

    # Severity check: critical/major cannot be dismissed
    local severity
    severity=$(jq -r --arg fid "$finding_id" '.findings[] | select(.id == $fid) | .severity' "$session_file")
    if [[ "$new_status" == "dismissed" && ("$severity" == "critical" || "$severity" == "major") ]]; then
        echo "Error: Cannot dismiss $severity findings — fix, defer, or waive instead" >&2
        exit 1
    fi

    local now
    now=$(timestamp)

    local updated
    updated=$(jq \
        --arg fid "$finding_id" \
        --arg status "$new_status" \
        --arg resolution "$resolution" \
        --arg now "$now" \
        '(.findings[] | select(.id == $fid)) |=
            (.status = $status
             | .resolvedAt = $now
             | if $resolution != "" then .resolution = $resolution else . end)
         | .updatedAt = $now
         | .events += [{
             type: "finding_resolved",
             timestamp: $now,
             detail: ("\($fid) → \($status)"),
             data: {findingId: $fid, status: $status}
           }]
         | if $status == "waived" then
             .waivers += [{
               findingId: $fid,
               reason: (if $resolution != "" then $resolution else "No reason provided" end),
               acceptedBy: "user",
               createdAt: $now
             }]
             | .events += [{
                 type: "waiver_added",
                 timestamp: $now,
                 detail: ("Waiver added for \($fid)"),
                 data: {findingId: $fid}
               }]
           else . end' \
        "$session_file")

    echo "$updated" > "${session_file}.tmp" && mv "${session_file}.tmp" "$session_file"
    echo "Resolved: $finding_id → $new_status"
}

cmd_list() {
    local session_id="$1"
    shift
    local session_file="$SESSIONS_DIR/${session_id}.json"

    if [[ ! -f "$session_file" ]]; then
        echo "Error: Session $session_id not found" >&2
        exit 1
    fi

    local filter_status="" filter_severity=""
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --status) filter_status="$2"; shift 2 ;;
            --severity) filter_severity="$2"; shift 2 ;;
            *) echo "Unknown option: $1"; exit 1 ;;
        esac
    done

    jq \
        --arg fs "$filter_status" \
        --arg fv "$filter_severity" \
        '[.findings[]
          | select(if $fs != "" then .status == $fs else true end)
          | select(if $fv != "" then .severity == $fv else true end)
        ]' "$session_file"
}

cmd_count() {
    local session_id="$1"
    local session_file="$SESSIONS_DIR/${session_id}.json"

    if [[ ! -f "$session_file" ]]; then
        echo "Error: Session $session_id not found" >&2
        exit 1
    fi

    jq '{
        byStatus: {
            total: (.findings | length),
            open: ([.findings[] | select(.status == "open")] | length),
            fixed: ([.findings[] | select(.status == "fixed")] | length),
            deferred: ([.findings[] | select(.status == "deferred")] | length),
            waived: ([.findings[] | select(.status == "waived")] | length),
            dismissed: ([.findings[] | select(.status == "dismissed")] | length)
        },
        bySeverity: {
            critical: ([.findings[] | select(.severity == "critical")] | length),
            major: ([.findings[] | select(.severity == "major")] | length),
            minor: ([.findings[] | select(.severity == "minor")] | length),
            nit: ([.findings[] | select(.severity == "nit")] | length)
        }
    }' "$session_file"
}

# Main dispatch
case "${1:-}" in
    add) shift; cmd_add "$@" ;;
    resolve) shift; cmd_resolve "$@" ;;
    list) shift; cmd_list "$@" ;;
    count) shift; cmd_count "$@" ;;
    -h|--help|"") usage ;;
    *) echo "Unknown command: $1"; usage ;;
esac
