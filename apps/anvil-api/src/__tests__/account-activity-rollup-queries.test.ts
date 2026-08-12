import { afterEach, describe, expect, it, vi } from 'vitest';
import type { NeonClient } from '../db/client.js';
import { ROLLUP_TOTAL_PLAN_KEY } from '../lib/account-activity-rollup.js';
import { findAccountActivityRollupHistory, rollupAccountActivity } from '../db/queries.js';

function mockSql(returnValue: unknown[] = []): NeonClient {
  const sql = vi.fn().mockResolvedValue(returnValue) as ReturnType<typeof vi.fn>;
  return sql as unknown as NeonClient;
}

/**
 * Reassemble the SQL text of every tagged-template call the mock received.
 * `flatParams` flattens one level so assertions don't need to know whether
 * an implementation bound a list as one array parameter (e.g.
 * `${days}::date[]`) or as several scalar parameters.
 */
function capturedQueries(
  sql: NeonClient
): Array<{ text: string; params: unknown[]; flatParams: unknown[] }> {
  return vi.mocked(sql).mock.calls.map((call) => {
    const [strings, ...params] = call as unknown as [readonly string[], ...unknown[]];
    return { text: strings.join(' ? '), params, flatParams: params.flat() };
  });
}

afterEach(() => {
  vi.restoreAllMocks();
});

describe('rollupAccountActivity (BACT-011)', () => {
  it('does nothing and issues no query for an empty day list', async () => {
    const sql = mockSql();
    const result = await rollupAccountActivity(sql, []);
    expect(result).toEqual([]);
    expect(sql).not.toHaveBeenCalled();
  });

  it('computes the UTC-day boundary from last_activity_at, not a session-local cast', async () => {
    const sql = mockSql([]);
    await rollupAccountActivity(sql, ['2026-08-12']);
    const [query] = capturedQueries(sql);
    expect(query!.text).toContain("AT TIME ZONE 'UTC'");
    expect(query!.flatParams).toContain('2026-08-12');
  });

  it('groups by plan and additionally emits a reserved all-plan total row', async () => {
    const sql = mockSql([]);
    await rollupAccountActivity(sql, ['2026-08-12']);
    const [query] = capturedQueries(sql);
    expect(query!.text).toContain('GROUP BY d.day, p.plan');
    expect(query!.flatParams).toContain(ROLLUP_TOTAL_PLAN_KEY);
  });

  it('upserts by overwrite, never by increment — idempotent re-run of the same day', async () => {
    const sql = mockSql([]);
    await rollupAccountActivity(sql, ['2026-08-12']);
    const [query] = capturedQueries(sql);
    expect(query!.text).toContain('ON CONFLICT (day, plan) DO UPDATE');
    expect(query!.text).toContain('SET active_accounts = EXCLUDED.active_accounts');
    // Must not be an additive increment, or a repeat run would double-count.
    expect(query!.text).not.toMatch(/active_accounts\s*\+\s*EXCLUDED/);
  });

  it('returns the upserted rows, mapped to camelCase', async () => {
    const sql = mockSql([
      { day: '2026-08-12', plan: 'beta', active_accounts: 3 },
      { day: '2026-08-12', plan: ROLLUP_TOTAL_PLAN_KEY, active_accounts: 3 },
    ]);
    const result = await rollupAccountActivity(sql, ['2026-08-12']);
    expect(result).toEqual([
      { day: '2026-08-12', plan: 'beta', activeAccounts: 3 },
      { day: '2026-08-12', plan: ROLLUP_TOTAL_PLAN_KEY, activeAccounts: 3 },
    ]);
  });

  it('binds every requested day as a parameter (multi-day catch-up sweep)', async () => {
    const sql = mockSql([]);
    await rollupAccountActivity(sql, ['2026-08-10', '2026-08-11', '2026-08-12']);
    const [query] = capturedQueries(sql);
    expect(query!.flatParams).toEqual(
      expect.arrayContaining(['2026-08-10', '2026-08-11', '2026-08-12'])
    );
  });
});

describe('findAccountActivityRollupHistory (BACT-011)', () => {
  it('reads the reserved all-plan total series when no plan filter is given', async () => {
    const sql = mockSql([
      { day: '2026-08-12', plan: ROLLUP_TOTAL_PLAN_KEY, active_accounts: 5 },
      { day: '2026-08-11', plan: ROLLUP_TOTAL_PLAN_KEY, active_accounts: 4 },
    ]);
    const result = await findAccountActivityRollupHistory(sql, { plan: null, days: 14 });
    expect(result).toEqual([
      { day: '2026-08-12', plan: ROLLUP_TOTAL_PLAN_KEY, activeAccounts: 5 },
      { day: '2026-08-11', plan: ROLLUP_TOTAL_PLAN_KEY, activeAccounts: 4 },
    ]);
    const [query] = capturedQueries(sql);
    expect(query!.flatParams).toContain(ROLLUP_TOTAL_PLAN_KEY);
  });

  it('filters to a specific plan series when a plan is given', async () => {
    const sql = mockSql([{ day: '2026-08-12', plan: 'beta', active_accounts: 5 }]);
    const result = await findAccountActivityRollupHistory(sql, { plan: 'beta', days: 14 });
    expect(result).toEqual([{ day: '2026-08-12', plan: 'beta', activeAccounts: 5 }]);
    const [query] = capturedQueries(sql);
    expect(query!.flatParams).toContain('beta');
    expect(query!.flatParams).not.toContain(ROLLUP_TOTAL_PLAN_KEY);
  });

  it('orders most-recent day first and bounds the row count by the requested window', async () => {
    const sql = mockSql([]);
    await findAccountActivityRollupHistory(sql, { plan: null, days: 30 });
    const [query] = capturedQueries(sql);
    expect(query!.text).toMatch(/ORDER BY day DESC/);
    expect(query!.text).toContain('LIMIT');
    expect(query!.flatParams).toContain(30);
  });

  it('returns an empty series rather than throwing when nothing has been rolled up yet', async () => {
    const sql = mockSql([]);
    const result = await findAccountActivityRollupHistory(sql, { plan: null, days: 14 });
    expect(result).toEqual([]);
  });
});
