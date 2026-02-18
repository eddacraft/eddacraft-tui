#!/usr/bin/env bash
set -euo pipefail

# Local-only helper for unattended Codex runs in this repo.
# Usage:
#   tools/local-agent-run.sh "Fix flaky signup test in apps/web"
#   tools/local-agent-run.sh --task-file /tmp/task.txt
#
# Notes:
# - Runs non-interactively via `codex exec --full-auto`
# - Logs to plans/agent-runs/<timestamp>.log
# - Sends OpenClaw wake event on completion/failure when available

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LOG_DIR="$ROOT_DIR/plans/agent-runs"
mkdir -p "$LOG_DIR"

TASK=""
if [[ "${1:-}" == "--task-file" ]]; then
  FILE_PATH="${2:-}"
  if [[ -z "$FILE_PATH" || ! -f "$FILE_PATH" ]]; then
    echo "Error: --task-file requires an existing file path" >&2
    exit 2
  fi
  TASK="$(cat "$FILE_PATH")"
else
  TASK="${1:-}"
fi

if [[ -z "$TASK" ]]; then
  cat >&2 <<'USAGE'
Usage:
  tools/local-agent-run.sh "<task prompt>"
  tools/local-agent-run.sh --task-file <path>
USAGE
  exit 2
fi

if ! command -v codex >/dev/null 2>&1; then
  echo "Error: codex not found in PATH" >&2
  exit 127
fi

STAMP="$(date +%Y%m%d-%H%M%S)"
LOG_FILE="$LOG_DIR/$STAMP.log"

read -r -d '' RUN_PROMPT <<EOF || true
You are working in: $ROOT_DIR

Task:
$TASK

Execution requirements:
- Keep scope tight and production-safe.
- Run relevant lint/tests/build for changed areas.
- At the end, provide:
  1) Files changed
  2) Checks run + pass/fail
  3) Remaining risks / follow-ups

When completely finished, run this command:
openclaw system event --text "Done: Codex task finished ($STAMP)" --mode now
EOF

{
  echo "[$(date -Is)] Starting Codex run"
  echo "[$(date -Is)] Repo: $ROOT_DIR"
  echo "[$(date -Is)] Log:  $LOG_FILE"
} | tee "$LOG_FILE"

set +e
(
  cd "$ROOT_DIR"
  codex exec --full-auto "$RUN_PROMPT"
) 2>&1 | tee -a "$LOG_FILE"
CODEX_EXIT=${PIPESTATUS[0]}
set -e

STATUS_TEXT="Done"
if [[ $CODEX_EXIT -ne 0 ]]; then
  STATUS_TEXT="Failed(exit=$CODEX_EXIT)"
fi

if command -v openclaw >/dev/null 2>&1; then
  openclaw system event --text "$STATUS_TEXT: local-agent-run $STAMP (log: $LOG_FILE)" --mode now || true
fi

echo "[$(date -Is)] $STATUS_TEXT (log: $LOG_FILE)"
exit "$CODEX_EXIT"
