#!/usr/bin/env bash
# DBCON-001: snapshot a Neon project via pg_dump into scripts/dbcon/snapshots/.
#
# Produces a gzipped plain-SQL dump named "<tag>-<ISO8601>.sql.gz" so the
# origin is always obvious and multiple snapshots sort chronologically.
#
# The snapshot includes schema + data (full pg_dump) so it can be replayed
# standalone into a fresh Postgres for verification or disaster recovery.
#
# Usage:
#   ./snapshot-db.sh <DB_URL> <tag>
#
# Example:
#   ./snapshot-db.sh "$EDDACRAFT_WEB_URL" eddacraft-web
set -euo pipefail

if [[ $# -ne 2 ]]; then
  echo "usage: $0 <DB_URL> <tag>" >&2
  exit 2
fi

DB_URL="$1"
TAG="$2"

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/snapshots"
mkdir -p "$DIR"

TS="$(date -u +%Y%m%dT%H%M%SZ)"
OUT="$DIR/${TAG}-${TS}.sql.gz"

echo "→ snapshotting $TAG via pg_dump …" >&2
pg_dump "$DB_URL" --no-owner --no-privileges --quote-all-identifiers \
  | gzip -9 > "$OUT"

SIZE=$(du -h "$OUT" | awk '{print $1}')
echo "→ wrote $OUT ($SIZE)" >&2
