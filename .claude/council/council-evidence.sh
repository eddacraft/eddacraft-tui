#!/bin/bash
# Council Evidence Management
# Attach validation evidence to Council review sessions.
#
# Usage:
#   council-evidence.sh add <session-id> --type <type> --description <desc> [options]
#   council-evidence.sh list <session-id>
#   council-evidence.sh run <session-id> --command <cmd> --description <desc> [--finding <id>...]

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SESSIONS_DIR="$SCRIPT_DIR/sessions"

usage() {
    cat <<'EOF'
Council Evidence Management

Commands:
  add     Attach evidence to a session
  list    List evidence in a session
  run     Run a command and capture result as evidence

add options:
  --type <test|lint|security-scan|build|manual|command>
  --description <text>
  --result <pass|fail|partial|skipped>
  --output <text>                    Abbreviated output
  --command <cmd>                    Command that produced evidence
  --finding <id>                     Link to finding (repeatable)

run options:
  --command <cmd>                    Command to execute
  --description <text>               What is being validated
  --finding <id>                     Link to finding (repeatable)

Examples:
  council-evidence.sh add council-a1b2c3d4 --type test --description "Unit tests pass" \
    --result pass --command "npm test"
  council-evidence.sh run council-a1b2c3d4 --command "npm test" \
    --description "Unit test suite" --finding C-003
EOF
    exit 0
}

timestamp() {
    date -u +"%Y-%m-%dT%H:%M:%SZ"
}

next_evidence_id() {
    local session_file="$1"
    local count
    count=$(jq '.evidence | length' "$session_file")
    printf "E-%03d" $((count + 1))
}

cmd_add() {
    local session_id="$1"
    shift
    local session_file="$SESSIONS_DIR/${session_id}.json"

    if [[ ! -f "$session_file" ]]; then
        echo "Error: Session $session_id not found" >&2
        exit 1
    fi

    local ev_type="" description="" result="" output="" command="" finding_ids=()

    while [[ $# -gt 0 ]]; do
        case "$1" in
            --type) ev_type="$2"; shift 2 ;;
            --description) description="$2"; shift 2 ;;
            --result) result="$2"; shift 2 ;;
            --output) output="$2"; shift 2 ;;
            --command) command="$2"; shift 2 ;;
            --finding) finding_ids+=("$2"); shift 2 ;;
            *) echo "Unknown option: $1"; exit 1 ;;
        esac
    done

    if [[ -z "$ev_type" || -z "$description" ]]; then
        echo "Error: --type and --description are required" >&2
        exit 1
    fi

    local evidence_id
    evidence_id=$(next_evidence_id "$session_file")
    local now
    now=$(timestamp)

    local findings_json
    findings_json=$(printf '%s\n' "${finding_ids[@]:-}" | jq -R . | jq -s '.')

    local evidence
    evidence=$(jq -n \
        --arg id "$evidence_id" \
        --arg type "$ev_type" \
        --arg description "$description" \
        --arg result "$result" \
        --arg output "$output" \
        --arg command "$command" \
        --argjson findingIds "$findings_json" \
        --arg now "$now" \
        '{
            id: $id,
            type: $type,
            description: $description,
            createdAt: $now
        }
        + (if $result != "" then {result: $result} else {} end)
        + (if $output != "" then {output: $output} else {} end)
        + (if $command != "" then {command: $command} else {} end)
        + (if ($findingIds | length) > 0 and ($findingIds[0] != "") then {findingIds: $findingIds} else {} end)')

    local updated
    updated=$(jq \
        --argjson evidence "$evidence" \
        --arg now "$now" \
        '.evidence += [$evidence]
         | .updatedAt = $now
         | .events += [{
             type: "evidence_added",
             timestamp: $now,
             detail: ("Evidence \($evidence.id): \($evidence.description)"),
             data: {evidenceId: $evidence.id}
           }]' \
        "$session_file")

    echo "$updated" > "${session_file}.tmp" && mv "${session_file}.tmp" "$session_file"
    echo "$evidence_id"
}

cmd_run() {
    local session_id="$1"
    shift
    local session_file="$SESSIONS_DIR/${session_id}.json"

    if [[ ! -f "$session_file" ]]; then
        echo "Error: Session $session_id not found" >&2
        exit 1
    fi

    local command="" description="" finding_ids=()

    while [[ $# -gt 0 ]]; do
        case "$1" in
            --command) command="$2"; shift 2 ;;
            --description) description="$2"; shift 2 ;;
            --finding) finding_ids+=("$2"); shift 2 ;;
            *) echo "Unknown option: $1"; exit 1 ;;
        esac
    done

    if [[ -z "$command" || -z "$description" ]]; then
        echo "Error: --command and --description are required" >&2
        exit 1
    fi

    # Run the command and capture output + exit code
    local output=""
    local exit_code=0
    output=$(bash -c "$command" 2>&1) || exit_code=$?

    local result="pass"
    if [[ $exit_code -ne 0 ]]; then
        result="fail"
    fi

    # Truncate output to 2000 chars
    if [[ ${#output} -gt 2000 ]]; then
        output="${output:0:1997}..."
    fi

    # Build finding args
    local finding_args=()
    for fid in ${finding_ids[@]+"${finding_ids[@]}"}; do
        [[ -n "$fid" ]] && finding_args+=(--finding "$fid")
    done

    cmd_add "$session_id" \
        --type command \
        --description "$description" \
        --result "$result" \
        --command "$command" \
        --output "$output" \
        ${finding_args[@]+"${finding_args[@]}"}
}

cmd_list() {
    local session_id="$1"
    local session_file="$SESSIONS_DIR/${session_id}.json"

    if [[ ! -f "$session_file" ]]; then
        echo "Error: Session $session_id not found" >&2
        exit 1
    fi

    jq '.evidence' "$session_file"
}

# Main dispatch
case "${1:-}" in
    add) shift; cmd_add "$@" ;;
    run) shift; cmd_run "$@" ;;
    list) shift; cmd_list "$@" ;;
    -h|--help|"") usage ;;
    *) echo "Unknown command: $1"; usage ;;
esac
