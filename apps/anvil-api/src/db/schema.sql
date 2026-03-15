-- Beta Access System Schema
-- Requires: citext extension, pgcrypto extension

CREATE EXTENSION IF NOT EXISTS citext;
CREATE EXTENSION IF NOT EXISTS pgcrypto;

-- Beta users table
CREATE TABLE beta_users (
  id         uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  email      citext UNIQUE NOT NULL,
  name       text,
  status     text NOT NULL DEFAULT 'active'
             CHECK (status IN ('active', 'suspended', 'banned')),
  notes      text,
  created_at timestamptz NOT NULL DEFAULT now(),
  updated_at timestamptz NOT NULL DEFAULT now()
);

-- Access tokens table (stores SHA-256 hashes only)
CREATE TABLE access_tokens (
  id         uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id    uuid NOT NULL REFERENCES beta_users(id) ON DELETE CASCADE,
  token_hash text UNIQUE NOT NULL,
  scopes     text[] NOT NULL DEFAULT '{beta}',
  expires_at timestamptz NOT NULL,
  revoked_at timestamptz,
  created_at timestamptz NOT NULL DEFAULT now()
);

-- Audit log for all admin actions
CREATE TABLE audit_log (
  id         uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  action     text NOT NULL,
  actor      text NOT NULL,
  metadata   jsonb NOT NULL DEFAULT '{}',
  created_at timestamptz NOT NULL DEFAULT now()
);

-- Waitlist table
CREATE TABLE waitlist (
  id         serial PRIMARY KEY,
  email      citext UNIQUE NOT NULL,
  source     text NOT NULL DEFAULT 'website',
  created_at timestamptz NOT NULL DEFAULT now(),
  updated_at timestamptz NOT NULL DEFAULT now()
);

-- Device code flow state (BAUTH-001)
CREATE TABLE device_codes (
  id            uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id       uuid NOT NULL REFERENCES beta_users(id) ON DELETE CASCADE,
  user_code     text UNIQUE NOT NULL,
  poll_token    text UNIQUE NOT NULL,
  confirmed_at  timestamptz,
  expires_at    timestamptz NOT NULL,
  created_at    timestamptz NOT NULL DEFAULT now()
);

-- Email OTP state (BAUTH-001)
CREATE TABLE otp_codes (
  id            uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id       uuid NOT NULL REFERENCES beta_users(id) ON DELETE CASCADE,
  code_hash     text NOT NULL,
  attempts      int NOT NULL DEFAULT 0,
  expires_at    timestamptz NOT NULL,
  consumed_at   timestamptz,
  created_at    timestamptz NOT NULL DEFAULT now()
);

-- Refresh token chain with family-based theft detection (BAUTH-001)
CREATE TABLE refresh_tokens (
  id            uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id       uuid NOT NULL REFERENCES beta_users(id) ON DELETE CASCADE,
  token_hash    text UNIQUE NOT NULL,
  family_id     uuid NOT NULL,
  expires_at    timestamptz NOT NULL,
  revoked_at    timestamptz,
  consumed_at   timestamptz,
  created_at    timestamptz NOT NULL DEFAULT now()
);

-- Indexes
CREATE INDEX idx_access_tokens_user_id ON access_tokens(user_id);
CREATE INDEX idx_access_tokens_token_hash ON access_tokens(token_hash);
CREATE INDEX idx_audit_log_action ON audit_log(action);
CREATE INDEX idx_audit_log_created_at ON audit_log(created_at);
CREATE INDEX idx_device_codes_user_code ON device_codes(user_code);
CREATE INDEX idx_device_codes_poll_token ON device_codes(poll_token);
CREATE INDEX idx_device_codes_user_id ON device_codes(user_id);
CREATE INDEX idx_device_codes_expires_at ON device_codes(expires_at);
CREATE INDEX idx_otp_codes_user_id ON otp_codes(user_id);
CREATE INDEX idx_otp_codes_expires_at ON otp_codes(expires_at);
CREATE INDEX idx_refresh_tokens_token_hash ON refresh_tokens(token_hash);
CREATE INDEX idx_refresh_tokens_family_id ON refresh_tokens(family_id);
CREATE INDEX idx_refresh_tokens_user_id ON refresh_tokens(user_id);
CREATE INDEX idx_refresh_tokens_expires_at ON refresh_tokens(expires_at);

-- Auto-update updated_at trigger
CREATE OR REPLACE FUNCTION update_updated_at()
RETURNS TRIGGER AS $$
BEGIN
  NEW.updated_at = now();
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER beta_users_updated_at
  BEFORE UPDATE ON beta_users
  FOR EACH ROW
  EXECUTE FUNCTION update_updated_at();

CREATE TRIGGER waitlist_updated_at
  BEFORE UPDATE ON waitlist
  FOR EACH ROW
  EXECUTE FUNCTION update_updated_at();
