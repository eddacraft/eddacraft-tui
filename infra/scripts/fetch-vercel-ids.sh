#!/usr/bin/env bash
set -euo pipefail

# Source logging library if available
_SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
_LOG_LIB="$(git rev-parse --show-toplevel 2>/dev/null || echo "${_SCRIPT_DIR}/../..")/.claude/hooks/lib/log.sh"
if [ -f "$_LOG_LIB" ]; then
  export ANVIL_LOG_TAG="infra:fetch-vercel"
  source "$_LOG_LIB"
fi

type log_enter >/dev/null 2>&1 && log_enter "$@"

if [ -z "${VERCEL_TOKEN:-}" ]; then
  echo "Error: VERCEL_TOKEN not set" >&2
  exit 1
fi

if ! command -v jq >/dev/null 2>&1; then
  echo "Error: jq is not installed or not found in PATH" >&2
  exit 1
fi

type log_info >/dev/null 2>&1 && log_info "fetching Vercel project IDs"
echo "Fetching Vercel project IDs..."
echo ""

type log_debug >/dev/null 2>&1 && log_debug "calling Vercel API: /v9/projects"
curl -fsS -H "Authorization: Bearer $VERCEL_TOKEN" \
  https://api.vercel.com/v9/projects \
  | jq -r '.projects[] | select(.name == "website" or .name == "anvil-api" or
      .name == "eddacraft-docs-shell" or .name == "eddacraft-docs-public" or
      .name == "eddacraft-anvil-docs-private") |
    "# \(.name) (id: \(.id))\npulumi import vercel:index/project:Project \(.name) \(.id)\n"'
