-- Migration: add name, company, role, use_case columns to waitlist table
-- Safe to run multiple times (IF NOT EXISTS equivalent via DO block)

DO $$
BEGIN
  IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_schema = current_schema() AND table_name = 'waitlist' AND column_name = 'name') THEN
    ALTER TABLE waitlist ADD COLUMN name text;
  END IF;
  IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_schema = current_schema() AND table_name = 'waitlist' AND column_name = 'company') THEN
    ALTER TABLE waitlist ADD COLUMN company text;
  END IF;
  IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_schema = current_schema() AND table_name = 'waitlist' AND column_name = 'role') THEN
    ALTER TABLE waitlist ADD COLUMN role text;
  END IF;
  IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_schema = current_schema() AND table_name = 'waitlist' AND column_name = 'use_case') THEN
    ALTER TABLE waitlist ADD COLUMN use_case text;
  END IF;
END $$;
