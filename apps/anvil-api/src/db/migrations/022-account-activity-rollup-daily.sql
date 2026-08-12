-- Migration: activity_rollup_daily for BACT-011 (beta account activity
-- phase 2, ADR-121, OQ-A).
--
-- Daily snapshot of distinct *active* account counts, per completed UTC day
-- and per `plan` (plus a reserved '__all__' total row) — historical DAA
-- that `beta_users.last_activity_at` (BACT-008) alone cannot reconstruct,
-- because that column is a latest-pointer (one timestamp per account, not a
-- history of every active day).
--
-- Grain: one row per (day, plan); `plan` is either a value from the
-- `beta_users.plan` closed set (today only 'beta') or the reserved
-- '__all__' sentinel carrying the cross-plan total for that day.
--
-- Idempotent, best-observation write: the job (lib/account-activity-rollup.ts,
-- db/queries.ts#rollupAccountActivity) always recomputes a day's count from
-- `beta_users` and upserts via
-- `ON CONFLICT (day, plan) DO UPDATE SET active_accounts =
--   GREATEST(activity_rollup_daily.active_accounts, EXCLUDED.active_accounts)`
-- — never a plain overwrite and never an increment. `last_activity_at` only
-- ever advances, so a later re-roll of an already-written day can only
-- observe the same or a *smaller* set of accounts still pointing at that
-- day; GREATEST keeps each day's best-ever snapshot instead of letting a
-- later, smaller recount shrink an already-correct earlier one. Re-running
-- the same completed day any number of times never double-counts (it is a
-- max, not a sum) and stored counts never decrease.
--
-- Late-rollup undercount (documented honestly, not silently): a day's
-- *first* rollup is only as good as `beta_users.last_activity_at` at the
-- moment the job runs. If an account's activity has already advanced past
-- that day before the day is ever rolled up once, that day's count can no
-- longer see the account, and GREATEST has nothing to raise it from — the
-- stored value approximates accounts observed active that day, not an
-- exact audit log. The job mitigates the common case by recomputing a
-- small trailing window of days on every run (self-heals short outages),
-- but cannot recover a day missed entirely past that window. See
-- `docs/runbooks/admin-cli.md` for the operator-facing version of this
-- caveat.
--
-- Retention: rows are kept indefinitely. Volume is trivial — at most
-- (number of plans + 1) rows per day, so even years of daily history stays
-- a tiny table. Unlike `telemetry_beacons` (migration 017), there is no raw
-- per-event table backing this rollup to prune; the rollup itself *is* the
-- retained aggregate.

SET LOCAL lock_timeout = '30s';

CREATE TABLE IF NOT EXISTS activity_rollup_daily (
  day             date NOT NULL,
  plan            text NOT NULL,
  active_accounts int  NOT NULL CHECK (active_accounts >= 0),
  computed_at     timestamptz NOT NULL DEFAULT now(),
  PRIMARY KEY (day, plan)
);

CREATE INDEX IF NOT EXISTS idx_activity_rollup_daily_plan_day
  ON activity_rollup_daily (plan, day DESC);
