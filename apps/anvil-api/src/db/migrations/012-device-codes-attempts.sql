-- Migration: per-code brute-force counter on device_codes.
--
-- Adds an `attempts` column so /device/confirm can lock a user_code after
-- repeated email mismatches, mirroring otp_codes.attempts. Without this,
-- a known 32-bit user_code (8 hex chars) is brute-forceable against a
-- targeted email over the 48h validity window for invite/approve flows.
-- See issue #922.

ALTER TABLE device_codes
  ADD COLUMN IF NOT EXISTS attempts int NOT NULL DEFAULT 0;
