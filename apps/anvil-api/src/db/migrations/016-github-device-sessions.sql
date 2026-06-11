-- Migration: create github_device_sessions for the brokered GitHub device flow.
-- GHCLIAUTH-004 (ADR-066): anvil-api brokers the GitHub Device Authorization
-- Grant (RFC 8628) server-side; Vercel serverless means session state must be
-- DB-backed. A dedicated table — not device_codes, whose user_code UNIQUE NOT
-- NULL + start-time user_id invariants the GitHub flow structurally breaks.
--
-- At-rest model: poll_token is stored only as a hash (lib/token.ts hashToken).
-- The GitHub device_code is stored ENCRYPTED, not hashed — the poll broker
-- must recover the plaintext for the token exchange (RFC 8628 §3.4). The key
-- is derived from the client-held poll_token (lib/github-device-crypto.ts), so
-- a DB dump alone recovers neither. No user column by design: the bound user
-- is derived solely from the GitHub token at poll-confirmation time.
--
-- minted_at / minted_session_enc are reserved for the poll path (GHCLIAUTH-005)
-- "mint exactly once, re-returnable within TTL" semantics.

-- SET LOCAL is transaction-scoped: this file relies on the migration runner
-- (scripts via src/db/migrate.ts) wrapping it in BEGIN/COMMIT. A manual apply
-- in autocommit mode silently skips the lock timeout.
SET LOCAL lock_timeout = '30s';

CREATE TABLE IF NOT EXISTS github_device_sessions (
  id                       uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  poll_token_hash          text UNIQUE NOT NULL,
  github_device_code_enc   text NOT NULL,
  interval_s               int NOT NULL,
  expires_at               timestamptz NOT NULL,
  last_polled_at           timestamptz,
  minted_at                timestamptz,
  minted_session_enc       text,
  created_at               timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_github_device_sessions_expires_at
  ON github_device_sessions(expires_at);
