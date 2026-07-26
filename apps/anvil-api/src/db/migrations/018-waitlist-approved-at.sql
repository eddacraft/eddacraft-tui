-- Migration: durable operator-grant timestamp on waitlist.
--
-- Until now, admin "approved" was derived by LEFT JOIN beta_users — any matching
-- beta_users row (including pending GitHub OAuth signups) looked approved, and
-- approved_at was proxied from beta_users.created_at. That made Neon hard to
-- filter and conflated "user row exists" with "admin admitted them".
--
-- approved_at is set only by admin approve / invite (first grant wins via
-- COALESCE). Revoke leaves it set (admission history). Signup leaves it NULL.
--
-- Backfill: active beta users who already share a waitlist email. Pending-only
-- beta_users rows (e.g. blocked GitHub OAuth) stay NULL so they remain in the
-- pending queue until an operator invites.

SET LOCAL lock_timeout = '30s';

DO $$
BEGIN
  IF NOT EXISTS (
    SELECT 1 FROM information_schema.columns
    WHERE table_schema = current_schema()
      AND table_name = 'waitlist'
      AND column_name = 'approved_at'
  ) THEN
    ALTER TABLE waitlist ADD COLUMN approved_at timestamptz NULL;
  END IF;
END $$;

-- First-grant stamp for people already active in beta. Prefer the earliest
-- admin grant audit when present; fall back to beta_users.created_at.
UPDATE waitlist w
SET
  approved_at = COALESCE(
    (
      SELECT MIN(a.created_at)
      FROM audit_log a
      WHERE a.action IN ('user.approved', 'user.invited')
        -- citext equality: audit metadata is plain text and may differ in case
        AND (a.metadata ->> 'email')::citext = w.email
    ),
    bu.created_at
  ),
  updated_at = NOW()
FROM beta_users bu
WHERE bu.email = w.email
  AND bu.status = 'active'
  AND w.approved_at IS NULL;
