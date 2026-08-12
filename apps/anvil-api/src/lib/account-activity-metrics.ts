/**
 * BACT-009 (ADR-121): admin account activity metrics — DAA/WAA/MAA windows,
 * never-active, and quiet cohorts computed from `beta_users.last_activity_at`
 * (BACT-008).
 *
 * Mirrors the `apps/anvil-api/src/lib/fleet-overview.ts` shape: a pure
 * aggregation function over rows the DB layer fetched, keyed to a
 * `Postgres`-supplied `as_of` value rather than the process wall clock. That
 * keeps window-boundary behaviour deterministic in tests and avoids drift
 * between the app clock and Postgres's clock.
 *
 * This is a NAMED-ACCOUNT surface — distinct from FLEET's anonymous install
 * DAI/WAU/MAU (`apps/anvil-api/src/lib/fleet-overview.ts`). Never sum or
 * compare the two directly (ADR-121 decision 5); never join a FLEET
 * `install_id` to an account here.
 */

import type { NeonClient } from '../db/client.js';

export const ACCOUNT_ACTIVITY_SCHEMA_VERSION = 'anvil.account-activity.v1';

const DAY_MS = 24 * 60 * 60 * 1000;

/**
 * One row per query result. `account_id` is `null` only for the
 * clock-anchor row a `LEFT JOIN` against zero matching `beta_users` rows
 * produces (empty table, or a `plan` filter with no matches) — it carries
 * `as_of` but represents no account and must not be counted.
 */
export interface AccountActivityQueryRow {
  as_of: string;
  account_id: string | null;
  last_activity_at: string | null;
}

export interface AccountActivityWindowCounts {
  daily: number;
  weekly: number;
  monthly: number;
}

export interface AccountActivityQuiet {
  idleDays: number;
  count: number;
}

export interface AccountActivityNotes {
  unit: 'accounts';
  activityDefinition: string;
  comparisonNote: string;
}

export interface AccountActivityOverview {
  schemaVersion: typeof ACCOUNT_ACTIVITY_SCHEMA_VERSION;
  asOf: string;
  plan: string | null;
  totalAccounts: number;
  activeAccounts: AccountActivityWindowCounts;
  neverActive: number;
  quiet: AccountActivityQuiet;
  notes: AccountActivityNotes;
}

export interface BuildAccountActivityOptions {
  /** Quiet-cohort threshold in days (default 30, OQ3). */
  idleDays: number;
  /** Plan filter already applied by the caller's query; echoed, not re-applied. */
  plan: string | null;
}

/**
 * Aggregate account activity window metrics from raw query rows.
 *
 * Windows are inclusive at the exact-day boundary: an account active exactly
 * N days ago (to the millisecond) still counts in the N-day window — the
 * complement of the existing BACT-006 `idle` filter, which treats exactly
 * `idleDays` old as NOT yet idle (`last_login_at < now() - interval`, a
 * strict less-than). `quiet` mirrors that same strict semantic: an account
 * exactly `idleDays` old is not quiet, only strictly older is.
 */
export function buildAccountActivityOverview(
  rows: AccountActivityQueryRow[],
  options: BuildAccountActivityOptions
): AccountActivityOverview {
  const asOf = rows[0]?.as_of;
  if (!asOf) {
    throw new Error('account activity query did not return as_of');
  }
  const asOfMs = new Date(asOf).getTime();
  const idleThresholdMs = options.idleDays * DAY_MS;

  let totalAccounts = 0;
  let daily = 0;
  let weekly = 0;
  let monthly = 0;
  let neverActive = 0;
  let quiet = 0;

  for (const row of rows) {
    if (!row.account_id) continue; // clock-anchor row only, no matching account
    totalAccounts += 1;

    if (!row.last_activity_at) {
      neverActive += 1;
      quiet += 1;
      continue;
    }

    const ageMs = asOfMs - new Date(row.last_activity_at).getTime();
    if (ageMs <= DAY_MS) daily += 1;
    if (ageMs <= 7 * DAY_MS) weekly += 1;
    if (ageMs <= 30 * DAY_MS) monthly += 1;
    if (ageMs > idleThresholdMs) quiet += 1;
  }

  return {
    schemaVersion: ACCOUNT_ACTIVITY_SCHEMA_VERSION,
    asOf,
    plan: options.plan,
    totalAccounts,
    activeAccounts: { daily, weekly, monthly },
    neverActive,
    quiet: { idleDays: options.idleDays, count: quiet },
    notes: {
      unit: 'accounts',
      activityDefinition:
        'last_activity_at set by interactive login, successful session refresh, or an ' +
        'authenticated allowlisted feature-touch (ADR-121); invite/approve alone does not count',
      comparisonNote:
        'DAA/WAA/MAA count named accounts (this surface) and are never comparable to FLEET ' +
        'DAI/WAU/MAU (anonymous installs, `anvil admin fleet`) — do not sum or join the two',
    },
  };
}

/**
 * Fetch raw `beta_users` activity rows for the window aggregation above.
 *
 * Uses a clock-anchor `LEFT JOIN` (matching `findFleetOverview`'s pattern)
 * so the query always returns at least one row carrying `as_of`, even when
 * no account matches (empty table, or an unmatched `plan` filter) — the
 * aggregator needs `as_of` to compute window ages regardless of row count.
 * Only `status = 'active'` accounts are counted, matching the existing
 * BACT-006 engagement-filter scope.
 */
export async function findAccountActivityRows(
  sql: NeonClient,
  plan: string | null
): Promise<AccountActivityQueryRow[]> {
  const result = plan
    ? await sql`
        SELECT
          clock.as_of::text AS as_of,
          u.id::text AS account_id,
          u.last_activity_at::text AS last_activity_at
        FROM (SELECT now() AS as_of) AS clock
        LEFT JOIN beta_users u ON u.status = 'active' AND u.plan = ${plan}
      `
    : await sql`
        SELECT
          clock.as_of::text AS as_of,
          u.id::text AS account_id,
          u.last_activity_at::text AS last_activity_at
        FROM (SELECT now() AS as_of) AS clock
        LEFT JOIN beta_users u ON u.status = 'active'
      `;
  return (result as Record<string, unknown>[]).map((row) => ({
    as_of: String(row['as_of']),
    account_id: row['account_id'] === null ? null : String(row['account_id']),
    last_activity_at: row['last_activity_at'] === null ? null : String(row['last_activity_at']),
  }));
}
