/**
 * BACT-011 (ADR-121, OQ-A): daily account-activity rollup for historical
 * DAA. `beta_users.last_activity_at` (BACT-008) is a *latest-pointer*
 * column — it holds only the most recent activity timestamp per account,
 * never a history of every active day. Window metrics (BACT-009) read that
 * pointer live and answer "how many accounts are active right now," but
 * cannot answer "how many accounts were active on day D" once an account
 * has gone quiet or become active again on a later day — the pointer has
 * moved on and day D's evidence is gone from `beta_users` itself.
 *
 * This rollup takes a snapshot: once a day, it counts distinct active
 * accounts whose `last_activity_at` falls (in UTC) on each of the last few
 * *completed* UTC days, and durably records that count per day/plan in
 * `activity_rollup_daily` (migration 022). The job runs frequently
 * (piggybacking on the existing hourly `/cron/cleanup` sweep, see
 * `routes/cron.ts`, and error-isolated from it — a rollup failure never
 * fails the cleanup sweep) and re-derives a trailing window of days on
 * every run so a short outage self-heals.
 *
 * The upsert is **best-observation**, not a plain overwrite:
 * `SET active_accounts = GREATEST(stored, newly-observed)`. Because
 * `last_activity_at` only ever advances, a *later* re-roll of an
 * already-written day can only observe the same or a *smaller* set of
 * accounts still pointing at that day (accounts that were active again
 * since have moved their pointer past it) — GREATEST keeps each day's
 * best-ever snapshot instead of letting a later, smaller recount shrink an
 * already-correct earlier one. Stored counts therefore never decrease on
 * re-roll, and the upsert is still idempotent and never double-counts (it
 * is a max, not a sum).
 *
 * That does **not** eliminate undercounting: **a day's *first* rollup, if
 * it happens late — after an account's `last_activity_at` has already
 * advanced past that day — has nothing to observe for that account, and
 * GREATEST has nothing to raise the stored value from.** The 7-day
 * trailing window mitigates the common outage case, but cannot recover a
 * day missed entirely past that window. The stored value approximates
 * accounts observed active that day, not an exact audit log. This
 * limitation must stay documented wherever the rollup is described (see
 * `docs/runbooks/admin-cli.md` — "Daily historical-DAA rollup" section)
 * rather than silently presented as exact history.
 */

export const ACCOUNT_ACTIVITY_ROLLUP_SCHEMA_VERSION = 'anvil.account-activity-rollup.v1';

/**
 * Reserved `plan` value in `activity_rollup_daily` carrying the all-plan
 * total for a day, alongside the per-plan breakdown rows. Distinct from any
 * value in `ACCOUNT_PLANS` (today only `beta`) by construction — the double
 * underscore is not a legal plan name.
 */
export const ROLLUP_TOTAL_PLAN_KEY = '__all__';

/**
 * How many trailing completed UTC days the job recomputes on every run.
 * Small and self-healing: an outage shorter than this window catches up
 * automatically on the next run; upsert semantics make repeat runs free of
 * double-counting. Kept small (not e.g. 90) because every day beyond the
 * first miss is already subject to the late-rollup undercount above — a
 * longer window would not recover missed evidence, only re-confirm it.
 */
export const DEFAULT_ROLLUP_LOOKBACK_DAYS = 7;

/** Default/maximum window for the admin history read (`GET /admin/activity?history=true`). */
export const DEFAULT_HISTORY_DAYS = 14;
export const MAX_HISTORY_DAYS = 90;

/**
 * Return the last `lookbackDays` *completed* UTC calendar days (as
 * `YYYY-MM-DD` strings) ending the day before `now`'s UTC date, oldest
 * first. Reasons entirely in UTC — never the process/local timezone —
 * matching the module's UTC-day rollup grain (ADR-121 / OQ-A).
 */
export function completedUtcDays(now: Date, lookbackDays: number): string[] {
  if (!Number.isInteger(lookbackDays) || lookbackDays < 1) {
    throw new Error(
      `completedUtcDays: lookbackDays must be a positive integer, got ${lookbackDays}`
    );
  }
  const todayUtcMs = Date.UTC(now.getUTCFullYear(), now.getUTCMonth(), now.getUTCDate());
  const days: string[] = [];
  for (let i = lookbackDays; i >= 1; i -= 1) {
    const dayMs = todayUtcMs - i * 24 * 60 * 60 * 1000;
    days.push(new Date(dayMs).toISOString().slice(0, 10));
  }
  return days;
}
