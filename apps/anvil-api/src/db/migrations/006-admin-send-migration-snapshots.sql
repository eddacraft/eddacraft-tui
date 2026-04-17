-- Migration: snapshot table for the send-migration preview/send contract.
-- The dry-run handler inserts a row; the real-send handler atomically
-- consumes it and compares the recorded recipient set against a fresh
-- query to detect cohort drift. Rows live for the TTL (10 minutes) then
-- are lazily reaped by the same handler.

CREATE TABLE IF NOT EXISTS send_migration_snapshots (
  token             TEXT PRIMARY KEY,
  source            TEXT NOT NULL,
  recipients        JSONB NOT NULL,
  created_by_actor  TEXT NOT NULL,
  created_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
  expires_at        TIMESTAMPTZ NOT NULL,
  consumed_at       TIMESTAMPTZ
);

-- Supports the lazy-reap sweep that runs on insert.
CREATE INDEX IF NOT EXISTS idx_send_migration_snapshots_expires_at
  ON send_migration_snapshots(expires_at);
