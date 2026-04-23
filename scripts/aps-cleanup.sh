#!/usr/bin/env bash
# aps-cleanup.sh — APS reconciliation and hygiene
# Runs on schedule via systemd timer. Reconciles module statuses and counts
# against actual work item states, flags stale entries, checks branch hygiene.
#
# Usage: ./aps-cleanup.sh [--repo=<path>] [--dry-run] [--notify]
# Default repo: ~/Projects/src/EddaCraft/anvil-001
#
# Modes:
#   (default)  Report findings only, append to cleanup-log.md
#   --dry-run  Print findings to stdout, don't write anything
#   --notify   Send alerts for items needing human attention

set -euo pipefail

REPO="${REPO:-$HOME/Projects/src/EddaCraft/anvil-001}"
DRY_RUN=false
NOTIFY=false
TIMESTAMP=$(date '+%Y-%m-%d %H:%M %Z')
FINDINGS=""

for arg in "$@"; do
  case $arg in
    --dry-run) DRY_RUN=true ;;
    --notify)  NOTIFY=true ;;
    --repo=*)  REPO="${arg#*=}" ;;
  esac
done

LOG="$REPO/plans/reviews/cleanup-log.md"
MODULES_DIR="$REPO/plans/modules"
ARCHIVE_DIR="$REPO/plans/archive/modules"
POST_MERGE_DIR="$REPO/plans/reviews/post-merge"

log() { echo "[aps-cleanup] $*" >&2; }
finding() { FINDINGS+="- $*\n"; }

cd "$REPO"

# ── 1. Git sync ──────────────────────────────────────────────────────────────
log "Fetching latest from origin..."
git fetch origin --quiet 2>/dev/null || log "Warning: git fetch failed (offline?)"

# ── 2. Module count reconciliation ──────────────────────────────────────────
# For each module, count actual work items and compare against the header.
shopt -s nullglob
log "Reconciling module work item counts..."

count_work_items() {
  local file="$1"
  local prefix="$2"
  local total=0 done=0 in_progress=0 proposed=0 draft=0 ready=0 deferred=0 superseded=0

  # Method 1: Structured work items with Status field
  # Count by grepping status lines near work item headings
  # Use awk to scan from each `### PREFIX-NNN` heading up to (but not
  # including) the next `### PREFIX-` heading, so Status is found no matter
  # how long the item body is. -A5 / fixed-N grep under-reports as soon as
  # an item grows past the window.
  local status_block
  status_block=$(awk -v pfx="$prefix" '
    BEGIN { re="^### " pfx "-[0-9]" }
    $0 ~ re { in_item=1; next }
    /^### [A-Z][A-Z0-9]*-[0-9]/ { in_item=0 }
    /^## / { in_item=0 }
    in_item && /^- \*\*Status\*\*|^- \*\*Status:|^Status:/ { print }
  ' "$file" 2>/dev/null || true)

  if [[ -n "$status_block" ]]; then
    done=$(echo "$status_block" | grep -ciE 'complete|done' || true)
    in_progress=$(echo "$status_block" | grep -ciE 'in.progress' || true)
    proposed=$(echo "$status_block" | grep -ci 'proposed' || true)
    deferred=$(echo "$status_block" | grep -ci 'deferred' || true)
    superseded=$(echo "$status_block" | grep -ci 'superseded' || true)
    draft=$(echo "$status_block" | grep -ciE '^.*draft' || true)
    ready=$(echo "$status_block" | grep -ciE '^.*ready' || true)
    total=$((done + in_progress + proposed + draft + ready + deferred + superseded))
  fi

  # Method 2: Checklist items (- [x] PREFIX-NNN: ...)
  if [[ "$total" -eq 0 ]]; then
    local checked unchecked
    checked=$(grep -cE "^\- \[x\] ${prefix}-[0-9]" "$file" 2>/dev/null || true)
    unchecked=$(grep -cE "^\- \[ \] ${prefix}-[0-9]" "$file" 2>/dev/null || true)
    checked=${checked:-0}
    unchecked=${unchecked:-0}

    if [[ "$((checked + unchecked))" -gt 0 ]]; then
      done=$checked
      draft=$unchecked
      total=$((checked + unchecked))
    fi
  fi

  # Method 3: Table rows with Done/Complete status
  if [[ "$total" -eq 0 ]]; then
    local table_total
    table_total=$(grep -cE "^\| ${prefix}-[0-9]" "$file" 2>/dev/null || true)
    table_total=${table_total:-0}
    if [[ "$table_total" -gt 0 ]]; then
      done=$(grep -cE "^\| ${prefix}-[0-9].*\| (Done|Complete)" "$file" 2>/dev/null || true)
      done=${done:-0}
      total=$table_total
    fi
  fi

  # Method 4: Just count headings (items exist but no status)
  if [[ "$total" -eq 0 ]]; then
    total=$(grep -cE "^### ${prefix}-[0-9]" "$file" 2>/dev/null || true)
    total=${total:-0}
    done=0
  fi

  echo "${total} ${done} ${in_progress} ${proposed} ${draft} ${ready} ${deferred} ${superseded}"
}

# Extract header count from module file (e.g. "43/64" or "In Progress (15/16)")
extract_header_count() {
  local file="$1"
  # Look for N/N pattern in the header table (first 20 lines)
  head -20 "$file" | grep -oE '[0-9]+/[0-9]+' | head -1 || echo ""
}

# Extract status from module header table row (lines starting with |)
extract_header_status() {
  local file="$1"
  # Only look at table data rows in the first 20 lines, excluding
  # header labels and markdown separator rows (| ---- | ---- |)
  local table_line
  table_line=$(head -20 "$file" \
    | grep -E '^\|' \
    | grep -viE '^\| *(ID|Scope|Module) ' \
    | grep -vE '^[[:space:]|:-]+$' \
    | head -1 || true)

  if [[ -z "$table_line" ]]; then
    echo "Unknown"
    return
  fi

  if echo "$table_line" | grep -qi 'Complete'; then echo "Complete"
  elif echo "$table_line" | grep -qi 'In Progress'; then echo "In Progress"
  elif echo "$table_line" | grep -qi 'Ready'; then echo "Ready"
  elif echo "$table_line" | grep -qi 'Proposed'; then echo "Proposed"
  elif echo "$table_line" | grep -qi 'Draft'; then echo "Draft"
  else echo "Unknown"
  fi
}

for module_file in "$MODULES_DIR"/*.aps.md; do
  module=$(basename "$module_file" .aps.md)

  # Extract prefix from first data row in the module header table, skipping
  # header labels (ID, Scope, Module) and separator rows (| --- | --- |)
  prefix=$(head -20 "$module_file" | grep -E '^\|' \
    | grep -viE '^\| *(ID|Scope|Module) ' \
    | grep -vE '^[[:space:]|:-]+$' \
    | head -1 \
    | sed -n 's/^| *\([A-Z][A-Z0-9]*\) .*/\1/p' || true)

  [[ -z "$prefix" ]] && continue

  read -r total done in_progress proposed draft ready deferred superseded <<< "$(count_work_items "$module_file" "$prefix")"

  [[ "$total" -eq 0 ]] && continue

  header_count=$(extract_header_count "$module_file")
  header_status=$(extract_header_status "$module_file")

  # Check header count vs actual
  if [[ -n "$header_count" ]]; then
    header_done="${header_count%%/*}"
    header_total="${header_count##*/}"

    if [[ "$header_done" -ne "$done" ]] || [[ "$header_total" -ne "$total" ]]; then
      finding "COUNT MISMATCH: $module — header says $header_count, actual is $done/$total"
    fi
  fi

  # Check if status should change
  active=$((total - deferred - superseded))
  if [[ "$done" -eq "$active" ]] && [[ "$active" -gt 0 ]] && [[ "$header_status" != "Complete" ]]; then
    if [[ "$deferred" -gt 0 ]]; then
      finding "STATUS: $module — all active items done ($done/$total, $deferred deferred), status is '$header_status' not Complete"
    else
      finding "STATUS: $module — all items done ($done/$total), status is '$header_status' not Complete"
    fi
  fi

done

# ── 3. Archive check ────────────────────────────────────────────────────────
# Flag Complete modules still in plans/modules/ (should be in archive)
log "Checking for Complete modules not yet archived..."

for module_file in "$MODULES_DIR"/*.aps.md; do
  module=$(basename "$module_file" .aps.md)
  status=$(extract_header_status "$module_file")

  if [[ "$status" == "Complete" ]]; then
    finding "ARCHIVE: $module — status is Complete but still in plans/modules/"
  fi
done

# ── 4. Post-merge test plans ────────────────────────────────────────────────
log "Checking post-merge test plans..."

for plan_file in "$POST_MERGE_DIR"/*.md; do
  [[ "$(basename "$plan_file")" == "TEMPLATE.md" ]] && continue

  branch_slug=$(basename "$plan_file" .md)
  unchecked=$(grep -c '^\- \[ \]' "$plan_file" || true)
  human_required=$(grep -c 'human required\|agent: no' "$plan_file" || true)
  agent_runnable=$(grep -c 'agent: yes' "$plan_file" || true)

  if [[ "$unchecked" -eq 0 ]]; then
    continue
  fi

  finding "POST-MERGE: $branch_slug — $unchecked steps remaining ($agent_runnable agent-runnable, $human_required human-required)"
done

# ── 5. Branch hygiene ───────────────────────────────────────────────────────
log "Checking branch hygiene..."

stale_branches=$(git branch -r --merged origin/dev 2>/dev/null \
  | grep -v 'HEAD\|main\|dev\|release/' \
  | sed 's/^[[:space:]]*//' || true)

if [[ -n "$stale_branches" ]]; then
  count=$(echo "$stale_branches" | grep -c . || true)
  finding "BRANCHES: $count merged branches still open (consider deleting)"
fi

dev_ahead=$(git rev-list --count origin/main..origin/dev 2>/dev/null || echo 0)
if [[ "$dev_ahead" -gt 20 ]]; then
  finding "DRIFT: dev is $dev_ahead commits ahead of main — promotion overdue"
fi

# ── 6. Index staleness check ────────────────────────────────────────────────
log "Checking index.aps.md for stale entries..."

INDEX="$REPO/plans/index.aps.md"
if [[ -f "$INDEX" ]]; then
  # Count modules in plans/modules/ vs linked in index
  module_count=$(ls "$MODULES_DIR"/*.aps.md 2>/dev/null | wc -l)
  indexed_count=$(grep -cE '\./modules/.*\.aps\.md\)' "$INDEX" 2>/dev/null || echo 0)
  archived_linked=$(grep -cE '\./archive/modules/.*\.aps\.md\)' "$INDEX" 2>/dev/null || echo 0)

  unindexed=$((module_count - indexed_count))
  if [[ "$unindexed" -gt 0 ]]; then
    finding "INDEX: $unindexed module(s) in plans/modules/ not linked in index.aps.md"
  fi
fi

# ── 7. Write results ────────────────────────────────────────────────────────
if [[ -n "$FINDINGS" ]]; then
  log "Findings:"
  echo -e "$FINDINGS" | while IFS= read -r line; do
    [[ -n "$line" ]] && log "  $line"
  done || true

  if [[ "$DRY_RUN" == false ]]; then
    mkdir -p "$(dirname "$LOG")"
    {
      echo ""
      echo "## Cleanup run — $TIMESTAMP"
      echo ""
      echo -e "$FINDINGS"
    } >> "$LOG"
    log "Findings written to cleanup-log.md"
  fi
else
  log "No issues found."
fi

# ── 8. Notify ───────────────────────────────────────────────────────────────
if [[ "$NOTIFY" == true ]] && [[ -n "$FINDINGS" ]]; then
  if echo -e "$FINDINGS" | grep -qE 'MISMATCH|STATUS|DRIFT|ARCHIVE'; then
    ALERT=$(echo -e "$FINDINGS" | grep -E 'MISMATCH|STATUS|DRIFT|ARCHIVE' | head -5)
    openclaw message send --channel telegram \
      "APS cleanup — $(date '+%H:%M')\n\n$ALERT" 2>/dev/null \
      || log "Notification failed (openclaw not available)"
  fi
fi

log "Done."
