#!/usr/bin/env bash
# DBCON-003: idempotently load waitlist rows into anvil-api-prod.
#
# Uses a TEMP staging table + INSERT … ON CONFLICT (email) DO NOTHING so the
# script is safe to re-run and to invoke back-to-back with CSVs from
# multiple sources (dedup happens on email).
#
# Required env:
#   BETA_DB_URL — connection string for anvil-api-prod (the target)
#
# Usage:
#   BETA_DB_URL=postgresql://... ./import-waitlist.sh [in.csv]
#
# Default input: waitlist-export.csv in the current directory.
set -euo pipefail

: "${BETA_DB_URL:?BETA_DB_URL is required (target Neon connection string)}"

IN="${1:-waitlist-export.csv}"
[[ -f "$IN" ]] || { echo "missing input CSV: $IN" >&2; exit 1; }

PRE=$(psql "$BETA_DB_URL" -tAc "SELECT count(*) FROM waitlist")
echo "→ pre-import count on target: $PRE" >&2

psql "$BETA_DB_URL" -v ON_ERROR_STOP=1 <<SQL
BEGIN;

CREATE TEMP TABLE waitlist_stage (
  email      citext NOT NULL,
  name       text,
  company    text,
  role       text,
  use_case   text,
  source     text NOT NULL,
  created_at timestamptz NOT NULL,
  updated_at timestamptz NOT NULL
) ON COMMIT DROP;

\copy waitlist_stage (email, name, company, role, use_case, source, created_at, updated_at) FROM '$IN' WITH CSV HEADER

\echo → staged rows:
SELECT count(*) AS staged FROM waitlist_stage;

\echo → inserting (ON CONFLICT email DO NOTHING):
WITH ins AS (
  INSERT INTO waitlist (email, name, company, role, use_case, source, created_at, updated_at)
  SELECT email, name, company, role, use_case, source, created_at, updated_at
  FROM waitlist_stage
  ON CONFLICT (email) DO NOTHING
  RETURNING 1
)
SELECT count(*) AS inserted FROM ins;

COMMIT;
SQL

POST=$(psql "$BETA_DB_URL" -tAc "SELECT count(*) FROM waitlist")
echo "→ post-import count on target: $POST  (delta: $((POST - PRE)))" >&2
