#!/bin/bash
# forge-defer.sh — File deferred findings as GitHub Issues or APS work items
#
# Usage:
#   forge-defer.sh file <finding-json>           File a single deferred finding
#   forge-defer.sh batch <findings-json-array>   File multiple deferred findings
#   forge-defer.sh check-dup <file> <desc>       Check for existing duplicate issue
#
# Requires: gh CLI authenticated
#
# Config:
#   FORGE_SOURCE — Source context (e.g. "PR #354, forge round 2")
#   FORGE_HASH   — Forge session hash for traceability

set -e

ACTION="$1"
shift

# --- Helpers ---

check_gh_repo() {
    # Verify gh CLI can resolve the current repo (lightweight connectivity check)
    if ! gh repo view --json name -q '.name' >/dev/null 2>&1; then
        echo "Error: Could not resolve repo. Is gh authenticated and in a git repo?" >&2
        return 1
    fi
}

detect_aps_context() {
    # Check if current branch or recent commit references an APS module
    local branch_name
    branch_name=$(git branch --show-current 2>/dev/null || echo "")
    local commit_msg
    commit_msg=$(git log -1 --format="%s" 2>/dev/null || echo "")

    # Look for APS module references like FORGE-001, FNEG-002, etc.
    local aps_ref=""
    if [[ "$branch_name" =~ ([A-Z]{2,6})-([0-9]{3}) ]]; then
        aps_ref="${BASH_REMATCH[1]}-${BASH_REMATCH[2]}"
    elif [[ "$commit_msg" =~ ([A-Z]{2,6})-([0-9]{3}) ]]; then
        aps_ref="${BASH_REMATCH[1]}-${BASH_REMATCH[2]}"
    fi

    echo "$aps_ref"
}

check_duplicate() {
    local file="$1"
    local description="$2"

    # Search for existing open issues with forge:deferred label matching both
    # file path and description to avoid false duplicates across findings in
    # the same file. Uses jq --arg for safe interpolation (no shell injection).
    local existing
    existing=$(gh issue list --label "forge:deferred" --state open --json title,number 2>/dev/null \
        | jq -r --arg file "$file" --arg desc "$description" \
        '.[] | select(.title | (contains($file) and contains($desc))) | .number' 2>/dev/null | head -1)

    echo "$existing"
}

file_github_issue() {
    local finding_json="$1"

    # Parse finding fields
    local id file line severity category description suggestion reasoning
    id=$(echo "$finding_json" | jq -r '.id // "unknown"')
    file=$(echo "$finding_json" | jq -r '.file // "unknown"')
    line=$(echo "$finding_json" | jq -r '.line // 0')
    severity=$(echo "$finding_json" | jq -r '.severity // "minor"')
    category=$(echo "$finding_json" | jq -r '.category // "uncategorized"')
    description=$(echo "$finding_json" | jq -r '.description // "No description"')
    suggestion=$(echo "$finding_json" | jq -r '.suggestion // "No suggestion"')
    reasoning=$(echo "$finding_json" | jq -r '.reasoning // "Auto-deferred"')

    # Map category to area label
    local area_label="area:${category}"

    # Check for duplicates
    local existing_issue
    existing_issue=$(check_duplicate "$file" "$description")

    if [[ -n "$existing_issue" ]]; then
        # Update existing issue with a comment instead of creating duplicate
        gh issue comment "$existing_issue" --body "$(cat <<COMMENT_EOF
**Forge re-detected this finding**

- **Source:** ${FORGE_SOURCE:-"forge session ${FORGE_HASH:-unknown}"}
- **Severity:** ${severity}
- **File:** \`${file}:${line}\`
- **Finding ID:** ${id}

The same or similar issue was flagged again during a Forge review session.
COMMENT_EOF
)" 2>/dev/null

        jq -n --arg id "$id" --arg num "$existing_issue" \
            '{findingId: $id, action: "duplicate", issueNumber: ($num | tonumber)}'
        return 0
    fi

    # Create new issue
    local issue_url
    issue_url=$(gh issue create \
        --title "[forge] ${description}" \
        --label "forge:deferred" \
        --label "${area_label}" \
        --body "$(cat <<BODY_EOF
## Deferred Finding

| Field | Value |
| ----- | ----- |
| **Source** | ${FORGE_SOURCE:-"forge session ${FORGE_HASH:-unknown}"} |
| **File** | \`${file}:${line}\` |
| **Severity** | ${severity} |
| **Category** | ${category} |
| **Finding ID** | ${id} |

### Description

${description}

### Suggested Fix

${suggestion}

### Deferral Reasoning

${reasoning}

---

*Filed automatically by the Forge pre-commit review pipeline.*
*Forge session: \`${FORGE_HASH:-unknown}\`*
BODY_EOF
)" 2>/dev/null)

    if [[ -n "$issue_url" ]]; then
        local issue_number
        issue_number=$(echo "$issue_url" | grep -oE '[0-9]+$')
        jq -n --arg id "$id" --arg url "$issue_url" --arg num "${issue_number:-0}" \
            '{findingId: $id, action: "filed", issueUrl: $url, issueNumber: ($num | tonumber)}'
    else
        jq -n --arg id "$id" '{findingId: $id, action: "error", error: "Failed to create issue"}' >&2
        return 1
    fi
}

file_aps_issue() {
    local finding_json="$1"
    local aps_ref="$2"

    # Extract the module ID from the APS reference (e.g. FORGE from FORGE-001)
    local module_id
    module_id=$(echo "$aps_ref" | cut -d'-' -f1)

    # Strict validation: module_id must be 2-6 uppercase letters only.
    # Prevents glob injection in the find command below.
    if [[ ! "$module_id" =~ ^[A-Z]{2,6}$ ]]; then
        echo "Error: Invalid module_id '${module_id}' — expected 2-6 uppercase letters" >&2
        file_github_issue "$finding_json"
        return
    fi

    local description severity
    description=$(echo "$finding_json" | jq -r '.description // "No description"')
    severity=$(echo "$finding_json" | jq -r '.severity // "minor"')
    local id
    id=$(echo "$finding_json" | jq -r '.id // "unknown"')

    # Find the module file
    local module_file
    module_file=$(find "${CLAUDE_PROJECT_DIR:-.}/plans/modules/" -name "*${module_id,,}*" -o -name "*${module_id}*" 2>/dev/null | head -1)

    if [[ -z "$module_file" ]]; then
        # Fall back to GitHub issue if APS module not found
        file_github_issue "$finding_json"
        return
    fi

    # Append as a draft work item to the module (flock prevents interleaving
    # when multiple forge-defer.sh processes run concurrently).
    # Lock file is kept around — flock works on inodes, so deleting it would
    # allow a second process to create a new inode and acquire a separate lock.
    (
        if ! flock -w 5 200 2>/dev/null; then
            echo "Error: failed to acquire lock on ${module_file}.lock" >&2
            return 1
        fi
        cat >> "$module_file" << APS_EOF

### ${module_id}-DEFER: ${description}

- **Intent:** Address deferred finding from Forge review
- **Severity:** ${severity}
- **Source:** Forge session ${FORGE_HASH:-unknown}, finding ${id}
- **Status:** Draft
- **Confidence:** medium
APS_EOF
    ) 200>"${module_file}.lock"

    jq -n --arg id "$id" --arg mod "$module_id" --arg file "$module_file" \
        '{findingId: $id, action: "aps-filed", module: $mod, file: $file}'
}

# --- Main ---

case "$ACTION" in
    file)
        FINDING_JSON="$1"
        check_gh_repo || exit 1

        APS_REF=$(detect_aps_context)
        if [[ -n "$APS_REF" ]]; then
            file_aps_issue "$FINDING_JSON" "$APS_REF"
        else
            file_github_issue "$FINDING_JSON"
        fi
        ;;

    batch)
        FINDINGS_JSON="$1"
        check_gh_repo || exit 1

        APS_REF=$(detect_aps_context)
        RESULTS="[]"
        TOTAL=$(echo "$FINDINGS_JSON" | jq 'length')

        for i in $(seq 0 $((TOTAL - 1))); do
            FINDING=$(echo "$FINDINGS_JSON" | jq ".[$i]")

            if [[ -n "$APS_REF" ]]; then
                RESULT=$(file_aps_issue "$FINDING" "$APS_REF")
            else
                RESULT=$(file_github_issue "$FINDING")
            fi

            RESULTS=$(echo "$RESULTS" | jq --argjson result "$RESULT" '. + [$result]')
        done

        echo "$RESULTS"
        ;;

    check-dup)
        FILE="$1"
        DESCRIPTION="$2"
        EXISTING=$(check_duplicate "$FILE" "$DESCRIPTION")
        if [[ -n "$EXISTING" ]]; then
            jq -n --arg num "$EXISTING" '{duplicate: true, issueNumber: ($num | tonumber)}'
        else
            jq -n '{duplicate: false}'
        fi
        ;;

    *)
        echo "Error: Unknown action '${ACTION}'" >&2
        echo "Usage: forge-defer.sh {file|batch|check-dup} [args]" >&2
        exit 1
        ;;
esac
