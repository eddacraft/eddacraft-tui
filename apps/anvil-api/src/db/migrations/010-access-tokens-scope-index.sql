-- Migration: add the access_tokens indexes that support
-- findActiveScopesForUser. Two indexes here:
--   * idx_access_tokens_user_id — a single-column index that exists in
--     schema.sql for fresh installs but was never written as a migration,
--     so production databases stood up before that schema.sql change run
--     a sequential scan on access_tokens for every /session/refresh,
--     /auth/device/poll, /auth/github/callback, and /auth/otp/verify
--     call. This migration backfills it.
--   * idx_access_tokens_active_scope_lookup — a partial composite that is
--     net-new in this release (also added to schema.sql alongside the
--     migration so fresh installs match production). The partial form
--     skips revoked rows; the (user_id, created_at DESC) leading edge
--     covers the query's filter and ORDER BY.
-- At launch traffic the absence of either index would saturate Neon
-- connection slots and produce visible auth latency.
--
-- Composite index aligns with the query shape — filter by user_id, drop
-- revoked / expired rows, take the most recent first. Using
-- CONCURRENTLY would be ideal but Neon migrations run inside a
-- transaction so the simpler form is shipped here; access_tokens is
-- small enough at current beta volume that the brief lock is benign.

CREATE INDEX IF NOT EXISTS idx_access_tokens_user_id
  ON access_tokens(user_id);

CREATE INDEX IF NOT EXISTS idx_access_tokens_active_scope_lookup
  ON access_tokens(user_id, created_at DESC)
  WHERE revoked_at IS NULL;
