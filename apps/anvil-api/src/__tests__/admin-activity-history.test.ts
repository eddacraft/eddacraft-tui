import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { Hono } from 'hono';
import { admin } from '../routes/admin.js';
import { _resetAdminRateLimitForTests } from '../middleware/admin-rate-limit.js';
import { ROLLUP_TOTAL_PLAN_KEY, DEFAULT_HISTORY_DAYS } from '../lib/account-activity-rollup.js';

// BACT-011 (ADR-121 OQ-A): `GET /admin/activity?history=true` extends the
// BACT-009 window-metrics envelope with a `history` block read from the
// activity_rollup_daily table (BACT-011). Backward tolerant: omitting
// `history` leaves the BACT-009 response shape untouched (asserted in
// admin-activity.test.ts, unmodified by this item).

const ADMIN_KEY = 'activity-history-test-admin-key';

const mockSql = vi.fn();
vi.mock('../db/client.js', () => ({
  getClient: vi.fn(() => mockSql),
}));

const app = new Hono();
app.route('/admin', admin);

function request(path: string, authKey?: string) {
  const headers: Record<string, string> = {};
  if (authKey) headers['Authorization'] = `Bearer ${authKey}`;
  return app.request(`/admin${path}`, { headers });
}

const AS_OF = '2026-08-13T12:00:00.000Z';

describe('GET /admin/activity?history=true (BACT-011)', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    _resetAdminRateLimitForTests();
    process.env['ADMIN_KEY'] = ADMIN_KEY;
  });

  afterEach(() => {
    delete process.env['ADMIN_KEY'];
    vi.restoreAllMocks();
  });

  it('omits the history block entirely when history is not requested', async () => {
    mockSql.mockResolvedValueOnce([{ as_of: AS_OF, account_id: null, last_activity_at: null }]);

    const response = await request('/activity', ADMIN_KEY);
    expect(response.status).toBe(200);
    const body = await response.json();
    expect(body.history).toBeUndefined();
    // Only the window-metrics query ran — no rollup-history query.
    expect(mockSql).toHaveBeenCalledTimes(1);
  });

  it('adds the all-plan total history series when history=true with no plan filter', async () => {
    mockSql
      .mockResolvedValueOnce([{ as_of: AS_OF, account_id: null, last_activity_at: null }])
      .mockResolvedValueOnce([
        { day: '2026-08-12', plan: ROLLUP_TOTAL_PLAN_KEY, active_accounts: 5 },
        { day: '2026-08-11', plan: ROLLUP_TOTAL_PLAN_KEY, active_accounts: 4 },
      ]);

    const response = await request('/activity?history=true', ADMIN_KEY);
    expect(response.status).toBe(200);
    const body = await response.json();
    expect(body.history.days).toBe(DEFAULT_HISTORY_DAYS);
    expect(body.history.series).toEqual([
      { day: '2026-08-12', plan: ROLLUP_TOTAL_PLAN_KEY, activeAccounts: 5 },
      { day: '2026-08-11', plan: ROLLUP_TOTAL_PLAN_KEY, activeAccounts: 4 },
    ]);
  });

  it('scopes history to the requested plan when a plan filter is set', async () => {
    mockSql
      .mockResolvedValueOnce([{ as_of: AS_OF, account_id: 'a', last_activity_at: AS_OF }])
      .mockResolvedValueOnce([{ day: '2026-08-12', plan: 'beta', active_accounts: 1 }]);

    const response = await request('/activity?plan=beta&history=true', ADMIN_KEY);
    expect(response.status).toBe(200);
    const body = await response.json();
    expect(body.history.series).toEqual([{ day: '2026-08-12', plan: 'beta', activeAccounts: 1 }]);
  });

  it('honours a custom historyDays window, bounded to MAX_HISTORY_DAYS', async () => {
    mockSql
      .mockResolvedValueOnce([{ as_of: AS_OF, account_id: null, last_activity_at: null }])
      .mockResolvedValueOnce([]);

    const response = await request('/activity?history=true&historyDays=30', ADMIN_KEY);
    expect(response.status).toBe(200);
    const body = await response.json();
    expect(body.history.days).toBe(30);

    const tooHigh = await request('/activity?history=true&historyDays=91', ADMIN_KEY);
    expect(tooHigh.status).toBe(400);
  });

  it('returns an empty series rather than erroring when nothing has been rolled up yet', async () => {
    mockSql
      .mockResolvedValueOnce([{ as_of: AS_OF, account_id: null, last_activity_at: null }])
      .mockResolvedValueOnce([]);

    const response = await request('/activity?history=true', ADMIN_KEY);
    expect(response.status).toBe(200);
    const body = await response.json();
    expect(body.history.series).toEqual([]);
  });
});
