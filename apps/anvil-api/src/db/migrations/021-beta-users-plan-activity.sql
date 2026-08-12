-- Migration: beta_users plan + last_activity_at for BACT-008 (beta account
-- activity phase 2, ADR-121).
--
-- Adds a durable `plan` name (closed set, only 'beta' initially) mapping to
-- catalogue audience `plan-beta`, and `last_activity_at` / `last_activity_kind`
-- so token-era users who never mint a fresh interactive session (session
-- refresh, authenticated feature-touch) still show up in DAA. Interactive
-- login keeps stamping via BACT-002's `first_login_at` / `last_login_at` and
-- also advances activity (kind `login`); refresh and feature-touch advance
-- *only* activity, never login stamps. Invite/approve set `plan` via the
-- column DEFAULT and never stamp login or activity.
--
-- Additive + backward tolerant: existing rows get `plan = 'beta'` (the only
-- legal value) and NULL activity (nothing recorded under this model yet).

SET LOCAL lock_timeout = '30s';

DO $$
BEGIN
  IF NOT EXISTS (
    SELECT 1 FROM information_schema.columns
    WHERE table_name = 'beta_users' AND column_name = 'plan'
  ) THEN
    ALTER TABLE beta_users ADD COLUMN plan text NOT NULL DEFAULT 'beta'
      CHECK (plan IN ('beta'));
  END IF;

  IF NOT EXISTS (
    SELECT 1 FROM information_schema.columns
    WHERE table_name = 'beta_users' AND column_name = 'last_activity_at'
  ) THEN
    ALTER TABLE beta_users ADD COLUMN last_activity_at timestamptz;
  END IF;

  IF NOT EXISTS (
    SELECT 1 FROM information_schema.columns
    WHERE table_name = 'beta_users' AND column_name = 'last_activity_kind'
  ) THEN
    ALTER TABLE beta_users ADD COLUMN last_activity_kind text
      CHECK (
        last_activity_kind IS NULL
        OR last_activity_kind IN ('login', 'refresh', 'feature')
      );
  END IF;
END $$;

CREATE INDEX IF NOT EXISTS idx_beta_users_last_activity_at
  ON beta_users (last_activity_at DESC NULLS LAST);
