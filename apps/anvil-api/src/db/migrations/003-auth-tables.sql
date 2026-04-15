-- Migration: create device_codes, otp_codes, refresh_tokens tables
-- Required for BAUTH-001: device code auth flow, email OTP, refresh tokens

CREATE TABLE IF NOT EXISTS device_codes (
  id             uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id        uuid REFERENCES beta_users(id) ON DELETE CASCADE,
  user_code      text UNIQUE NOT NULL,
  poll_token     text UNIQUE NOT NULL,
  confirmed_at   timestamptz,
  expires_at     timestamptz NOT NULL,
  last_polled_at timestamptz,
  created_at     timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS otp_codes (
  id            uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id       uuid NOT NULL REFERENCES beta_users(id) ON DELETE CASCADE,
  code_hash     text NOT NULL,
  attempts      int NOT NULL DEFAULT 0,
  expires_at    timestamptz NOT NULL,
  consumed_at   timestamptz,
  created_at    timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS refresh_tokens (
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
CREATE INDEX IF NOT EXISTS idx_device_codes_user_code ON device_codes(user_code);
CREATE INDEX IF NOT EXISTS idx_device_codes_poll_token ON device_codes(poll_token);
CREATE INDEX IF NOT EXISTS idx_device_codes_user_id ON device_codes(user_id);
CREATE INDEX IF NOT EXISTS idx_device_codes_expires_at ON device_codes(expires_at);
CREATE INDEX IF NOT EXISTS idx_otp_codes_user_id ON otp_codes(user_id);
CREATE INDEX IF NOT EXISTS idx_otp_codes_expires_at ON otp_codes(expires_at);
CREATE INDEX IF NOT EXISTS idx_refresh_tokens_token_hash ON refresh_tokens(token_hash);
CREATE INDEX IF NOT EXISTS idx_refresh_tokens_family_id ON refresh_tokens(family_id);
CREATE INDEX IF NOT EXISTS idx_refresh_tokens_user_id ON refresh_tokens(user_id);
CREATE INDEX IF NOT EXISTS idx_refresh_tokens_expires_at ON refresh_tokens(expires_at);
