#!/bin/bash
# Council Publication
# Generate publication summaries from Council session state.
#
# Usage:
#   council-publish.sh <session-id> [--format <markdown|json>] [--output <file>]
#   council-publish.sh <session-id> --pr           Generate PR body markdown
#   council-publish.sh <session-id> --commit       Generate commit trailer

set -euo pipefail

_tmpfiles=()
_cleanup() { for f in "${_tmpfiles[@]}"; do rm -f "$f"; done; }
trap _cleanup EXIT

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SESSIONS_DIR="$SCRIPT_DIR/sessions"

usage() {
    cat <<'EOF'
Council Publication

Generate publication summaries from Council session state.

Commands:
  <session-id>                       Generate full summary (default: markdown)
  <session-id> --pr                  Generate PR body markdown
  <session-id> --commit              Generate commit trailer

Options:
  --format <markdown|json>           Output format (default: markdown)
  --output <file>                    Write to file instead of stdout

Examples:
  council-publish.sh council-a1b2c3d4
  council-publish.sh council-a1b2c3d4 --pr
  council-publish.sh council-a1b2c3d4 --commit
  council-publish.sh council-a1b2c3d4 --format json --output review-summary.json
EOF
    exit 0
}

timestamp() {
    date -u +"%Y-%m-%dT%H:%M:%SZ"
}

generate_markdown() {
    local session_file="$1"

    local id mode pack status
    id=$(jq -r '.id' "$session_file")
    mode=$(jq -r '.mode' "$session_file")
    pack=$(jq -r '.reviewerPack' "$session_file")
    status=$(jq -r '.status' "$session_file")

    local target_type
    target_type=$(jq -r '.target.type' "$session_file")

    local total open fixed deferred waived dismissed
    total=$(jq '.findings | length' "$session_file")
    open=$(jq '[.findings[] | select(.status == "open")] | length' "$session_file")
    fixed=$(jq '[.findings[] | select(.status == "fixed")] | length' "$session_file")
    deferred=$(jq '[.findings[] | select(.status == "deferred")] | length' "$session_file")
    waived=$(jq '[.findings[] | select(.status == "waived")] | length' "$session_file")
    dismissed=$(jq '[.findings[] | select(.status == "dismissed")] | length' "$session_file")

    local critical major minor nit
    critical=$(jq '[.findings[] | select(.severity == "critical")] | length' "$session_file")
    major=$(jq '[.findings[] | select(.severity == "major")] | length' "$session_file")
    minor=$(jq '[.findings[] | select(.severity == "minor")] | length' "$session_file")
    nit=$(jq '[.findings[] | select(.severity == "nit")] | length' "$session_file")

    local evidence_count waiver_count
    evidence_count=$(jq '.evidence | length' "$session_file")
    waiver_count=$(jq '.waivers | length' "$session_file")

    # Determine verdict
    local verdict="clean"
    if [[ "$total" -eq 0 ]]; then
        verdict="no-findings"
    elif [[ "$open" -gt 0 ]]; then
        verdict="open-findings"
    elif [[ "$waived" -gt 0 ]]; then
        verdict="clean-with-waivers"
    elif [[ "$status" != "converged" && "$status" != "published" ]]; then
        verdict="not-converged"
    fi

    local verdict_icon
    case "$verdict" in
        clean) verdict_icon="PASS" ;;
        clean-with-waivers) verdict_icon="PASS (waivers)" ;;
        open-findings) verdict_icon="OPEN" ;;
        no-findings) verdict_icon="PASS (no findings)" ;;
        not-converged) verdict_icon="IN PROGRESS" ;;
    esac

    cat <<EOF
## Council Review Summary

| | |
|---|---|
| **Session** | \`$id\` |
| **Mode** | $mode |
| **Pack** | $pack |
| **Target** | $target_type |
| **Verdict** | **$verdict_icon** |

### Findings

| Status | Count |
|--------|-------|
| Total | $total |
| Fixed | $fixed |
| Deferred | $deferred |
| Waived | $waived |
| Dismissed | $dismissed |
| Open | $open |

| Severity | Count |
|----------|-------|
| Critical | $critical |
| Major | $major |
| Minor | $minor |
| Nit | $nit |
EOF

    # Notable waivers
    if [[ "$waiver_count" -gt 0 ]]; then
        echo ""
        echo "### Waivers"
        echo ""
        jq -r '.waivers[] | "- **\(.findingId)**: \(.reason) _(accepted by \(.acceptedBy))_"' "$session_file"
    fi

    # Evidence summary
    if [[ "$evidence_count" -gt 0 ]]; then
        echo ""
        echo "### Evidence"
        echo ""
        jq -r '.evidence[] | "- [\(.result // "n/a")] \(.description)" + (if .command then " (`" + .command + "`)" else "" end)' "$session_file"
    fi

    # Open findings detail
    if [[ "$open" -gt 0 ]]; then
        echo ""
        echo "### Open Findings"
        echo ""
        jq -r '.findings[] | select(.status == "open") | "- **\(.id)** [\(.severity)/\(.category)]: \(.description)" + (if .file then " (\(.file):\(.line // "?"))" else "" end)' "$session_file"
    fi

    echo ""
    echo "---"
    echo "_Reviewed by Council ($mode/$pack)_"
}

generate_pr_body() {
    local session_file="$1"
    generate_markdown "$session_file"
}

generate_commit_trailer() {
    local session_file="$1"

    local id mode verdict
    id=$(jq -r '.id' "$session_file")
    mode=$(jq -r '.mode' "$session_file")

    local open
    open=$(jq '[.findings[] | select(.status == "open")] | length' "$session_file")
    local total
    total=$(jq '.findings | length' "$session_file")

    if [[ "$open" -eq 0 && "$total" -gt 0 ]]; then
        verdict="clean"
    elif [[ "$open" -gt 0 ]]; then
        verdict="open($open)"
    else
        verdict="no-findings"
    fi

    echo "Council-Review: $id ($mode, $verdict)"
}

generate_json_summary() {
    local session_file="$1"

    local now
    now=$(timestamp)

    jq --arg now "$now" '{
        sessionId: .id,
        mode: .mode,
        reviewerPack: .reviewerPack,
        reviewers: .reviewers,
        target: .target.type,
        findingCounts: {
            total: (.findings | length),
            fixed: ([.findings[] | select(.status == "fixed")] | length),
            deferred: ([.findings[] | select(.status == "deferred")] | length),
            waived: ([.findings[] | select(.status == "waived")] | length),
            dismissed: ([.findings[] | select(.status == "dismissed")] | length),
            open: ([.findings[] | select(.status == "open")] | length)
        },
        severityCounts: {
            critical: ([.findings[] | select(.severity == "critical")] | length),
            major: ([.findings[] | select(.severity == "major")] | length),
            minor: ([.findings[] | select(.severity == "minor")] | length),
            nit: ([.findings[] | select(.severity == "nit")] | length)
        },
        notableWaivers: [.waivers[] | "\(.findingId): \(.reason)"],
        evidenceSummary: [.evidence[] | "[\(.result // "n/a")] \(.description)"],
        verdict: (
            if (.status != "converged" and .status != "published") then "not-converged"
            elif (.findings | length) == 0 then "no-findings"
            elif ([.findings[] | select(.status == "open")] | length) > 0 then "open-findings"
            elif ([.findings[] | select(.status == "waived")] | length) > 0 then "clean-with-waivers"
            else "clean"
            end
        ),
        generatedAt: $now
    }' "$session_file"
}

# Parse arguments
session_id="${1:-}"
if [[ -z "$session_id" || "$session_id" == "-h" || "$session_id" == "--help" ]]; then
    usage
fi
shift

format="markdown"
output=""
pr_mode=false
commit_mode=false

while [[ $# -gt 0 ]]; do
    case "$1" in
        --format) format="$2"; shift 2 ;;
        --output) output="$2"; shift 2 ;;
        --pr) pr_mode=true; shift ;;
        --commit) commit_mode=true; shift ;;
        *) echo "Unknown option: $1"; exit 1 ;;
    esac
done

session_file="$SESSIONS_DIR/${session_id}.json"
if [[ ! -f "$session_file" ]]; then
    echo "Error: Session $session_id not found" >&2
    exit 1
fi

# Generate output
result=""
if $commit_mode; then
    result=$(generate_commit_trailer "$session_file")
elif $pr_mode; then
    result=$(generate_pr_body "$session_file")
elif [[ "$format" == "json" ]]; then
    result=$(generate_json_summary "$session_file")
else
    result=$(generate_markdown "$session_file")
fi

# Generate the structured summary for session persistence (always JSON, regardless of output format)
now=$(timestamp)
summary_json=$(generate_json_summary "$session_file")

_tmpfiles+=("${session_file}.tmp")
# Persist summary into session state, transition status to published, and add event
jq --arg now "$now" --argjson summary "$summary_json" \
    '.summary = $summary
     | .status = "published"
     | .updatedAt = $now
     | .events += [{type: "published", timestamp: $now, detail: "Publication summary generated and persisted"}]' \
    "$session_file" > "${session_file}.tmp" && mv "${session_file}.tmp" "$session_file"

# Output
if [[ -n "$output" ]]; then
    echo "$result" > "$output"
    echo "Written to: $output"
else
    echo "$result"
fi
