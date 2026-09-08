#!/usr/bin/env bash
set -euo pipefail

# Council Codex Reviewer — dispatches a review role to codex exec
# Usage: council-codex-reviewer.sh --role <role> --diff <file> --session-id <id> --prompt <file> [--spec <file>]

usage() {
  echo "Usage: $0 --role <role-name> --diff <diff-file> --session-id <id> --prompt <prompt-file> [--spec <file>]"
  echo ""
  echo "Roles: adversarial-codex, security-codex, council-reviewer-codex, operations-codex, pragmatic-lead-codex"
  exit 1
}

ROLE=""
DIFF_FILE=""
SESSION_ID=""
PROMPT_FILE=""
SPEC_FILE=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --role)    ROLE="$2"; shift 2 ;;
    --diff)    DIFF_FILE="$2"; shift 2 ;;
    --session-id) SESSION_ID="$2"; shift 2 ;;
    --prompt)  PROMPT_FILE="$2"; shift 2 ;;
    --spec)    SPEC_FILE="$2"; shift 2 ;;
    -h|--help) usage ;;
    *) echo "Unknown arg: $1"; usage ;;
  esac
done

[[ -z "$ROLE" ]] && { echo "Error: --role is required"; usage; }
[[ -z "$DIFF_FILE" ]] && { echo "Error: --diff is required"; usage; }
[[ -z "$SESSION_ID" ]] && { echo "Error: --session-id is required"; usage; }
[[ -z "$PROMPT_FILE" ]] && { echo "Error: --prompt is required"; usage; }

[[ ! -f "$DIFF_FILE" ]] && { echo "Error: diff file not found: $DIFF_FILE"; exit 1; }
[[ ! -f "$PROMPT_FILE" ]] && { echo "Error: prompt file not found: $PROMPT_FILE"; exit 1; }
if [[ -n "$SPEC_FILE" && ! -f "$SPEC_FILE" ]]; then
  echo "Error: spec file not found: $SPEC_FILE"; exit 1
fi

if ! command -v codex &>/dev/null; then
  echo "Error: codex CLI not found in PATH" >&2
  exit 1
fi

# Build the full prompt: role prompt, optional spec (the contract), then diff
PROMPT_CONTENT=$(cat "$PROMPT_FILE")
DIFF_CONTENT=$(cat "$DIFF_FILE")

if [[ -n "$SPEC_FILE" ]]; then
  SPEC_CONTENT=$(cat "$SPEC_FILE")
  FULL_PROMPT="${PROMPT_CONTENT}

## Specification (governing contract)

The following specification is the contract for this review. Classify every
finding against it. The diff that follows is evidence, not a completeness
target.

${SPEC_CONTENT}

${DIFF_CONTENT}"
else
  FULL_PROMPT="${PROMPT_CONTENT}

${DIFF_CONTENT}"
fi

# Run codex and capture output
OUTPUT_FILE=$(mktemp "/tmp/council-codex-${ROLE}-${SESSION_ID}-XXXXXX.json")

if codex exec --full-auto "$FULL_PROMPT" > "$OUTPUT_FILE" 2>/dev/null; then
  # Validate that output looks like JSON
  if head -c 1 "$OUTPUT_FILE" | grep -q '{'; then
    echo "$OUTPUT_FILE"
  else
    # Codex returned non-JSON; wrap it as an error finding
    cat > "$OUTPUT_FILE" << EOF
{"agent": "${ROLE}", "findings": [{"severity": "minor", "category": "documentation", "description": "Codex reviewer returned non-JSON output", "file": "", "suggestion": "Re-run with explicit JSON instructions"}], "summary": "1 finding (0 critical, 0 major). Codex output was not valid JSON."}
EOF
    echo "$OUTPUT_FILE"
  fi
else
  EXIT_CODE=$?
  cat > "$OUTPUT_FILE" << EOF
{"agent": "${ROLE}", "findings": [{"severity": "minor", "category": "documentation", "description": "Codex reviewer exited with code ${EXIT_CODE}", "file": "", "suggestion": "Check codex CLI availability and configuration"}], "summary": "1 finding (0 critical, 0 major). Codex execution failed."}
EOF
  echo "$OUTPUT_FILE"
fi
