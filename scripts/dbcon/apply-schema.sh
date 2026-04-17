#!/usr/bin/env bash
# DBCON-002: apply the canonical anvil-api schema to a target Neon DB.
#
# The canonical schema lives at apps/anvil-api/src/db/schema.sql and creates
# all 7 tables plus required extensions (citext, pgcrypto).
#
# Usage:
#   ./apply-schema.sh <DB_URL>
#
# Example:
#   ./apply-schema.sh "$ANVIL_API_PROD_URL"
set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "usage: $0 <DB_URL>" >&2
  exit 2
fi

DB_URL="$1"

REPO_ROOT="$(git rev-parse --show-toplevel)"
SCHEMA="$REPO_ROOT/apps/anvil-api/src/db/schema.sql"

[[ -f "$SCHEMA" ]] || { echo "schema not found: $SCHEMA" >&2; exit 1; }

echo "→ applying $SCHEMA …" >&2
psql "$DB_URL" -v ON_ERROR_STOP=1 -f "$SCHEMA"

echo "→ schema applied; verifying …" >&2
psql "$DB_URL" -v ON_ERROR_STOP=1 <<'SQL'
\echo → tables:
\dt
\echo → extensions:
SELECT extname FROM pg_extension ORDER BY extname;
SQL
