-- Migration: add github_id to beta_users for GitHub device-flow account linking.
--
-- GHCLIAUTH-003 (ADR-066): a GitHub identity is linked to a beta_users row on
-- first login by its stable numeric id. Once stored, github_id is the
-- authoritative match key for returning users (emails and usernames change; the
-- numeric id does not). Email remains the invitation key (waitlist -> approve ->
-- invite); github_id is a sparse, nullable *credential* column, not a second
-- account namespace.
--
-- Additive + nullable: existing rows get github_id = NULL and are unaffected.
-- Postgres treats NULLs as distinct under UNIQUE, so unlinked users never
-- collide; the constraint enforces one beta_users row per GitHub account.

-- Bound the ALTER lock-acquisition window so a deploy applying this during
-- in-flight auth traffic fails fast rather than queuing. Mirrors migration 014.
SET LOCAL lock_timeout = '30s';

-- IDEMPOTENCY: guard the ADD COLUMN so a fresh install (schema.sql already at
-- the post-015 shape) and a re-run are both no-ops.
DO $$
BEGIN
  IF NOT EXISTS (
    SELECT 1 FROM information_schema.columns
    WHERE table_name = 'beta_users' AND column_name = 'github_id'
  ) THEN
    ALTER TABLE beta_users ADD COLUMN github_id bigint UNIQUE;
  END IF;
END $$;
