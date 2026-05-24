-- Migration: hash broadcast snapshot tokens at rest.
--
-- The previous schema stored the raw bearer token as the PRIMARY KEY of
-- send_broadcast_snapshots — out of step with access_tokens and
-- refresh_tokens, both of which store SHA-256(token). A read-only DB
-- leak (backup, replica, slow-query log, pg_stat_activity) would have
-- yielded usable preview tokens.
--
-- Threat model is mild — the consume endpoint is gated by adminAuth so
-- the leaked token is only useful to an already-authenticated caller —
-- but the asymmetry with the existing token-hash convention is real
-- pattern erosion. This migration brings the snapshot table in line.
--
-- In-flight snapshots from before this deploy become unusable because
-- their stored raw token won't match SHA-256(client-supplied token).
-- The 10-minute TTL bounds the window: any operator mid-preview at
-- deploy time has to re-run --dry-run after deploy completes. Worst
-- case is an extra ~10 min until natural reap clears the dead rows.

ALTER TABLE IF EXISTS send_broadcast_snapshots
  RENAME COLUMN token TO token_hash;

-- The primary-key constraint follows the column rename automatically;
-- no separate DROP/ADD needed. The expires_at reap index is unaffected.
