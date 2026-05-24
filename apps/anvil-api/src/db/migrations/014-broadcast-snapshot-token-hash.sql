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

-- Bound the ALTER TABLE lock-acquisition window so a deploy applying
-- this migration during in-flight /admin/broadcast traffic fails fast
-- rather than queuing behind a long-held lock. Mirrors migration 013.
SET LOCAL lock_timeout = '30s';

-- IDEMPOTENCY: ALTER TABLE ... IF EXISTS only guards the table, not the
-- column. On a fresh install where schema.sql already creates
-- `token_hash` (post-014 shape), the RENAME would fail with
-- 'column "token" does not exist'. Guard via information_schema lookup.
DO $$
BEGIN
  IF EXISTS (
    SELECT 1 FROM information_schema.columns
    WHERE table_name = 'send_broadcast_snapshots' AND column_name = 'token'
  ) THEN
    ALTER TABLE send_broadcast_snapshots RENAME COLUMN token TO token_hash;
  END IF;
END $$;

-- The primary-key constraint follows the column rename automatically;
-- no separate DROP/ADD needed. The expires_at reap index is unaffected.
