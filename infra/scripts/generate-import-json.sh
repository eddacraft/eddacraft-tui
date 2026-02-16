#!/usr/bin/env bash
set -euo pipefail

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

echo "Fetching Vercel project IDs..."

website_prj=$(curl -fsS -H "Authorization: Bearer $VERCEL_API_TOKEN" \
  "https://api.vercel.com/v9/projects/website" | jq -r '.id')
echo "  website: $website_prj"

docs_prj=$(curl -fsS -H "Authorization: Bearer $VERCEL_API_TOKEN" \
  "https://api.vercel.com/v9/projects/docs-site" | jq -r '.id')
echo "  docs-site: $docs_prj"

echo "Fetching Vercel env var IDs for website..."

website_envs=$(curl -fsS -H "Authorization: Bearer $VERCEL_API_TOKEN" \
  "https://api.vercel.com/v10/projects/$website_prj/env")

require_single_id() {
  local key="$1" ids="$2"
  local count
  count=$(echo "$ids" | grep -c . || true)
  if [ "$count" -eq 0 ] || [ -z "$ids" ]; then
    echo "Error: No env var found for key '$key'" >&2; exit 1
  elif [ "$count" -gt 1 ]; then
    echo "Error: Multiple env vars found for key '$key' — expected exactly 1:" >&2
    echo "$ids" >&2; exit 1
  fi
  echo "$ids"
}

website_env_db_raw=$(echo "$website_envs" | jq -r '.envs[] | select(.key == "DATABASE_URL") | .id')
website_env_db=$(require_single_id "DATABASE_URL" "$website_env_db_raw")
echo "  DATABASE_URL: $website_env_db"

website_env_unosend_raw=$(echo "$website_envs" | jq -r '.envs[] | select(.key == "UNOSEND_API_KEY") | .id')
website_env_unosend=$(require_single_id "UNOSEND_API_KEY" "$website_env_unosend_raw")
echo "  UNOSEND_API_KEY: $website_env_unosend"

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
      "type": "anvil:vercel:App",
      "name": "docs-site-app",
      "logicalName": "docs-site",
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
      "logicalName": "website-database-url",
      "id": "$website_prj/$website_env_db",
      "parent": "website-app"
    },
    {
      "type": "vercel:index/projectEnvironmentVariable:ProjectEnvironmentVariable",
      "name": "website-unosend-env",
      "logicalName": "website-unosend-api-key",
      "id": "$website_prj/$website_env_unosend",
      "parent": "website-app"
    },

    {
      "type": "vercel:index/project:Project",
      "name": "docs-project",
      "logicalName": "docs-site",
      "id": "$docs_prj",
      "parent": "docs-site-app"
    },
    {
      "type": "vercel:index/projectDomain:ProjectDomain",
      "name": "docs-domain",
      "logicalName": "docs-site-docs-eddacraft-ai",
      "id": "$docs_prj/docs.eddacraft.ai",
      "parent": "docs-site-app"
    },

    {
      "type": "azure-native:dns:RecordSet",
      "name": "root-txt-eddacraft-ai",
      "id": "$DNS_BASE/TXT/@"
    },
    {
      "type": "azure-native:dns:RecordSet",
      "name": "dmarc-eddacraft-ai",
      "id": "$DNS_BASE/TXT/_dmarc"
    },
    {
      "type": "azure-native:dns:RecordSet",
      "name": "mx-send-eddacraft-ai",
      "id": "$DNS_BASE/MX/send"
    },
    {
      "type": "azure-native:dns:RecordSet",
      "name": "txt-send-eddacraft-ai",
      "id": "$DNS_BASE/TXT/send"
    },
    {
      "type": "azure-native:dns:RecordSet",
      "name": "dmarc-send-eddacraft-ai",
      "id": "$DNS_BASE/TXT/_dmarc.send"
    },
    {
      "type": "azure-native:dns:RecordSet",
      "name": "unosend-dkim-eddacraft-ai",
      "id": "$DNS_BASE/TXT/unosend._domainkey"
    }
  ]
}
ENDJSON

echo ""
echo "Generated $OUTPUT with $(jq '.resources | length' "$OUTPUT") resources"
echo "  - 2 VercelApp components"
echo "  - 6 Vercel child resources (2 projects, 2 domains, 2 env vars)"
echo "  - 6 Azure DNS RecordSets"
