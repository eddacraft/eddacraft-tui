#!/usr/bin/env bash
# aps-cleanup.sh — APS reconciliation and hygiene
# Runs on schedule via systemd timer. Reconciles module statuses and counts
# against actual work item states, flags stale entries, checks branch hygiene.
#
# Usage: ./aps-cleanup.sh [--repo=<path>] [--dry-run] [--notify]
# Default repo: ~/Projects/src/anvil-001
#
# Modes:
#   (default)  Report findings only, append to cleanup-log.md
#   --dry-run  Print findings to stdout, don't write anything
#   --notify   Send alerts for items needing human attention

set -euo pipefail

REPO="${REPO:-$HOME/Projects/src/anvil-001}"
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

# ── --advance-released: release-time Merged → Released/Shipped (#1715) ───────
# Delegates to scripts/aps/advance-released.mjs (node — reuses the APS module
# parser + handles both `###` and `####` item headings). Invoked as:
#   aps-cleanup.sh --advance-released --release-record <path> --tag <tag> \
#     --sha <sha> --date <date> [--dry-run] [--repo=<path>]
# Replaces the manual awk|jq|perl walk in the release runbook §13.
advance_released_mode() {
  local release_record="" tag="" sha="" date="" repo_override=""
  local -a passthru=()   # array, so optional flags can't word-split
  while [ $# -gt 0 ]; do
    case "$1" in
      --release-record)   release_record="$2"; shift ;;
      --release-record=*) release_record="${1#*=}" ;;
      --tag)   tag="$2";  shift ;;
      --tag=*) tag="${1#*=}" ;;
      --sha)   sha="$2";  shift ;;
      --sha=*) sha="${1#*=}" ;;
      --date)   date="$2"; shift ;;
      --date=*) date="${1#*=}" ;;
      --dry-run) passthru+=(--dry-run) ;;
      --repo)    repo_override="$2"; shift ;;
      --repo=*)  repo_override="${1#*=}" ;;
    esac
    shift
  done
  local script_dir; script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
  local root="$repo_override"
  if [ -z "$root" ]; then
    root="$(git -C "$script_dir" rev-parse --show-toplevel 2>/dev/null)" \
      || { echo "[aps-cleanup] error: cannot locate repo root; pass --repo=<path>" >&2; exit 2; }
  fi
  exec node "$script_dir/aps/advance-released.mjs" \
    --root="$root" --release-record="$release_record" \
    --tag="$tag" --sha="$sha" --date="$date" "${passthru[@]}"
}

for arg in "$@"; do
  if [ "$arg" = "--advance-released" ]; then
    advance_released_mode "$@"   # execs node; does not return
  fi
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

# APS lifecycle vocabulary (dev-workflow rule 5):
#   Draft → Proposed → Ready → In Progress → Merged → Released/Shipped → Complete
# Plus accepted variants used across the repo:
#   Done       — implementation-finished waypoint (== Merged in the older
#                modules that predate the Merged terminology)
#   Resolved   — design-question-answered (used by `dashboard-foundation`)
#   Todo       — pre-flight waypoint, equivalent to Draft (used by V050F)
#   Blocked    — in-flight but waiting on an external dep
#   Deferred   — punted; excluded from active count
#   Superseded — replaced by another item; excluded from active count
DONE_STATUSES_RE='complete|done|merged|released|shipped|resolved'
TERMINAL_HEADER_STATUSES_RE='Complete|Done|Merged|Released|Shipped'

# Classify a single status-line string into one bucket. Priority order
# matters: a line like `Status: Complete — merged via PR #1234` contains
# both "complete" and "merged" — the terminal bucket should win exactly
# once, not double-count across `done` and `merged`.
classify_status_line() {
  local lower
  lower=$(echo "$1" | tr '[:upper:]' '[:lower:]')
  case "$lower" in
    *complete*|*done*|*merged*|*released*|*shipped*|*resolved*) echo "done" ;;
    *in\ progress*|*in-progress*)                               echo "in_progress" ;;
    *ready*)                                                    echo "ready" ;;
    *proposed*)                                                 echo "proposed" ;;
    *blocked*)                                                  echo "blocked" ;;
    *todo*)                                                     echo "todo" ;;
    *deferred*)                                                 echo "deferred" ;;
    *superseded*)                                               echo "superseded" ;;
    *draft*)                                                    echo "draft" ;;
    *)                                                          echo "unknown" ;;
  esac
}

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
  # Items can be at heading depth `###` (most modules) or `####`
  # (multilayer-protection-v2 nests items under group headings, so the
  # work items sit one level deeper). Accept either. Within a single
  # item, only the FIRST `**Status:**` line is canonical — some items
  # carry sub-progress notes that also use the `**Status:**` shape and
  # would otherwise inflate the count (e.g. tui-dashboard-render's
  # TUIDASH-001 carries a primary + nested status).
  local status_block
  status_block=$(awk -v pfx="$prefix" '
    BEGIN { re="^###+ " pfx "-[0-9]" }
    $0 ~ re { in_item=1; status_found=0; next }
    /^####? [A-Z][A-Z0-9]*-[0-9]/ { in_item=0 }
    /^## / { in_item=0 }
    in_item && !status_found && /(^- \*\*Status\*\*|^- \*\*Status:|^Status:)/ {
      print; status_found=1
    }
  ' "$file" 2>/dev/null || true)

  if [[ -n "$status_block" ]]; then
    # Classify each status line into exactly one bucket (priority chain)
    # rather than running independent regex counters and double-counting
    # lines like `Status: Complete — merged via PR #...`. The pre-fix
    # script under-counted modules whose statuses use the `Merged` /
    # `Released` / `Resolved` / `Todo` vocabulary because the individual
    # `grep -c` calls only matched `complete|done`.
    while IFS= read -r line; do
      [[ -z "$line" ]] && continue
      case "$(classify_status_line "$line")" in
        done)        done=$((done + 1)) ;;
        in_progress) in_progress=$((in_progress + 1)) ;;
        proposed)    proposed=$((proposed + 1)) ;;
        ready)       ready=$((ready + 1)) ;;
        blocked)     in_progress=$((in_progress + 1)) ;;
        todo)        draft=$((draft + 1)) ;;
        deferred)    deferred=$((deferred + 1)) ;;
        superseded)  superseded=$((superseded + 1)) ;;
        draft)       draft=$((draft + 1)) ;;
      esac
    done <<< "$status_block"
    # `total` is the heading count (authoritative item count) — not the
    # sum of classified statuses. The pre-fix script summed the buckets,
    # which under-reported modules whose items omit an explicit
    # `**Status:**` line (a few items per module is common — e.g.,
    # `dashboard-foundation` carries design-question items whose body
    # paragraphs serve as their resolution rather than a structured
    # `**Status:**` field). An unspecified-status item contributes to
    # `total` but not to any bucket; the caller still sees the right
    # `done/total` ratio.
    local heading_count
    heading_count=$(grep -cE "^###+ ${prefix}-[0-9]" "$file" 2>/dev/null || echo 0)
    heading_count=${heading_count:-0}
    local bucket_sum=$((done + in_progress + proposed + draft + ready + deferred + superseded))
    if [[ "$heading_count" -ge "$bucket_sum" ]]; then
      total=$heading_count
    else
      total=$bucket_sum
    fi
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

  # Method 3: Table rows with terminal-done status in a column.
  # `Done|Complete|Merged|Released|Shipped|Resolved` are all variants of
  # "no longer in flight".
  if [[ "$total" -eq 0 ]]; then
    local table_total
    table_total=$(grep -cE "^\| ${prefix}-[0-9]" "$file" 2>/dev/null || true)
    table_total=${table_total:-0}
    if [[ "$table_total" -gt 0 ]]; then
      done=$(grep -ciE "^\| ${prefix}-[0-9].*\| (${DONE_STATUSES_RE})" "$file" 2>/dev/null || true)
      done=${done:-0}
      total=$table_total
    fi
  fi

  # Method 4: Just count headings (items exist but no status). Match
  # `###` and `####` so modules with grouped sub-headings (MLP2-style)
  # are counted the same as flat-list modules.
  if [[ "$total" -eq 0 ]]; then
    total=$(grep -cE "^###+ ${prefix}-[0-9]" "$file" 2>/dev/null || true)
    total=${total:-0}
    done=0
  fi

  echo "${total} ${done} ${in_progress} ${proposed} ${draft} ${ready} ${deferred} ${superseded}"
}

# Extract header count from module file (e.g. "43/64" or "In Progress (15/16)")
extract_header_count() {
  local file="$1"
  # Look for N/N pattern in the header *table row* only. Anchoring to lines
  # that start with `|` stops us picking up things like "CLAR-007/008" from
  # a module header comment, which previously produced a false COUNT
  # MISMATCH on the reparented QLODX module.
  head -20 "$file" \
    | grep -E '^\|' \
    | grep -viE '^\| *(ID|Scope|Module) ' \
    | grep -vE '^[[:space:]|:-]+$' \
    | grep -oE '[0-9]+/[0-9]+' \
    | head -1 || echo ""
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

  # Priority chain mirrors `classify_status_line`. Terminal-but-pre-release
  # statuses (Done / Merged / Released / Shipped) come BEFORE Complete in
  # the test order because a row like
  #     `| WOUT | @aneki | Done | 6/6 |`
  # would otherwise match `Complete` against the (absent) literal — actually
  # neither matches, so the order only matters when a row carries prose
  # like `Done — Completion verified`. Keep the most-specific bucket first.
  if echo "$table_line" | grep -qiE '\<Complete\>'; then echo "Complete"
  elif echo "$table_line" | grep -qiE '\<Released\>|\<Shipped\>'; then echo "Released"
  elif echo "$table_line" | grep -qiE '\<Merged\>'; then echo "Merged"
  elif echo "$table_line" | grep -qiE '\<Done\>'; then echo "Done"
  elif echo "$table_line" | grep -qi 'In Progress'; then echo "In Progress"
  elif echo "$table_line" | grep -qiE '\<Blocked\>'; then echo "Blocked"
  elif echo "$table_line" | grep -qiE '\<Ready\>'; then echo "Ready"
  elif echo "$table_line" | grep -qiE '\<Proposed\>'; then echo "Proposed"
  elif echo "$table_line" | grep -qiE '\<Draft\>'; then echo "Draft"
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

  # Check if status should advance. A module whose items are all done but
  # whose module-level status is still `In Progress` / `Ready` / `Draft` /
  # `Proposed` / `Blocked` / `Unknown` is genuinely stale and should
  # advance. Statuses that are themselves terminal-or-pre-release
  # (`Complete`, `Done`, `Merged`, `Released`, `Shipped`) are accepted —
  # `Done` modules legitimately wait for release evidence before
  # advancing to Complete (dev-workflow rule 5 lifecycle:
  # `Merged → Released/Shipped → Complete`).
  active=$((total - deferred - superseded))
  if [[ "$done" -eq "$active" ]] && [[ "$active" -gt 0 ]] \
    && ! echo "$header_status" | grep -qiE "^(${TERMINAL_HEADER_STATUSES_RE})\$"; then
    if [[ "$deferred" -gt 0 ]]; then
      finding "STATUS: $module — all active items done ($done/$total, $deferred deferred), status is '$header_status' not Complete/Done/Merged/Released/Shipped"
    else
      finding "STATUS: $module — all items done ($done/$total), status is '$header_status' not Complete/Done/Merged/Released/Shipped"
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
# Main-first since the OPMODEL-012 cutover (2026-05-11): `main` is the sole
# integration target and `dev` is retired, so branches are measured merged
# against `origin/main`, and the old dev→main promotion-drift check is gone.
log "Checking branch hygiene..."

stale_branches=$(git branch -r --merged origin/main 2>/dev/null \
  | sed 's/^[[:space:]]*//' \
  | grep -vE '^origin/HEAD ->|^origin/main$|^origin/release/' || true)

if [[ -n "$stale_branches" ]]; then
  count=$(echo "$stale_branches" | grep -c . || true)
  finding "BRANCHES: $count merged branches still open (consider deleting)"
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
  if echo -e "$FINDINGS" | grep -qE 'MISMATCH|STATUS|ARCHIVE'; then
    ALERT=$(echo -e "$FINDINGS" | grep -E 'MISMATCH|STATUS|ARCHIVE' | head -5)
    openclaw message send --channel telegram \
      "APS cleanup — $(date '+%H:%M')\n\n$ALERT" 2>/dev/null \
      || log "Notification failed (openclaw not available)"
  fi
fi

log "Done."
