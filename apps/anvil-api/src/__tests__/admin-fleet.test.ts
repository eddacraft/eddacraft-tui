import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { Hono } from 'hono';
import { admin } from '../routes/admin.js';
import { _resetAdminRateLimitForTests } from '../middleware/admin-rate-limit.js';

const ADMIN_KEY = 'fleet-test-admin-key';

const mockSql = vi.fn();
vi.mock('../db/client.js', () => ({
  getClient: vi.fn(() => mockSql),
}));

const app = new Hono();
app.route('/admin', admin);

function request(authKey?: string) {
  const headers: Record<string, string> = {};
  if (authKey) headers['Authorization'] = `Bearer ${authKey}`;
  return app.request('/admin/fleet', { headers });
}

describe('GET /admin/fleet', () => {
  const originalAdminKey = process.env['ADMIN_KEY'];

  beforeEach(() => {
    vi.clearAllMocks();
    _resetAdminRateLimitForTests();
    process.env['ADMIN_KEY'] = ADMIN_KEY;
    mockSql.mockResolvedValue([]);
  });

  afterEach(() => {
    if (originalAdminKey === undefined) delete process.env['ADMIN_KEY'];
    else process.env['ADMIN_KEY'] = originalAdminKey;
    vi.restoreAllMocks();
  });

  it('returns 401 without a bearer', async () => {
    const response = await request();
    expect(response.status).toBe(401);
  });

  it('returns 403 for the wrong bearer', async () => {
    const response = await request('wrong-key');
    expect(response.status).toBe(403);
  });

  it('returns the stable empty contract through admin auth and rate limiting', async () => {
    mockSql.mockResolvedValue([
      {
        as_of: '2026-07-16',
        beacon_id: null,
        install_id: null,
        received_on: null,
        version: null,
        install_method: null,
        feature_id: null,
        feature_key: null,
        usage_count: null,
      },
    ]);

    const response = await request(ADMIN_KEY);

    expect(response.status).toBe(200);
    expect(response.headers.get('X-RateLimit-Scope')).toBe('all');
    expect(await response.json()).toEqual({
      schemaVersion: 'anvil.fleet-overview.v1',
      asOf: '2026-07-16',
      activeInstalls: { daily: 0, weekly: 0, monthly: 0 },
      distributions: { versions: [], installMethods: [] },
      featureAdoption: [],
      retentionCohorts: [],
      notes: { activityDefinition: 'beacon observed', rawRetentionDays: 90 },
    });
    const [query, retentionDays] = mockSql.mock.calls[0]!;
    expect((query as TemplateStringsArray).join(' ')).toMatch(
      /current_date[\s\S]+telemetry_beacons[\s\S]+telemetry_beacon_features/
    );
    expect(retentionDays).toBe(90);
  });
});
