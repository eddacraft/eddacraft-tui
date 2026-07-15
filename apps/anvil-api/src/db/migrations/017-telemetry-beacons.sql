-- Migration: fleet telemetry beacon storage (FLEET-005, ADR-107 §3/§6).
--
-- Privacy by construction: the raw beacon row carries ONLY the ADR-107
-- dimension allowlist. There is deliberately NO ip column (ingest never
-- retains IPs) and NO timestamptz column — arrival time coarsens to a date
-- (received_on, DEFAULT current_date). Adding any column here requires a
-- dated amendment to ADR-107.
--
-- Retention (§6): raw rows are kept for the configured window (default 90
-- days, lib/telemetry-retention.ts); the hourly cron sweep rolls expired
-- rows up into the telemetry_daily_* aggregate tables (kept indefinitely)
-- and then deletes them — see rollupAndPurgeExpiredTelemetryBeacons in
-- db/queries.ts. Read access is operator-only and lands with FLEET-007;
-- nothing in this migration grants a public read surface.

-- SET LOCAL is transaction-scoped: this file relies on the migration runner
-- (scripts via src/db/migrate.ts) wrapping it in BEGIN/COMMIT. A manual apply
-- in autocommit mode silently skips the lock timeout.
SET LOCAL lock_timeout = '30s';

CREATE TABLE IF NOT EXISTS telemetry_beacons (
  id                    uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  schema_version        int  NOT NULL,
  install_id            uuid NOT NULL,
  version               text NOT NULL CHECK (char_length(version) <= 64),
  install_method        text NOT NULL CHECK (install_method IN
                          ('homebrew', 'scoop', 'winget', 'cargo_dist',
                           'cargo_install', 'dev_build', 'unknown')),
  platform              text NOT NULL CHECK (char_length(platform) <= 64),
  channel               text NOT NULL CHECK (char_length(channel) <= 32),
  flag_snapshot_version text NOT NULL CHECK (char_length(flag_snapshot_version) <= 64),
  -- Date-coarsened arrival marker (ADR-107 §3). DATE on purpose; never add
  -- a time-of-day or timestamptz column to this table.
  received_on           date NOT NULL DEFAULT current_date
);

-- (feature key, count) usage pairs since the last beacon, one row per key.
-- Cascade keeps the purge a single DELETE on telemetry_beacons.
CREATE TABLE IF NOT EXISTS telemetry_beacon_features (
  id          uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  beacon_id   uuid NOT NULL REFERENCES telemetry_beacons(id) ON DELETE CASCADE,
  feature_key text NOT NULL CHECK (char_length(feature_key) <= 128),
  usage_count int  NOT NULL CHECK (usage_count >= 0)
);

-- Kept-indefinitely daily aggregates (ADR-107 §6), populated by the
-- retention sweep from raw rows leaving the window.
CREATE TABLE IF NOT EXISTS telemetry_daily_installs (
  day            date NOT NULL,
  version        text NOT NULL,
  install_method text NOT NULL,
  platform       text NOT NULL,
  channel        text NOT NULL,
  install_count  int  NOT NULL,
  PRIMARY KEY (day, version, install_method, platform, channel)
);

CREATE TABLE IF NOT EXISTS telemetry_daily_feature_usage (
  day           date   NOT NULL,
  feature_key   text   NOT NULL,
  usage_count   bigint NOT NULL,
  install_count int    NOT NULL,
  PRIMARY KEY (day, feature_key)
);

-- Purge/rollup scan key.
CREATE INDEX IF NOT EXISTS idx_telemetry_beacons_received_on
  ON telemetry_beacons(received_on);
-- Unique-install / retention-cohort queries (FLEET-007 reads).
CREATE INDEX IF NOT EXISTS idx_telemetry_beacons_install_id
  ON telemetry_beacons(install_id, received_on);
CREATE INDEX IF NOT EXISTS idx_telemetry_beacon_features_beacon_id
  ON telemetry_beacon_features(beacon_id);
