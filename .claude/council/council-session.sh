#!/bin/bash
# Council Session Management
# Create, resume, query, and close Council review sessions.
#
# Usage:
#   council-session.sh init --mode <streaming|batch> --target <type> [--pack <quick|standard|full>]
#   council-session.sh resume <session-id>
#   council-session.sh status [session-id]
#   council-session.sh close <session-id> [--status <converged|abandoned>]
#   council-session.sh list [--active]
#
# Session files are stored in .claude/council/sessions/<session-id>.json

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SESSIONS_DIR="$SCRIPT_DIR/sessions"
mkdir -p "$SESSIONS_DIR"

usage() {
    cat <<'EOF'
Council Session Management

Commands:
  init      Create a new review session
  resume    Resume an existing session
  status    Show session status (latest if no ID given)
  close     Close a session
  list      List sessions
  escalate  Upgrade a session's reviewer pack
  add-reviewer  Register a reviewer in the session

init options:
  --mode <streaming|batch>           Review mode (default: streaming)
  --target <worktree|staged|branch|files|commit>  Target type (default: worktree)
  --pack <quick|standard|full>       Reviewer pack (default: quick)
  --branch <name>                    Branch name (for branch targets)
  --base <name>                      Base branch for comparison
  --files <f1,f2,...>                Specific files (for files targets)
  --commit <sha>                     Commit SHA (for commit targets, auto-sets target)

list options:
  --active                           Show only active/paused sessions

escalate options:
  --pack <quick|standard|full>       Target pack (default: auto-upgrade to next level)

add-reviewer options:
  --name <reviewer>                  Reviewer identifier (required)
  --completed                        Mark reviewer as completed (default: started)

Examples:
  council-session.sh init --mode streaming --target staged --pack quick
  council-session.sh status
  council-session.sh close council-a1b2c3d4 --status converged
EOF
    exit 0
}

generate_id() {
    echo "council-$(head -c 4 /dev/urandom | od -An -tx1 | tr -d ' \n')"
}

timestamp() {
    date -u +"%Y-%m-%dT%H:%M:%SZ"
}

get_diff_stats() {
    local target_type="${1:-worktree}"
    local branch="${2:-}"
    local base="${3:-}"
    local commit="${4:-}"

    case "$target_type" in
        staged)
            git diff --cached --stat 2>/dev/null | tail -1 | sed 's/^ //' || echo "0 files changed"
            ;;
        worktree)
            git diff --stat 2>/dev/null | tail -1 | sed 's/^ //' || echo "0 files changed"
            ;;
        branch)
            if [[ -n "$base" ]]; then
                git diff "$base"..."${branch:-HEAD}" --stat 2>/dev/null | tail -1 | sed 's/^ //' || echo "0 files changed"
            else
                git diff main...HEAD --stat 2>/dev/null | tail -1 | sed 's/^ //' || echo "0 files changed"
            fi
            ;;
        commit)
            if [[ -n "$commit" ]]; then
                git show --stat "$commit" 2>/dev/null | tail -1 | sed 's/^ //' || echo "0 files changed"
            else
                echo "0 files changed"
            fi
            ;;
        *)
            echo "0 files changed"
            ;;
    esac
}

parse_diff_stats() {
    local stats_line="$1"
    local files_changed=0
    local insertions=0
    local deletions=0

    files_changed=$(echo "$stats_line" | grep -oP '\d+(?= files? changed)' || echo 0)
    insertions=$(echo "$stats_line" | grep -oP '\d+(?= insertions?)' || echo 0)
    deletions=$(echo "$stats_line" | grep -oP '\d+(?= deletions?)' || echo 0)

    echo "{\"filesChanged\":$files_changed,\"insertions\":$insertions,\"deletions\":$deletions}"
}

cmd_init() {
    local mode="streaming"
    local target_type="worktree"
    local pack="quick"
    local branch=""
    local base=""
    local files=""
    local commit=""

    while [[ $# -gt 0 ]]; do
        case "$1" in
            --mode) mode="$2"; shift 2 ;;
            --target) target_type="$2"; shift 2 ;;
            --pack) pack="$2"; shift 2 ;;
            --branch) branch="$2"; shift 2 ;;
            --base) base="$2"; shift 2 ;;
            --files) files="$2"; shift 2 ;;
            --commit) commit="$2"; shift 2 ;;
            *) echo "Unknown option: $1"; exit 1 ;;
        esac
    done

    # Validate enums
    case "$mode" in
        streaming|batch) ;;
        *) echo "Error: Invalid mode '$mode'. Must be: streaming, batch" >&2; exit 1 ;;
    esac
    case "$target_type" in
        worktree|staged|branch|files|commit) ;;
        *) echo "Error: Invalid target '$target_type'. Must be: worktree, staged, branch, files, commit" >&2; exit 1 ;;
    esac
    case "$pack" in
        quick|standard|full) ;;
        *) echo "Error: Invalid pack '$pack'. Must be: quick, standard, full" >&2; exit 1 ;;
    esac

    local session_id
    session_id=$(generate_id)
    local now
    now=$(timestamp)

    # Auto-set target type if --commit provided
    if [[ -n "$commit" ]]; then
        target_type="commit"
    fi

    # Validate commit SHA exists when target is commit
    if [[ "$target_type" == "commit" ]]; then
        if [[ -z "$commit" ]]; then
            echo "Error: --commit <sha> is required for commit targets" >&2
            exit 1
        fi
        if ! git rev-parse --verify "$commit" > /dev/null 2>&1; then
            echo "Error: Commit '$commit' not found" >&2
            exit 1
        fi
    fi

    # Build target object
    local target_json
    target_json=$(jq -n \
        --arg type "$target_type" \
        --arg branch "$branch" \
        --arg base "$base" \
        --arg files "$files" \
        --arg commit "$commit" \
        '{type: $type}
         + (if $branch != "" then {branch: $branch} else {} end)
         + (if $base != "" then {baseBranch: $base} else {} end)
         + (if $files != "" then {files: ($files | split(","))} else {} end)
         + (if $type == "commit" and $commit != "" then {commit: $commit} else {} end)')
    # Build session
    local session
    session=$(jq -n \
        --arg id "$session_id" \
        --arg mode "$mode" \
        --arg pack "$pack" \
        --arg now "$now" \
        --argjson target "$target_json" \
        '{
            id: $id,
            mode: $mode,
            status: "active",
            target: $target,
            reviewerPack: $pack,
            reviewers: [],
            findings: [],
            evidence: [],
            waivers: [],
            events: [{
                type: "session_created",
                timestamp: $now,
                detail: ("Council \($mode) session created with \($pack) pack")
            }],
            createdAt: $now,
            updatedAt: $now
        }')

    echo "$session" | jq . > "$SESSIONS_DIR/${session_id}.json"
    echo "$session_id"
}

cmd_resume() {
    local session_id="$1"
    local session_file="$SESSIONS_DIR/${session_id}.json"

    if [[ ! -f "$session_file" ]]; then
        echo "Error: Session $session_id not found" >&2
        exit 1
    fi

    local now
    now=$(timestamp)
    local updated
    updated=$(jq \
        --arg now "$now" \
        '.status = "active"
         | .updatedAt = $now
         | .events += [{type: "session_resumed", timestamp: $now, detail: "Session resumed"}]' \
        "$session_file")

    echo "$updated" > "$session_file"
    echo "Resumed: $session_id"
}

cmd_status() {
    local session_id="${1:-}"

    if [[ -z "$session_id" ]]; then
        # Find most recent active session
        local latest
        latest=$(ls -t "$SESSIONS_DIR"/*.json 2>/dev/null | head -1)
        if [[ -z "$latest" ]]; then
            echo "No sessions found"
            exit 0
        fi
        session_id=$(jq -r '.id' "$latest")
    fi

    local session_file="$SESSIONS_DIR/${session_id}.json"
    if [[ ! -f "$session_file" ]]; then
        echo "Error: Session $session_id not found" >&2
        exit 1
    fi

    jq '{
        id,
        mode,
        status,
        target: .target.type,
        reviewerPack,
        reviewers,
        findingCounts: {
            total: (.findings | length),
            open: ([.findings[] | select(.status == "open")] | length),
            fixed: ([.findings[] | select(.status == "fixed")] | length),
            deferred: ([.findings[] | select(.status == "deferred")] | length),
            waived: ([.findings[] | select(.status == "waived")] | length),
            dismissed: ([.findings[] | select(.status == "dismissed")] | length)
        },
        evidenceCount: (.evidence | length),
        waiverCount: (.waivers | length),
        createdAt,
        updatedAt
    }' "$session_file"
}

cmd_close() {
    local session_id="$1"
    shift
    local close_status="converged"

    while [[ $# -gt 0 ]]; do
        case "$1" in
            --status) close_status="$2"; shift 2 ;;
            *) echo "Unknown option: $1"; exit 1 ;;
        esac
    done

    local session_file="$SESSIONS_DIR/${session_id}.json"
    if [[ ! -f "$session_file" ]]; then
        echo "Error: Session $session_id not found" >&2
        exit 1
    fi

    # Validate convergence prerequisites
    local total_count=0 open_count=0
    if [[ "$close_status" == "converged" ]]; then
        local counts
        if ! counts=$(jq -r '[([.findings[] | select(.status == "open")] | length), (.findings | length)] | join(" ")' "$session_file"); then
            echo "Error: Failed to read findings from session file: $session_file" >&2
            exit 1
        fi
        if ! [[ "$counts" =~ ^[0-9]+[[:space:]][0-9]+$ ]]; then
            echo "Error: Unexpected findings counts format in $session_file: '$counts'" >&2
            exit 1
        fi
        read -r open_count total_count <<< "$counts"

        if [[ "$open_count" -gt 0 ]]; then
            echo "Error: Cannot converge with $open_count open finding(s). Resolve all findings first." >&2
            exit 1
        fi
    fi

    local now
    now=$(timestamp)
    local event_detail=""
    if [[ "$close_status" == "converged" ]]; then
        if [[ "$total_count" -eq 0 ]]; then
            event_detail="Session converged with no findings reviewed"
        else
            event_detail="Session converged"
        fi
    fi

    local updated
    updated=$(jq \
        --arg status "$close_status" \
        --arg now "$now" \
        --arg event_detail "$event_detail" \
        '.status = $status
         | .updatedAt = $now
         | if $status == "converged" then
             .convergedAt = $now
             | .events += [{type: "converged", timestamp: $now, detail: $event_detail}]
           elif $status == "abandoned" then
             .events += [{type: "session_abandoned", timestamp: $now, detail: "Session abandoned"}]
           else
             .events += [{type: "session_closed", timestamp: $now, detail: ("Session closed as " + $status)}]
           end' \
        "$session_file")

    echo "$updated" > "$session_file"
    echo "Closed: $session_id ($close_status)"
}

cmd_list() {
    local active_only=false
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --active) active_only=true; shift ;;
            *) echo "Unknown option: $1"; exit 1 ;;
        esac
    done

    local sessions_found=false
    for f in "$SESSIONS_DIR"/*.json; do
        [[ -f "$f" ]] || continue
        sessions_found=true

        if $active_only; then
            local status
            status=$(jq -r '.status' "$f")
            [[ "$status" == "active" || "$status" == "paused" ]] || continue
        fi

        jq -r '[.id, .mode, .status, .reviewerPack, (.findings | length | tostring) + " findings", .createdAt] | join("  ")' "$f"
    done

    if ! $sessions_found; then
        echo "No sessions found"
    fi
}

cmd_escalate() {
    local session_id="$1"
    shift
    local new_pack=""

    while [[ $# -gt 0 ]]; do
        case "$1" in
            --pack) new_pack="$2"; shift 2 ;;
            *) echo "Unknown option: $1"; exit 1 ;;
        esac
    done

    local session_file="$SESSIONS_DIR/${session_id}.json"
    if [[ ! -f "$session_file" ]]; then
        echo "Error: Session $session_id not found" >&2
        exit 1
    fi

    local current_status
    current_status=$(jq -r '.status' "$session_file")
    if [[ "$current_status" != "active" && "$current_status" != "paused" ]]; then
        echo "Error: Cannot escalate a $current_status session (must be active or paused)" >&2
        exit 1
    fi

    local current_pack
    current_pack=$(jq -r '.reviewerPack' "$session_file")

    # Determine target pack: explicit or auto-upgrade
    if [[ -z "$new_pack" ]]; then
        case "$current_pack" in
            quick) new_pack="standard" ;;
            standard) new_pack="full" ;;
            full) echo "Already at full pack"; exit 0 ;;
        esac
    fi

    # Validate pack enum
    case "$new_pack" in
        quick|standard|full) ;;
        *) echo "Error: Invalid pack '$new_pack'. Must be: quick, standard, full" >&2; exit 1 ;;
    esac

    if [[ "$new_pack" == "$current_pack" ]]; then
        echo "Already at $current_pack pack"
        exit 0
    fi

    # Reject downgrades
    case "$current_pack:$new_pack" in
        quick:standard|quick:full|standard:full) ;; # upgrade allowed
        *)
            echo "Error: Cannot escalate from $current_pack to $new_pack (must be an upgrade)" >&2
            exit 1
            ;;
    esac

    local now
    now=$(timestamp)

    # Upgrade existing session in place and record escalation event
    local updated
    updated=$(jq \
        --arg pack "$new_pack" \
        --arg now "$now" \
        --arg old_pack "$current_pack" \
        '.reviewerPack = $pack
         | .updatedAt = $now
         | .events += [{
             type: "escalated",
             timestamp: $now,
             detail: ("Escalated from \($old_pack) to \($pack)"),
             data: {fromPack: $old_pack, toPack: $pack}
           }]' \
        "$session_file")

    echo "$updated" > "$session_file"
    echo "Escalated: $session_id ($current_pack → $new_pack)"
}

cmd_add_reviewer() {
    local session_id="$1"
    shift
    local reviewer="" event_type="reviewer_started"

    while [[ $# -gt 0 ]]; do
        case "$1" in
            --name) reviewer="$2"; shift 2 ;;
            --completed) event_type="reviewer_completed"; shift ;;
            *) echo "Unknown option: $1"; exit 1 ;;
        esac
    done

    if [[ -z "$reviewer" ]]; then
        echo "Error: --name is required" >&2
        exit 1
    fi

    local session_file="$SESSIONS_DIR/${session_id}.json"
    if [[ ! -f "$session_file" ]]; then
        echo "Error: Session $session_id not found" >&2
        exit 1
    fi

    local now
    now=$(timestamp)

    local updated
    if [[ "$event_type" == "reviewer_started" ]]; then
        # Add to reviewers list (deduplicated) and emit event
        updated=$(jq \
            --arg reviewer "$reviewer" \
            --arg now "$now" \
            'if (.reviewers | index($reviewer)) then . else .reviewers += [$reviewer] end
             | .updatedAt = $now
             | .events += [{
                 type: "reviewer_started",
                 timestamp: $now,
                 detail: ("Reviewer \($reviewer) started"),
                 data: {reviewer: $reviewer}
               }]' \
            "$session_file")
    else
        # Add to reviewers list (deduplicated) and emit completed event
        updated=$(jq \
            --arg reviewer "$reviewer" \
            --arg now "$now" \
            'if (.reviewers | index($reviewer)) then . else .reviewers += [$reviewer] end
             | .updatedAt = $now
             | .events += [{
                 type: "reviewer_completed",
                 timestamp: $now,
                 detail: ("Reviewer \($reviewer) completed"),
                 data: {reviewer: $reviewer}
               }]' \
            "$session_file")
    fi

    echo "$updated" > "$session_file"
    echo "$event_type: $reviewer"
}

# Main dispatch
case "${1:-}" in
    init) shift; cmd_init "$@" ;;
    resume) shift; cmd_resume "$@" ;;
    status) shift; cmd_status "${1:-}" ;;
    close) shift; cmd_close "$@" ;;
    list) shift; cmd_list "$@" ;;
    escalate) shift; cmd_escalate "$@" ;;
    add-reviewer) shift; cmd_add_reviewer "$@" ;;
    -h|--help|"") usage ;;
    *) echo "Unknown command: $1"; usage ;;
esac
