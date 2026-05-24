-- Migration: generalise send_migration_snapshots into send_broadcast_snapshots
-- so /admin/broadcast can store the template, template props, audience key,
-- and audience params alongside the recipient set. The existing
-- /admin/send-migration handler keeps working unchanged by virtue of the
-- back-compat shim landing in EMAIL-006; the table that records its
-- snapshots just has a wider schema underneath.
--
-- The existing primary key (token) and reap-supporting expires_at index
-- are preserved by the RENAME — Postgres carries both with the table.
--
-- IDEMPOTENCY: this migration must succeed both incrementally (after 006
-- created the legacy table) and on fresh-install environments where
-- schema.sql created `send_broadcast_snapshots` directly. Every
-- statement uses IF EXISTS / IF NOT EXISTS guards so a fresh-schema run
-- becomes a no-op without ALTER TABLE errors.

ALTER TABLE IF EXISTS send_migration_snapshots RENAME TO send_broadcast_snapshots;

ALTER INDEX IF EXISTS idx_send_migration_snapshots_expires_at
  RENAME TO idx_send_broadcast_snapshots_expires_at;

-- Drop the source CHECK constraint before backfill: the column is moving
-- into audience_params (jsonb) and the check no longer applies. Named
-- constraints are not introspectable across PG versions cheaply, so we
-- match the auto-generated check name via information_schema.
ALTER TABLE IF EXISTS send_broadcast_snapshots
  DROP CONSTRAINT IF EXISTS send_migration_snapshots_source_check;

-- Add the new columns with temporary defaults sufficient for the
-- backfill of any rows present from the send-migration era. New rows
-- must always supply explicit values (defaults dropped below).
-- IF NOT EXISTS makes this safe on fresh installs where schema.sql
-- already created the columns.
ALTER TABLE IF EXISTS send_broadcast_snapshots
  ADD COLUMN IF NOT EXISTS template        TEXT  NOT NULL DEFAULT 'waitlist-migration',
  ADD COLUMN IF NOT EXISTS template_props  JSONB NOT NULL DEFAULT '{}'::jsonb,
  ADD COLUMN IF NOT EXISTS audience_key    TEXT  NOT NULL DEFAULT 'waitlist:source',
  ADD COLUMN IF NOT EXISTS audience_params JSONB NOT NULL DEFAULT '{}'::jsonb;

-- Backfill audience_params from the existing source column so any
-- pre-rename rows resolve to the same recipient cohort under the
-- generalised model. The `WHERE` clause guards the fresh-install path
-- where the `source` column was never created; information_schema lookup
-- is the only safe way to gate the UPDATE on column existence.
DO $$
BEGIN
  IF EXISTS (
    SELECT 1 FROM information_schema.columns
    WHERE table_name = 'send_broadcast_snapshots' AND column_name = 'source'
  ) THEN
    EXECUTE 'UPDATE send_broadcast_snapshots
             SET audience_params = jsonb_build_object(''source'', source)';
  END IF;
END $$;

ALTER TABLE IF EXISTS send_broadcast_snapshots DROP COLUMN IF EXISTS source;

-- Defaults existed only for the backfill. New inserts come from the
-- application layer which is required to supply every field. DROP
-- DEFAULT on a column with no default is a no-op in Postgres, so this
-- is safe on the fresh-install path too.
ALTER TABLE IF EXISTS send_broadcast_snapshots
  ALTER COLUMN template        DROP DEFAULT,
  ALTER COLUMN template_props  DROP DEFAULT,
  ALTER COLUMN audience_key    DROP DEFAULT,
  ALTER COLUMN audience_params DROP DEFAULT;
