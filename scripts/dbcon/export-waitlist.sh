#!/usr/bin/env bash
# DBCON-003: export waitlist rows from a source Neon DB to CSV.
#
# Columns exported: email, name, company, role, use_case, source, created_at, updated_at
# The serial `id` column is intentionally omitted — the target DB re-generates
# ids on insert; email is the dedup key.
#
# Run this once per legacy source (eddacraft-web, beta-user-tokens) with a
# unique OUT path.
#
# Required env:
#   WAITLIST_DB_URL — connection string for the source Neon project
#
# Usage:
#   WAITLIST_DB_URL=postgresql://... ./export-waitlist.sh [out.csv]
#
# Default output: waitlist-export.csv in the current directory.
set -euo pipefail

: "${WAITLIST_DB_URL:?WAITLIST_DB_URL is required (source Neon connection string)}"

OUT="${1:-waitlist-export.csv}"

psql "$WAITLIST_DB_URL" -v ON_ERROR_STOP=1 <<SQL
\copy (SELECT email, name, company, role, use_case, source, created_at, updated_at FROM waitlist ORDER BY email) TO '$OUT' WITH CSV HEADER
SQL

LINES=$(wc -l < "$OUT" | tr -d ' ')
ROWS=$((LINES - 1))
echo "→ exported $ROWS waitlist rows to $OUT (header + $ROWS data lines)" >&2
