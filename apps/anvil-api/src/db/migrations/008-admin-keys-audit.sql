-- Migration: append-only audit trail for admin_keys mutations.
-- Every INSERT and every revoked_at UPDATE on admin_keys is paired with an
-- admin_keys_audit row recording the Pulumi commit SHA that authorised the
-- change and the actor who triggered the pipeline. This is the two-person-rule
-- evidence for key provisioning.

CREATE TABLE IF NOT EXISTS admin_keys_audit (
  id                 UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  admin_key_id       UUID NOT NULL REFERENCES admin_keys(id),
  action             TEXT NOT NULL CHECK (action IN ('created', 'revoked')),
  change_actor       TEXT NOT NULL,
  pulumi_commit_sha  TEXT NOT NULL,
  note               TEXT,
  occurred_at        TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_admin_keys_audit_admin_key_id
  ON admin_keys_audit(admin_key_id);

CREATE INDEX IF NOT EXISTS idx_admin_keys_audit_occurred_at
  ON admin_keys_audit(occurred_at DESC);
