-- Migration: create waitlist table on the beta DB.
-- Bridge until database-consolidation.aps.md lands — admin approve/invite
-- routes call upsertWaitlistWithName / findWaitlistEntryByEmail which require
-- the table to exist on the DB pointed to by DATABASE_URL. Historically this
-- lived on a separate Neon project; this migration colocates it.

CREATE EXTENSION IF NOT EXISTS citext;

CREATE TABLE IF NOT EXISTS waitlist (
  id         serial PRIMARY KEY,
  email      citext UNIQUE NOT NULL,
  name       text,
  company    text,
  role       text,
  use_case   text,
  source     text NOT NULL DEFAULT 'website',
  created_at timestamptz NOT NULL DEFAULT now(),
  updated_at timestamptz NOT NULL DEFAULT now()
);

-- update_updated_at() is defined by schema.sql. Relying on that definition
-- here keeps a single source of truth and avoids silently overwriting
-- live-patched function attributes (SECURITY DEFINER, COST, etc.).
DROP TRIGGER IF EXISTS waitlist_updated_at ON waitlist;
CREATE TRIGGER waitlist_updated_at
  BEFORE UPDATE ON waitlist
  FOR EACH ROW
  EXECUTE FUNCTION update_updated_at();
