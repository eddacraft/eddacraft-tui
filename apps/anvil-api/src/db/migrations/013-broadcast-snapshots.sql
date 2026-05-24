-- Migration: generalise send_migration_snapshots into send_broadcast_snapshots
-- so /admin/broadcast can store the template, template props, audience key,
-- and audience params alongside the recipient set. The existing
-- /admin/send-migration handler keeps working unchanged by virtue of the
-- back-compat shim landing in EMAIL-006; the table that records its
-- snapshots just has a wider schema underneath.
--
-- The existing primary key (token) and reap-supporting expires_at index
-- are preserved by the RENAME — Postgres carries both with the table.

ALTER TABLE send_migration_snapshots RENAME TO send_broadcast_snapshots;

ALTER INDEX idx_send_migration_snapshots_expires_at
  RENAME TO idx_send_broadcast_snapshots_expires_at;

-- Drop the source CHECK constraint before backfill: the column is moving
-- into audience_params (jsonb) and the check no longer applies. Named
-- constraints are not introspectable across PG versions cheaply, so we
-- match the auto-generated check name via information_schema.
ALTER TABLE send_broadcast_snapshots
  DROP CONSTRAINT IF EXISTS send_migration_snapshots_source_check;

-- Add the new columns with temporary defaults sufficient for the
-- backfill of any rows present from the send-migration era. New rows
-- must always supply explicit values (defaults dropped below).
ALTER TABLE send_broadcast_snapshots
  ADD COLUMN template        TEXT  NOT NULL DEFAULT 'waitlist-migration',
  ADD COLUMN template_props  JSONB NOT NULL DEFAULT '{}'::jsonb,
  ADD COLUMN audience_key    TEXT  NOT NULL DEFAULT 'waitlist:source',
  ADD COLUMN audience_params JSONB NOT NULL DEFAULT '{}'::jsonb;

-- Backfill audience_params from the existing source column so any
-- pre-rename rows resolve to the same recipient cohort under the
-- generalised model.
UPDATE send_broadcast_snapshots
   SET audience_params = jsonb_build_object('source', source);

ALTER TABLE send_broadcast_snapshots DROP COLUMN source;

-- Defaults existed only for the backfill. New inserts come from the
-- application layer which is required to supply every field.
ALTER TABLE send_broadcast_snapshots
  ALTER COLUMN template        DROP DEFAULT,
  ALTER COLUMN template_props  DROP DEFAULT,
  ALTER COLUMN audience_key    DROP DEFAULT,
  ALTER COLUMN audience_params DROP DEFAULT;
