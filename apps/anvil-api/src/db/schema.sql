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
             CHECK (status IN ('active', 'pending', 'suspended', 'banned')),
  notes      text,
  -- GHCLIAUTH-003 (ADR-066): GitHub numeric id, linked on first GitHub login.
  -- Sparse + nullable; authoritative match key for returning users once set.
  -- Email stays the invitation key. NULLs are distinct under UNIQUE.
  github_id  bigint UNIQUE,
  created_at timestamptz NOT NULL DEFAULT now(),
  updated_at timestamptz NOT NULL DEFAULT now()
);

-- Access tokens table (stores SHA-256 hashes only)
CREATE TABLE access_tokens (
  id         uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id    uuid NOT NULL REFERENCES beta_users(id) ON DELETE CASCADE,
  token_hash text UNIQUE NOT NULL,
  scopes     text[] NOT NULL DEFAULT '{beta}',
  is_edict   boolean NOT NULL DEFAULT false,
  expires_at timestamptz NOT NULL,
  revoked_at timestamptz,
  created_at timestamptz NOT NULL DEFAULT now()
);

-- Audit log for all admin actions
CREATE TABLE audit_log (
  id          uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  action      text NOT NULL,
  actor       text NOT NULL,
  metadata    jsonb NOT NULL DEFAULT '{}',
  created_at  timestamptz NOT NULL DEFAULT now(),
  -- Mirrors migration 009-audit-log-auth-method.sql so fresh-install
  -- environments (CI, ephemeral preview deployments, disaster recovery)
  -- have the column from row zero. Per ADMINCLIH dual-auth rollout.
  auth_method text NOT NULL DEFAULT 'shared'
              CHECK (auth_method IN ('shared', 'per_operator'))
);

-- Waitlist table
CREATE TABLE waitlist (
  id         serial PRIMARY KEY,
  email      citext UNIQUE NOT NULL,
  name       text,
  company    text,
  role       text,
  use_case   text,
  source     text NOT NULL DEFAULT 'website',
  created_at timestamptz NOT NULL DEFAULT now(),
  updated_at timestamptz NOT NULL DEFAULT now()
);

-- Device code flow state (BAUTH-001)
CREATE TABLE device_codes (
  id            uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id       uuid REFERENCES beta_users(id) ON DELETE CASCADE,
  user_code     text UNIQUE NOT NULL,
  poll_token    text UNIQUE NOT NULL,
  confirmed_at  timestamptz,
  expires_at    timestamptz NOT NULL,
  last_polled_at timestamptz,
  attempts      int NOT NULL DEFAULT 0,
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

-- Snapshot table for the broadcast preview/send contract (EMAIL-002,
-- generalised from the ADMINCLIH-001 send-migration table by migration
-- 013). The dry-run handler inserts a row; the real-send handler
-- atomically consumes it and compares the recorded recipient set
-- against a fresh resolver run to detect cohort drift. Rows live for
-- the TTL (10 minutes) then are lazily reaped by the same handler.
CREATE TABLE send_broadcast_snapshots (
  -- SHA-256(raw token), produced by lib/token.ts:hashToken with the
  -- TOKEN_PEPPER env var. Mirrors the access_tokens / refresh_tokens
  -- at-rest hashing convention. The raw token is returned to the
  -- operator only once, by insertBroadcastSnapshot.
  token_hash        text PRIMARY KEY,
  template          text NOT NULL,
  template_props    jsonb NOT NULL,
  audience_key      text NOT NULL,
  audience_params   jsonb NOT NULL,
  recipients        jsonb NOT NULL,
  created_by_actor  text NOT NULL,
  created_at        timestamptz NOT NULL DEFAULT now(),
  expires_at        timestamptz NOT NULL,
  -- NULL = unconsumed; set once on real-send (consume-once invariant
  -- enforced atomically by consumeBroadcastSnapshot's UPDATE ... WHERE
  -- consumed_at IS NULL).
  consumed_at       timestamptz
);

-- Indexes
CREATE INDEX idx_access_tokens_user_id ON access_tokens(user_id);
CREATE INDEX idx_access_tokens_token_hash ON access_tokens(token_hash);
-- Mirrors migration 010-access-tokens-scope-index.sql so fresh-install
-- environments (CI, ephemeral preview deployments, disaster recovery)
-- get the partial composite index findActiveScopesForUser uses on the
-- auth hot path. Migration 010 backfills onto existing prod DBs; this
-- entry keeps schema.sql authoritative for new ones.
CREATE INDEX idx_access_tokens_active_scope_lookup
  ON access_tokens(user_id, created_at DESC)
  WHERE revoked_at IS NULL;
CREATE INDEX idx_audit_log_action ON audit_log(action);
CREATE INDEX idx_audit_log_actor ON audit_log(actor);
CREATE INDEX idx_audit_log_created_at ON audit_log(created_at);
CREATE INDEX idx_audit_log_metadata_email_lower ON audit_log (LOWER((metadata->>'email')));
-- Mirrors migration 009-audit-log-auth-method.sql.
CREATE INDEX idx_audit_log_auth_method ON audit_log(auth_method);
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
CREATE INDEX idx_send_broadcast_snapshots_expires_at
  ON send_broadcast_snapshots(expires_at);

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
