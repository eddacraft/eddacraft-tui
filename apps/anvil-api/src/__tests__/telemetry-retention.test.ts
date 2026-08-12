import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { Hono } from 'hono';
import type { NeonClient } from '../db/client.js';
import {
  DEFAULT_TELEMETRY_RETENTION_DAYS,
  getTelemetryRetentionDays,
} from '../lib/telemetry-retention.js';
import { rollupAndPurgeExpiredTelemetryBeacons } from '../db/queries.js';

const cronMocks = vi.hoisted(() => ({
  getClient: vi.fn(),
  cleanupExpiredDeviceCodes: vi.fn(),
  cleanupExpiredGithubDeviceSessions: vi.fn(),
  cleanupExpiredOtpCodes: vi.fn(),
  cleanupExpiredRefreshTokens: vi.fn(),
  cleanupExpiredBroadcastSnapshots: vi.fn(),
  rollupAndPurgeExpiredTelemetryBeacons: vi.fn(),
  rollupAccountActivity: vi.fn(),
}));

function mockSql(): NeonClient {
  const sql = vi.fn().mockReturnValue({ tagged: true }) as ReturnType<typeof vi.fn> & {
    transaction: ReturnType<typeof vi.fn>;
  };
  sql.transaction = vi.fn();
  return sql as unknown as NeonClient;
}

/** Reassemble the SQL text of every tagged-template call the mock received. */
function capturedQueries(sql: NeonClient): Array<{ text: string; params: unknown[] }> {
  return vi.mocked(sql).mock.calls.map((call) => {
    const [strings, ...params] = call as unknown as [readonly string[], ...unknown[]];
    return { text: strings.join(' ? '), params };
  });
}

const originalRetentionEnv = process.env['TELEMETRY_RETENTION_DAYS'];

afterEach(() => {
  if (originalRetentionEnv === undefined) {
    delete process.env['TELEMETRY_RETENTION_DAYS'];
  } else {
    process.env['TELEMETRY_RETENTION_DAYS'] = originalRetentionEnv;
  }
  vi.restoreAllMocks();
});

describe('telemetry retention configuration', () => {
  it('defaults to the ADR-107 §6 raw-row retention of 90 days', () => {
    delete process.env['TELEMETRY_RETENTION_DAYS'];

    expect(DEFAULT_TELEMETRY_RETENTION_DAYS).toBe(90);
    expect(getTelemetryRetentionDays()).toBe(90);
  });

  it('honours a TELEMETRY_RETENTION_DAYS override', () => {
    process.env['TELEMETRY_RETENTION_DAYS'] = '30';

    expect(getTelemetryRetentionDays()).toBe(30);
  });

  it('rejects invalid explicit configuration rather than silently defaulting', () => {
    for (const bad of ['abc', '0', '-5', '1.5', '91', 'Infinity']) {
      process.env['TELEMETRY_RETENTION_DAYS'] = bad;
      expect(() => getTelemetryRetentionDays()).toThrow(/TELEMETRY_RETENTION_DAYS/);
    }
  });
});

describe('rollupAndPurgeExpiredTelemetryBeacons', () => {
  it('applies the retention window as a bound parameter, not a SQL literal', async () => {
    const sql = mockSql();
    vi.mocked(sql.transaction).mockResolvedValue([[], [], [{ id: 'b-1' }, { id: 'b-2' }]]);

    const purged = await rollupAndPurgeExpiredTelemetryBeacons(sql, 90);

    expect(purged).toBe(2);
    const queries = capturedQueries(sql);
    expect(queries).toHaveLength(3);

    const purge = queries[2]!;
    expect(purge.text).toContain('DELETE FROM telemetry_beacons');
    expect(purge.text).toMatch(/received_on\s*<\s*current_date\s*-\s*\(\s*\?\s*::int\s*-\s*1\s*\)/);
    // The 90-day value arrives as a parameter (configuration), never baked
    // into the SQL text as a magic literal.
    expect(purge.params).toContain(90);
    expect(purge.text).not.toContain('90');
  });

  it('rolls raw rows up into the kept-indefinitely aggregates before purging', async () => {
    const sql = mockSql();
    vi.mocked(sql.transaction).mockResolvedValue([[], [], []]);

    await rollupAndPurgeExpiredTelemetryBeacons(sql, 90);

    // Rollups and the purge run in one transaction so a failed rollup can
    // never lose raw rows.
    expect(sql.transaction).toHaveBeenCalledTimes(1);
    expect(sql.transaction).toHaveBeenCalledWith([
      expect.anything(),
      expect.anything(),
      expect.anything(),
    ]);

    const [installs, features] = capturedQueries(sql);
    expect(installs!.text).toContain('INSERT INTO telemetry_daily_installs');
    expect(installs!.text).toContain('ON CONFLICT');
    expect(installs!.params).toContain(90);
    expect(features!.text).toContain('INSERT INTO telemetry_daily_feature_usage');
    expect(features!.text).toContain('ON CONFLICT');
    expect(features!.params).toContain(90);
  });

  it('rejects a non-positive or non-integer retention window', async () => {
    const sql = mockSql();

    await expect(rollupAndPurgeExpiredTelemetryBeacons(sql, 0)).rejects.toThrow(/retention/i);
    await expect(rollupAndPurgeExpiredTelemetryBeacons(sql, 1.5)).rejects.toThrow(/retention/i);
    await expect(rollupAndPurgeExpiredTelemetryBeacons(sql, 91)).rejects.toThrow(/retention/i);
    expect(sql.transaction).not.toHaveBeenCalled();
  });
});

describe('cron cleanup — telemetry retention sweep', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    cronMocks.cleanupExpiredDeviceCodes.mockResolvedValue(0);
    cronMocks.cleanupExpiredGithubDeviceSessions.mockResolvedValue(0);
    cronMocks.cleanupExpiredOtpCodes.mockResolvedValue(0);
    cronMocks.cleanupExpiredRefreshTokens.mockResolvedValue(0);
    cronMocks.cleanupExpiredBroadcastSnapshots.mockResolvedValue(0);
    cronMocks.rollupAndPurgeExpiredTelemetryBeacons.mockResolvedValue(7);
    cronMocks.rollupAccountActivity.mockResolvedValue([]);
    cronMocks.getClient.mockReturnValue({} as NeonClient);
  });

  it('purges raw beacons with the configured retention window', async () => {
    vi.doMock('../db/client.js', () => ({ getClient: cronMocks.getClient }));
    vi.doMock('../db/queries.js', () => ({
      cleanupExpiredBroadcastSnapshots: cronMocks.cleanupExpiredBroadcastSnapshots,
      cleanupExpiredDeviceCodes: cronMocks.cleanupExpiredDeviceCodes,
      cleanupExpiredGithubDeviceSessions: cronMocks.cleanupExpiredGithubDeviceSessions,
      cleanupExpiredOtpCodes: cronMocks.cleanupExpiredOtpCodes,
      cleanupExpiredRefreshTokens: cronMocks.cleanupExpiredRefreshTokens,
      rollupAndPurgeExpiredTelemetryBeacons: cronMocks.rollupAndPurgeExpiredTelemetryBeacons,
      rollupAccountActivity: cronMocks.rollupAccountActivity,
    }));
    const { cron } = await import('../routes/cron.js');
    vi.doUnmock('../db/client.js');
    vi.doUnmock('../db/queries.js');

    const originalCronSecret = process.env['CRON_SECRET'];
    process.env['CRON_SECRET'] = 'cron-secret';
    delete process.env['TELEMETRY_RETENTION_DAYS'];
    try {
      const app = new Hono();
      app.route('/cron', cron);

      const response = await app.request('/cron/cleanup', {
        headers: { Authorization: 'Bearer cron-secret' },
      });

      expect(response.status).toBe(200);
      const body = (await response.json()) as { cleaned: Record<string, number> };
      expect(body.cleaned['telemetryBeacons']).toBe(7);
      expect(cronMocks.rollupAndPurgeExpiredTelemetryBeacons).toHaveBeenCalledWith(
        expect.anything(),
        DEFAULT_TELEMETRY_RETENTION_DAYS
      );
    } finally {
      if (originalCronSecret === undefined) {
        delete process.env['CRON_SECRET'];
      } else {
        process.env['CRON_SECRET'] = originalCronSecret;
      }
    }
  });
});
