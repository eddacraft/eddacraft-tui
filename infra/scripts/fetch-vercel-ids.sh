#!/usr/bin/env bash
set -euo pipefail

if [ -z "${VERCEL_TOKEN:-}" ]; then
  echo "Error: VERCEL_TOKEN not set" >&2
  exit 1
fi

if ! command -v jq >/dev/null 2>&1; then
  echo "Error: jq is not installed or not found in PATH" >&2
  exit 1
fi

echo "Fetching Vercel project IDs..."
echo ""

curl -fsS -H "Authorization: Bearer $VERCEL_TOKEN" \
  https://api.vercel.com/v9/projects \
  | jq -r '.projects[] | select(.name == "website" or .name == "docs-site") |
    "# \(.name) (id: \(.id))\npulumi import vercel:index/project:Project \(.name) \(.id)\n"'
