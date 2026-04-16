-- Migration: add indexes to support admin-cli audit queries.
-- /admin/audit?actor=X and /admin/user/:email's recentAudit lookup
-- (metadata->>'email') both did sequential scans pre-migration.
-- Index the email expression case-insensitively so historical
-- mixed-case rows still match the lowercased lookup.

CREATE INDEX IF NOT EXISTS idx_audit_log_actor
  ON audit_log(actor);

CREATE INDEX IF NOT EXISTS idx_audit_log_metadata_email_lower
  ON audit_log (LOWER((metadata->>'email')));
