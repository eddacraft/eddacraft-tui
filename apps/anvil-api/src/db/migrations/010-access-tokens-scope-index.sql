-- Migration: backfill the access_tokens index that supports
-- findActiveScopesForUser. The index existed in schema.sql (fresh-install
-- DDL) but was never written as a migration, so production databases
-- that pre-date it take a sequential scan on access_tokens for every
-- /session/refresh, /auth/device/poll, /auth/github/callback, and
-- /auth/otp/verify call. At launch traffic that saturates Neon
-- connection slots and produces visible auth latency.
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
