-- Migration: mark user-facing early-access edicts on access_tokens.
--
-- Edicts are revokable long-lived early-access credentials issued after the
-- waitlist-to-beta transition. Keeping this as token metadata lets operators
-- query edict holders without turning edict into a separate entitlement model.

ALTER TABLE access_tokens
  ADD COLUMN IF NOT EXISTS is_edict boolean NOT NULL DEFAULT false;
