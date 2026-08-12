import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { Hono } from 'hono';
import { admin } from '../routes/admin.js';
import { _resetAdminRateLimitForTests } from '../middleware/admin-rate-limit.js';

const ADMIN_KEY = 'activity-test-admin-key';

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
const DAY_MS = 24 * 60 * 60 * 1000;

function isoMinusMs(ms: number): string {
  return new Date(new Date(AS_OF).getTime() - ms).toISOString();
}

describe('GET /admin/activity (BACT-009)', () => {
  const originalAdminKey = process.env['ADMIN_KEY'];

  beforeEach(() => {
    vi.clearAllMocks();
    _resetAdminRateLimitForTests();
    process.env['ADMIN_KEY'] = ADMIN_KEY;
  });

  afterEach(() => {
    if (originalAdminKey === undefined) delete process.env['ADMIN_KEY'];
    else process.env['ADMIN_KEY'] = originalAdminKey;
    vi.restoreAllMocks();
  });

  it('returns 401 without a bearer', async () => {
    const response = await request('/activity');
    expect(response.status).toBe(401);
  });

  it('returns 403 for the wrong bearer', async () => {
    const response = await request('/activity', 'wrong-key');
    expect(response.status).toBe(403);
  });

  it('returns the stable envelope for an empty account table', async () => {
    mockSql.mockResolvedValueOnce([{ as_of: AS_OF, account_id: null, last_activity_at: null }]);

    const response = await request('/activity', ADMIN_KEY);
    expect(response.status).toBe(200);
    const body = await response.json();
    expect(body.schemaVersion).toBe('anvil.account-activity.v1');
    expect(body.asOf).toBe(AS_OF);
    expect(body.plan).toBeNull();
    expect(body.totalAccounts).toBe(0);
    expect(body.activeAccounts).toEqual({ daily: 0, weekly: 0, monthly: 0 });
    expect(body.neverActive).toBe(0);
    expect(body.quiet).toEqual({ idleDays: 30, count: 0 });
    expect(body.notes.unit).toBe('accounts');
  });

  it('computes DAA/WAA/MAA and cohorts from account rows', async () => {
    mockSql.mockResolvedValueOnce([
      { as_of: AS_OF, account_id: 'a', last_activity_at: isoMinusMs(0) },
      { as_of: AS_OF, account_id: 'b', last_activity_at: isoMinusMs(3 * DAY_MS) },
      { as_of: AS_OF, account_id: 'c', last_activity_at: null },
      { as_of: AS_OF, account_id: 'd', last_activity_at: isoMinusMs(90 * DAY_MS) },
    ]);

    const response = await request('/activity', ADMIN_KEY);
    expect(response.status).toBe(200);
    const body = await response.json();
    expect(body.totalAccounts).toBe(4);
    expect(body.activeAccounts).toEqual({ daily: 1, weekly: 2, monthly: 2 });
    expect(body.neverActive).toBe(1);
    expect(body.quiet).toEqual({ idleDays: 30, count: 2 }); // null + 90-day-old
  });

  it('passes the plan filter through to the query and echoes it back', async () => {
    mockSql.mockResolvedValueOnce([
      { as_of: AS_OF, account_id: 'a', last_activity_at: isoMinusMs(0) },
    ]);

    const response = await request('/activity?plan=beta', ADMIN_KEY);
    expect(response.status).toBe(200);
    const body = await response.json();
    expect(body.plan).toBe('beta');
    expect(body.totalAccounts).toBe(1);
    // The plan filter is bound into the SQL tagged-template call as one of
    // its interpolated values.
    const callArgs = mockSql.mock.calls[0] as unknown[];
    expect(callArgs).toContain('beta');
  });

  it('rejects an unrecognised plan value', async () => {
    const response = await request('/activity?plan=enterprise', ADMIN_KEY);
    expect(response.status).toBe(400);
  });

  it('honours a custom idleDays quiet window', async () => {
    mockSql.mockResolvedValueOnce([
      { as_of: AS_OF, account_id: 'a', last_activity_at: isoMinusMs(15 * DAY_MS) },
    ]);

    const response = await request('/activity?idleDays=14', ADMIN_KEY);
    expect(response.status).toBe(200);
    const body = await response.json();
    expect(body.quiet).toEqual({ idleDays: 14, count: 1 });
  });

  it('rejects idleDays outside the supported range', async () => {
    const tooLow = await request('/activity?idleDays=0', ADMIN_KEY);
    expect(tooLow.status).toBe(400);

    const tooHigh = await request('/activity?idleDays=366', ADMIN_KEY);
    expect(tooHigh.status).toBe(400);
  });
});
