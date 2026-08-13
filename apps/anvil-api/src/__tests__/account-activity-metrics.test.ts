import { describe, expect, it, vi } from 'vitest';
import {
  ACCOUNT_ACTIVITY_SCHEMA_VERSION,
  buildAccountActivityOverview,
  findAccountActivityRows,
  type AccountActivityQueryRow,
} from '../lib/account-activity-metrics.js';
import type { NeonClient } from '../db/client.js';

const AS_OF = '2026-08-13T12:00:00.000Z';
const DAY_MS = 24 * 60 * 60 * 1000;

function isoMinusMs(ms: number): string {
  return new Date(new Date(AS_OF).getTime() - ms).toISOString();
}

/** Build query rows the way `findAccountActivityRows` would: one row per
 * matching account, `account_id` null only for the clock-anchor row that a
 * LEFT JOIN against an empty/non-matching `beta_users` produces. */
function rows(
  accounts: Array<{ id: string; last_activity_at: string | null }>
): AccountActivityQueryRow[] {
  if (accounts.length === 0) {
    return [{ as_of: AS_OF, account_id: null, last_activity_at: null }];
  }
  return accounts.map((a) => ({
    as_of: AS_OF,
    account_id: a.id,
    last_activity_at: a.last_activity_at,
  }));
}

describe('buildAccountActivityOverview', () => {
  it('counts accounts active within the daily/weekly/monthly windows', () => {
    const overview = buildAccountActivityOverview(
      rows([
        { id: 'a', last_activity_at: isoMinusMs(0) }, // now — daily/weekly/monthly
        { id: 'b', last_activity_at: isoMinusMs(3 * DAY_MS) }, // weekly/monthly only
        { id: 'c', last_activity_at: isoMinusMs(20 * DAY_MS) }, // monthly only
        { id: 'd', last_activity_at: isoMinusMs(90 * DAY_MS) }, // none of the windows
      ]),
      { idleDays: 30, plan: null }
    );

    expect(overview.schemaVersion).toBe(ACCOUNT_ACTIVITY_SCHEMA_VERSION);
    expect(overview.asOf).toBe(AS_OF);
    expect(overview.totalAccounts).toBe(4);
    expect(overview.activeAccounts).toEqual({ daily: 1, weekly: 2, monthly: 3 });
  });

  it('includes the boundary exactly at N days in that window (inclusive)', () => {
    const overview = buildAccountActivityOverview(
      rows([
        { id: 'daily-boundary', last_activity_at: isoMinusMs(1 * DAY_MS) },
        { id: 'weekly-boundary', last_activity_at: isoMinusMs(7 * DAY_MS) },
        { id: 'monthly-boundary', last_activity_at: isoMinusMs(30 * DAY_MS) },
      ]),
      { idleDays: 30, plan: null }
    );

    // Exactly 1 day old still counts as daily active; 7/30 fall out of the
    // daily window but land in weekly/monthly respectively.
    expect(overview.activeAccounts.daily).toBe(1);
    expect(overview.activeAccounts.weekly).toBe(2); // daily-boundary + weekly-boundary
    expect(overview.activeAccounts.monthly).toBe(3); // all three
  });

  it('excludes activity one millisecond past a window boundary', () => {
    const overview = buildAccountActivityOverview(
      rows([
        { id: 'just-over-daily', last_activity_at: isoMinusMs(1 * DAY_MS + 1) },
        { id: 'just-over-weekly', last_activity_at: isoMinusMs(7 * DAY_MS + 1) },
        { id: 'just-over-monthly', last_activity_at: isoMinusMs(30 * DAY_MS + 1) },
      ]),
      { idleDays: 30, plan: null }
    );

    expect(overview.activeAccounts.daily).toBe(0);
    expect(overview.activeAccounts.weekly).toBe(1); // just-over-daily only
    expect(overview.activeAccounts.monthly).toBe(2); // just-over-daily + just-over-weekly
  });

  it('returns all-zero counts for an empty account table, still carrying asOf', () => {
    const overview = buildAccountActivityOverview(rows([]), { idleDays: 30, plan: null });

    expect(overview.asOf).toBe(AS_OF);
    expect(overview.totalAccounts).toBe(0);
    expect(overview.activeAccounts).toEqual({ daily: 0, weekly: 0, monthly: 0 });
    expect(overview.neverActive).toBe(0);
    expect(overview.quiet).toEqual({ idleDays: 30, count: 0 });
  });

  it('counts null last_activity_at as never-active and quiet', () => {
    const overview = buildAccountActivityOverview(
      rows([
        { id: 'invited-never-active', last_activity_at: null },
        { id: 'active-today', last_activity_at: isoMinusMs(0) },
      ]),
      { idleDays: 30, plan: null }
    );

    expect(overview.neverActive).toBe(1);
    expect(overview.quiet.count).toBe(1);
    expect(overview.totalAccounts).toBe(2);
  });

  it('excludes activity exactly at the idle-days boundary from quiet (strictly older only)', () => {
    const overview = buildAccountActivityOverview(
      rows([
        { id: 'exactly-idle-days', last_activity_at: isoMinusMs(30 * DAY_MS) },
        { id: 'just-past-idle-days', last_activity_at: isoMinusMs(30 * DAY_MS + 1) },
      ]),
      { idleDays: 30, plan: null }
    );

    expect(overview.quiet.count).toBe(1);
  });

  it('honours a custom idleDays window', () => {
    const overview = buildAccountActivityOverview(
      rows([{ id: 'a', last_activity_at: isoMinusMs(15 * DAY_MS) }]),
      { idleDays: 14, plan: null }
    );

    expect(overview.quiet).toEqual({ idleDays: 14, count: 1 });
  });

  it('echoes the plan filter applied by the caller', () => {
    const overview = buildAccountActivityOverview(rows([{ id: 'a', last_activity_at: null }]), {
      idleDays: 30,
      plan: 'beta',
    });

    expect(overview.plan).toBe('beta');
  });

  it('labels the unit as accounts and distinguishes DAA from FLEET DAI', () => {
    const overview = buildAccountActivityOverview(rows([]), { idleDays: 30, plan: null });

    expect(overview.notes.unit).toBe('accounts');
    expect(overview.notes.comparisonNote.toLowerCase()).toContain('dai');
  });

  it('throws when the query returned no rows at all (contract violation)', () => {
    expect(() => buildAccountActivityOverview([], { idleDays: 30, plan: null })).toThrow();
  });
});

describe('findAccountActivityRows', () => {
  // Timestamp portability: the query casts with `extract(epoch from …)` — a
  // plain numeric with no dependence on the session's DateStyle setting —
  // rather than `::text` (Postgres formats timestamp text per DateStyle:
  // ISO vs Postgres/SQL/German output, MDY/DMY/YMD field order; `new
  // Date(nonIsoString)` parsing is then engine-defined in JS for anything
  // but the ISO subset). These tests prove the row mapper never falls back
  // to parsing Postgres's textual timestamp output.

  function fakeSql(rows: unknown[]): NeonClient {
    return vi.fn().mockResolvedValue(rows) as unknown as NeonClient;
  }

  it('converts a numeric epoch to the equivalent ISO string, independent of any DateStyle', async () => {
    const asOfEpoch = new Date(AS_OF).getTime() / 1000;
    const lastActivityEpoch = new Date(isoMinusMs(3 * DAY_MS)).getTime() / 1000;

    const rows = await findAccountActivityRows(
      fakeSql([{ as_of: asOfEpoch, account_id: 'a', last_activity_at: lastActivityEpoch }]),
      null
    );

    expect(rows).toEqual([
      { as_of: AS_OF, account_id: 'a', last_activity_at: isoMinusMs(3 * DAY_MS) },
    ]);
  });

  it('accepts a numeric-string epoch (some drivers coerce numerics to strings)', async () => {
    const asOfEpoch = new Date(AS_OF).getTime() / 1000;

    const rows = await findAccountActivityRows(
      fakeSql([{ as_of: String(asOfEpoch), account_id: null, last_activity_at: null }]),
      null
    );

    expect(rows).toEqual([{ as_of: AS_OF, account_id: null, last_activity_at: null }]);
  });

  it('fails loudly rather than silently misparsing a non-numeric epoch — e.g. a DateStyle=German-shaped timestamp a regressed ::text query might emit', async () => {
    await expect(
      findAccountActivityRows(
        fakeSql([{ as_of: '13.08.2026 12:00:00', account_id: null, last_activity_at: null }]),
        null
      )
    ).rejects.toThrow(/non-finite/);
  });

  it('sends the plan filter as a bound query parameter', async () => {
    const asOfEpoch = new Date(AS_OF).getTime() / 1000;
    const sql = fakeSql([{ as_of: asOfEpoch, account_id: null, last_activity_at: null }]);

    await findAccountActivityRows(sql, 'beta');

    const callArgs = (sql as unknown as ReturnType<typeof vi.fn>).mock.calls[0] as unknown[];
    expect(callArgs).toContain('beta');
  });
});
