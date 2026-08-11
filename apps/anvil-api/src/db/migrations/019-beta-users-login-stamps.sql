-- Migration: beta_users login stamps for BACT-002 (beta account activity).
--
-- Records first/last interactive login so operators can answer "has this
-- invitee logged in?" without scanning refresh_tokens. Invite/approve access
-- token mint does NOT stamp these columns — only interactive session mint
-- paths (GitHub OAuth, GitHub device, OTP, legacy device confirm) do.
--
-- Additive + nullable: existing rows stay NULL (never logged in / pre-BACT).

SET LOCAL lock_timeout = '30s';

DO $$
BEGIN
  IF NOT EXISTS (
    SELECT 1 FROM information_schema.columns
    WHERE table_name = 'beta_users' AND column_name = 'first_login_at'
  ) THEN
    ALTER TABLE beta_users ADD COLUMN first_login_at timestamptz;
  END IF;

  IF NOT EXISTS (
    SELECT 1 FROM information_schema.columns
    WHERE table_name = 'beta_users' AND column_name = 'last_login_at'
  ) THEN
    ALTER TABLE beta_users ADD COLUMN last_login_at timestamptz;
  END IF;

  IF NOT EXISTS (
    SELECT 1 FROM information_schema.columns
    WHERE table_name = 'beta_users' AND column_name = 'last_login_method'
  ) THEN
    ALTER TABLE beta_users ADD COLUMN last_login_method text
      CHECK (
        last_login_method IS NULL
        OR last_login_method IN ('github', 'otp', 'device')
      );
  END IF;
END $$;

CREATE INDEX IF NOT EXISTS idx_beta_users_last_login_at
  ON beta_users (last_login_at DESC NULLS LAST);
