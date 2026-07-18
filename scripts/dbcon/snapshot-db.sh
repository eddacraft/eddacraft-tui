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

# Guard against path traversal / weird characters in TAG — it becomes part of
# the output filename, so keep it to a conservative slug charset.
if ! [[ "$TAG" =~ ^[a-z0-9][a-z0-9-]*$ ]]; then
  echo "tag must match [a-z0-9][a-z0-9-]* (got: $TAG)" >&2
  exit 2
fi

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/snapshots"
mkdir -p "$DIR"

TS="$(date -u +%Y%m%dT%H%M%SZ)"
OUT="$DIR/${TAG}-${TS}.sql.gz"
TMP_OUT="$DIR/.${TAG}-${TS}.$$.sql.gz.tmp"

cleanup_tmp() {
  rm -f "$TMP_OUT"
}
trap cleanup_tmp EXIT

echo "→ snapshotting $TAG via pg_dump …" >&2
# Write via temp + rename so a failed dump/gzip cannot leave a final-named
# partial .sql.gz that looks like a usable disaster-recovery snapshot.
pg_dump "$DB_URL" --no-owner --no-privileges --quote-all-identifiers \
  | gzip -9 > "$TMP_OUT"
mv -f "$TMP_OUT" "$OUT"
trap - EXIT

SIZE=$(du -h "$OUT" | awk '{print $1}')
echo "→ wrote $OUT ($SIZE)" >&2
