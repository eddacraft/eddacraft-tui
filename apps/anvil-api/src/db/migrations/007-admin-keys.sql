-- Migration: per-operator admin keys.
-- Each row maps a hashed bearer token to an operator identity. The middleware
-- hashes the presented bearer with HMAC-SHA-256 keyed by a server-side pepper
-- (not in the DB) and performs a single indexed SELECT on hashed_key.
--
-- Append-only from the app: provisioning (INSERT) and revocation (UPDATE of
-- revoked_at) only. No DELETEs. All mutations are paired with a row in
-- admin_keys_audit (migration 008) recording the Pulumi commit SHA and the
-- change actor.

CREATE TABLE IF NOT EXISTS admin_keys (
  id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  hashed_key    TEXT UNIQUE NOT NULL,
  actor_email   TEXT NOT NULL,
  note          TEXT,
  created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
  revoked_at    TIMESTAMPTZ
);

-- Lookup path is hashed_key equality; the UNIQUE constraint already creates a
-- btree index, so no separate index is needed for that. Add an index for
-- actor_email so revocation / rotation queries ("all active keys for X") don't
-- sequential-scan as the table grows.
CREATE INDEX IF NOT EXISTS idx_admin_keys_actor_email
  ON admin_keys(actor_email);
