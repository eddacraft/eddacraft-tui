#!/usr/bin/env bash
# DBCON-003: verify every source email is present on the target.
#
# Run once per source — the target accumulates across imports, so EXTRA is
# informational only (it reflects rows from other sources or prior imports).
# MISSING must be 0 for the source being checked.
#
# Required env:
#   WAITLIST_DB_URL — source Neon connection string (the one being checked)
#   BETA_DB_URL     — target Neon connection string (anvil-api-prod)
set -euo pipefail

: "${WAITLIST_DB_URL:?WAITLIST_DB_URL is required (source)}"
: "${BETA_DB_URL:?BETA_DB_URL is required (target)}"

SRC=$(psql "$WAITLIST_DB_URL" -tAc "SELECT count(*) FROM waitlist")
TGT=$(psql "$BETA_DB_URL"     -tAc "SELECT count(*) FROM waitlist")

echo "source: $SRC rows"
echo "target: $TGT rows"
echo ""

TMPDIR=$(mktemp -d)
trap 'rm -rf "$TMPDIR"' EXIT

# Normalise (lower) and byte-sort so `comm` compares deterministically —
# `email` is citext (case-insensitive) and DB collation may differ from the
# local locale, either of which can cause spurious MISSING/EXTRA output.
psql "$WAITLIST_DB_URL" -tAc "SELECT lower(email) FROM waitlist" | LC_ALL=C sort > "$TMPDIR/src"
psql "$BETA_DB_URL"     -tAc "SELECT lower(email) FROM waitlist" | LC_ALL=C sort > "$TMPDIR/tgt"

MISSING=$(comm -23 "$TMPDIR/src" "$TMPDIR/tgt" | wc -l | tr -d ' ')
EXTRA=$(  comm -13 "$TMPDIR/src" "$TMPDIR/tgt" | wc -l | tr -d ' ')

echo "emails in SOURCE but not TARGET: $MISSING  (must be 0)"
echo "emails in TARGET but not SOURCE: $EXTRA   (informational — other sources / prior imports)"

if [[ "$MISSING" -ne 0 ]]; then
  echo ""
  echo "→ first 10 missing emails:"
  comm -23 "$TMPDIR/src" "$TMPDIR/tgt" | head -10
  exit 1
fi
