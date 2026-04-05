-- Migration: add 'pending' to beta_users status CHECK constraint
-- Required for DOCSAUTH: GitHub OAuth creates users with status = 'pending'
-- Safe to run multiple times (drops and re-creates the constraint)

ALTER TABLE beta_users DROP CONSTRAINT IF EXISTS beta_users_status_check;
ALTER TABLE beta_users ADD CONSTRAINT beta_users_status_check
  CHECK (status IN ('active', 'pending', 'suspended', 'banned'));
