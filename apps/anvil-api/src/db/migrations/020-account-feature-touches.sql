-- Migration: account_feature_touches for BACT-004 (beta account activity).
--
-- Identity-bound, allowlisted feature-touch markers for customer-success.
-- Distinct from FLEET telemetry_beacons (anonymous install-id path, ADR-107).
-- No install_id, IP, path, argv, or free-form command strings on this table.

CREATE TABLE IF NOT EXISTS account_feature_touches (
  user_id       uuid NOT NULL REFERENCES beta_users(id) ON DELETE CASCADE,
  feature_key   text NOT NULL
                CHECK (feature_key IN ('watch', 'start', 'check', 'auth')),
  first_seen_at timestamptz NOT NULL DEFAULT now(),
  last_seen_at  timestamptz NOT NULL DEFAULT now(),
  touch_count   bigint NOT NULL DEFAULT 1
                CHECK (touch_count >= 1),
  PRIMARY KEY (user_id, feature_key)
);

CREATE INDEX IF NOT EXISTS idx_account_feature_touches_feature_last_seen
  ON account_feature_touches (feature_key, last_seen_at DESC);

CREATE INDEX IF NOT EXISTS idx_account_feature_touches_last_seen
  ON account_feature_touches (last_seen_at DESC);
