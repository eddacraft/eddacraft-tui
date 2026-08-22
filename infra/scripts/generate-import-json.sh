#!/usr/bin/env bash
set -euo pipefail

# Source logging library if available
_SCRIPT_SELF_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
_LOG_LIB="$(git rev-parse --show-toplevel 2>/dev/null || echo "${_SCRIPT_SELF_DIR}/../..")/.claude/hooks/lib/log.sh"
if [ -f "$_LOG_LIB" ]; then
  export ANVIL_LOG_TAG="infra:gen-import"
  source "$_LOG_LIB"
fi

type log_enter >/dev/null 2>&1 && log_enter "$@"

# Generates import.json for pulumi import --file
# Queries Vercel API for project/domain/env-var IDs and constructs Azure DNS resource IDs
#
# Required env vars:
#   VERCEL_API_TOKEN    - Vercel API bearer token
#   ARM_SUBSCRIPTION_ID - Azure subscription ID

: "${VERCEL_API_TOKEN:?VERCEL_API_TOKEN is required}"
: "${ARM_SUBSCRIPTION_ID:?ARM_SUBSCRIPTION_ID is required}"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OUTPUT="${SCRIPT_DIR}/../import.json"

type log_info >/dev/null 2>&1 && log_info "generating import.json"
type log_debug >/dev/null 2>&1 && log_debug "output=${OUTPUT}"

echo "Fetching Vercel project IDs..."

type log_debug >/dev/null 2>&1 && log_debug "fetching website project ID..."
website_prj=$(curl -fsS -H "Authorization: Bearer $VERCEL_API_TOKEN" \
  "https://api.vercel.com/v9/projects/website" | jq -r '.id')
echo "  website: $website_prj"
type log_debug >/dev/null 2>&1 && log_debug "website_prj=${website_prj}"

echo "Fetching Vercel env var IDs for website..."
type log_debug >/dev/null 2>&1 && log_debug "fetching env vars for website..."

website_envs=$(curl -fsS -H "Authorization: Bearer $VERCEL_API_TOKEN" \
  "https://api.vercel.com/v10/projects/$website_prj/env")

require_id() {
  local key="$1" ids="$2"
  local count
  count=$(echo "$ids" | grep -c . || true)
  if [ "$count" -eq 0 ] || [ -z "$ids" ]; then
    echo "Error: No env var found for key '$key'" >&2; exit 1
  elif [ "$count" -gt 1 ]; then
    # Vercel may have separate env vars per target (production/preview).
    # Pulumi creates one env var with both targets, so pick the first —
    # Pulumi will reconcile targets on the next `up`.
    echo "  (multiple env vars for '$key' — using first for import)" >&2
    echo "$ids" | head -1
    return
  fi
  echo "$ids"
}

website_env_db_raw=$(echo "$website_envs" | jq -r '.envs[] | select(.key == "DATABASE_URL") | .id')
website_env_db=$(require_id "DATABASE_URL" "$website_env_db_raw")
echo "  DATABASE_URL: $website_env_db"

website_env_resend_raw=$(echo "$website_envs" | jq -r '.envs[] | select(.key == "RESEND_API_KEY") | .id')
website_env_resend=$(require_id "RESEND_API_KEY" "$website_env_resend_raw")
echo "  RESEND_API_KEY: $website_env_resend"

echo "Constructing Azure DNS resource IDs..."

SUB="$ARM_SUBSCRIPTION_ID"
RG="rg-prd-ap-public-web"
ZONE="eddacraft.ai"
DNS_BASE="/subscriptions/$SUB/resourceGroups/$RG/providers/Microsoft.Network/dnsZones/$ZONE"

echo "  Base: $DNS_BASE"

# Build the import JSON
# - Component resources (VercelApp) use "component": true, no "id"
# - Child resources reference parent by the "name" key in this file
# - "logicalName" maps to the actual Pulumi resource name in the program
cat > "$OUTPUT" <<ENDJSON
{
  "resources": [
    {
      "type": "anvil:vercel:App",
      "name": "website-app",
      "logicalName": "website",
      "component": true
    },
    {
      "type": "vercel:index/project:Project",
      "name": "website-project",
      "logicalName": "website",
      "id": "$website_prj",
      "parent": "website-app"
    },
    {
      "type": "vercel:index/projectDomain:ProjectDomain",
      "name": "website-domain",
      "logicalName": "website-eddacraft-ai",
      "id": "$website_prj/eddacraft.ai",
      "parent": "website-app"
    },
    {
      "type": "vercel:index/projectEnvironmentVariable:ProjectEnvironmentVariable",
      "name": "website-db-env",
      "logicalName": "anvil-api-database-url",
      "id": "$website_prj/$website_env_db",
      "parent": "website-app"
    },
    {
      "type": "vercel:index/projectEnvironmentVariable:ProjectEnvironmentVariable",
      "name": "website-resend-env",
      "logicalName": "website-resend-api-key",
      "id": "$website_prj/$website_env_resend",
      "parent": "website-app"
    },

    {
      "type": "azure-native:dns:RecordSet",
      "name": "root-txt-eddacraft-ai",
      "id": "$DNS_BASE/TXT/@"
    }
  ]
}
ENDJSON

type log_info >/dev/null 2>&1 && log_info "import.json generated at ${OUTPUT}"
echo ""
echo "Generated $OUTPUT with $(jq '.resources | length' "$OUTPUT") resources"
echo "  - 1 VercelApp component"
echo "  - 4 Vercel child resources (1 project, 1 domain, 2 env vars)"
echo "  - 1 Azure DNS RecordSet (4 Resend records already in state)"
