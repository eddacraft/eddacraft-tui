#!/usr/bin/env bash
# aps-cleanup.sh — APS verification and post-merge test plan runner
# Runs on schedule via systemd. Checks Committed APS items, verifies merges,
# runs post-merge test plans, flags blockers.
#
# Usage: ./aps-cleanup.sh [--repo <path>] [--dry-run] [--notify]
# Default repo: ~/src-morgan/anvil-001

set -euo pipefail

REPO="${REPO:-$HOME/src-morgan/anvil-001}"
DRY_RUN=false
NOTIFY=false
LOG="$REPO/plans/reviews/cleanup-log.md"
POST_MERGE_DIR="$REPO/plans/reviews/post-merge"
TIMESTAMP=$(date '+%Y-%m-%d %H:%M %Z')
FINDINGS=""

# Parse args
for arg in "$@"; do
  case $arg in
    --dry-run) DRY_RUN=true ;;
    --notify)  NOTIFY=true ;;
    --repo=*)  REPO="${arg#*=}" ;;
  esac
done

log() { echo "[aps-cleanup] $*"; }
finding() { FINDINGS+="- $*\n"; }

# ── 1. Git sync ──────────────────────────────────────────────────────────────
log "Fetching latest from origin..."
cd "$REPO"
git fetch origin --quiet

# ── 2. APS Committed sweep ───────────────────────────────────────────────────
log "Scanning for Committed APS modules..."

COMMITTED_MODULES=$(grep -rl "Committed" "$REPO/plans/modules/" 2>/dev/null || true)

for module_file in $COMMITTED_MODULES; do
  module=$(basename "$module_file" .aps.md)

  # Find the branch associated with this module (heuristic: branch contains module slug)
  branch=$(git branch -r --merged origin/dev 2>/dev/null \
    | grep -i "$module" | head -1 | xargs || true)

  if [[ -n "$branch" ]]; then
    log "Module $module — branch ${branch} merged into dev"

    # Check CI status on the merge commit
    merge_sha=$(git log origin/dev --oneline --grep="$module" | head -1 | awk '{print $1}')
    if [[ -n "$merge_sha" ]]; then
      ci_status=$(gh run list --commit "$merge_sha" --json conclusion --jq '.[0].conclusion' 2>/dev/null || echo "unknown")
      if [[ "$ci_status" == "success" ]]; then
        log "CI green for $module — advancing to Complete"
        if [[ "$DRY_RUN" == false ]]; then
          sed -i 's/^status: Committed/status: Complete/' "$module_file"
          sed -i 's/\*\*Committed\*\*/Complete/' "$module_file"
        fi
        finding "✅ $module — advanced Committed → Complete (CI: $ci_status)"
      else
        finding "⚠️  $module — merged but CI status: $ci_status"
      fi
    fi
  else
    finding "⏳ $module — Committed, branch not yet merged to dev"
  fi
done

# ── 3. Post-merge test plans ─────────────────────────────────────────────────
log "Checking post-merge test plans..."

shopt -s nullglob
for plan_file in "$POST_MERGE_DIR"/*.md; do
  [[ "$(basename $plan_file)" == "TEMPLATE.md" ]] && continue

  branch_slug=$(basename "$plan_file" .md)
  unchecked=$(grep -c '^\- \[ \]' "$plan_file" || true)
  human_required=$(grep -c 'human required\|agent: no' "$plan_file" || true)
  agent_runnable=$(grep -c 'agent: yes' "$plan_file" || true)

  if [[ "$unchecked" -eq 0 ]]; then
    finding "✅ Post-merge plan complete: $branch_slug"
    continue
  fi

  finding "📋 Post-merge plan: $branch_slug — $unchecked steps remaining ($agent_runnable agent-runnable, $human_required human-required)"

  if [[ "$human_required" -gt 0 ]]; then
    finding "🙋 Human attention needed: $branch_slug ($(grep 'human required\|agent: no' "$plan_file" | head -3 | sed 's/^/  /'))"
  fi
done

# ── 4. Branch hygiene ────────────────────────────────────────────────────────
log "Checking branch hygiene..."

# Merged branches still open
stale_branches=$(git branch -r --merged origin/dev \
  | grep -v 'HEAD\|main\|dev\|release/' \
  | xargs || true)

if [[ -n "$stale_branches" ]]; then
  finding "🧹 Merged branches still open (consider deleting): $stale_branches"
fi

# dev → main drift
dev_ahead=$(git rev-list --count origin/main..origin/dev 2>/dev/null || echo 0)
if [[ "$dev_ahead" -gt 20 ]]; then
  finding "⚠️  dev is $dev_ahead commits ahead of main — promotion overdue"
fi

# ── 5. Write log ─────────────────────────────────────────────────────────────
if [[ -n "$FINDINGS" ]]; then
  log "Writing findings to cleanup-log.md..."
  if [[ "$DRY_RUN" == false ]]; then
    {
      echo ""
      echo "## Cleanup run — $TIMESTAMP"
      echo ""
      echo -e "$FINDINGS"
    } >> "$LOG"
  else
    echo "--- DRY RUN findings ---"
    echo -e "$FINDINGS"
  fi
fi

# ── 6. Notify ────────────────────────────────────────────────────────────────
if [[ "$NOTIFY" == true ]] && [[ -n "$FINDINGS" ]]; then
  # Check for anything needing attention
  if echo -e "$FINDINGS" | grep -q '⚠️\|🙋'; then
    ALERT=$(echo -e "$FINDINGS" | grep '⚠️\|🙋' | head -5)
    openclaw message send --channel telegram \
      "🤖 APS cleanup — $(date '+%H:%M')\n\n$ALERT" 2>/dev/null \
      || log "Notification failed (openclaw not available)"
  fi
fi

log "Done."
