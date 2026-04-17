-- Migration: audit_log.auth_method column.
-- Differentiates shared-admin-key rows from per-operator-key rows during the
-- dual-auth rollout. Existing rows (written before per-operator keys existed)
-- are backfilled to 'shared'. New rows are written by admin-auth middleware.

ALTER TABLE audit_log
  ADD COLUMN IF NOT EXISTS auth_method TEXT NOT NULL DEFAULT 'shared'
  CHECK (auth_method IN ('shared', 'per_operator'));

-- Filtering audit rows by auth_method is a common rollout-tracking query.
CREATE INDEX IF NOT EXISTS idx_audit_log_auth_method
  ON audit_log(auth_method);
