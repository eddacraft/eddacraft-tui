import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { Hono } from 'hono';
import type { NeonClient } from '../db/client.js';
import { DEFAULT_ROLLUP_LOOKBACK_DAYS } from '../lib/account-activity-rollup.js';

// BACT-011 (ADR-121 OQ-A): the daily rollup piggybacks on the existing
// hourly `/cron/cleanup` sweep (the "cron/job already used by anvil-api")
// rather than a new scheduling mechanism / vercel.json entry. Mirrors the
// mocking posture of telemetry-retention.test.ts's cron section.

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

beforeEach(() => {
  vi.clearAllMocks();
  cronMocks.cleanupExpiredDeviceCodes.mockResolvedValue(0);
  cronMocks.cleanupExpiredGithubDeviceSessions.mockResolvedValue(0);
  cronMocks.cleanupExpiredOtpCodes.mockResolvedValue(0);
  cronMocks.cleanupExpiredRefreshTokens.mockResolvedValue(0);
  cronMocks.cleanupExpiredBroadcastSnapshots.mockResolvedValue(0);
  cronMocks.rollupAndPurgeExpiredTelemetryBeacons.mockResolvedValue(0);
  cronMocks.rollupAccountActivity.mockResolvedValue([
    { day: '2026-08-12', plan: 'beta', activeAccounts: 2 },
    { day: '2026-08-12', plan: '__all__', activeAccounts: 2 },
  ]);
  cronMocks.getClient.mockReturnValue({} as NeonClient);
});

afterEach(() => {
  vi.doUnmock('../db/client.js');
  vi.doUnmock('../db/queries.js');
});

async function loadCronApp() {
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
  const app = new Hono();
  app.route('/cron', cron);
  return app;
}

describe('GET /cron/cleanup — account activity rollup (BACT-011)', () => {
  it('rolls up the trailing completed-UTC-day window on every run', async () => {
    const app = await loadCronApp();
    const originalCronSecret = process.env['CRON_SECRET'];
    process.env['CRON_SECRET'] = 'cron-secret';
    try {
      const response = await app.request('/cron/cleanup', {
        headers: { Authorization: 'Bearer cron-secret' },
      });

      expect(response.status).toBe(200);
      expect(cronMocks.rollupAccountActivity).toHaveBeenCalledTimes(1);
      const [, days] = cronMocks.rollupAccountActivity.mock.calls[0] as [unknown, string[]];
      expect(days).toHaveLength(DEFAULT_ROLLUP_LOOKBACK_DAYS);
      // Never includes today (the still-open UTC day) — only completed days.
      const todayUtc = new Date().toISOString().slice(0, 10);
      expect(days).not.toContain(todayUtc);

      const body = (await response.json()) as { activityRollup: { days: number; rows: number } };
      expect(body.activityRollup).toEqual({ days: DEFAULT_ROLLUP_LOOKBACK_DAYS, rows: 2 });
    } finally {
      if (originalCronSecret === undefined) delete process.env['CRON_SECRET'];
      else process.env['CRON_SECRET'] = originalCronSecret;
    }
  });

  it('still requires the CRON_SECRET bearer — the rollup is not publicly triggerable', async () => {
    const app = await loadCronApp();
    const originalCronSecret = process.env['CRON_SECRET'];
    process.env['CRON_SECRET'] = 'cron-secret';
    try {
      const response = await app.request('/cron/cleanup');
      expect(response.status).toBe(401);
      expect(cronMocks.rollupAccountActivity).not.toHaveBeenCalled();
    } finally {
      if (originalCronSecret === undefined) delete process.env['CRON_SECRET'];
      else process.env['CRON_SECRET'] = originalCronSecret;
    }
  });

  it('isolates a rollup failure: cleanup still 200s with its counts intact', async () => {
    cronMocks.cleanupExpiredDeviceCodes.mockResolvedValue(3);
    cronMocks.cleanupExpiredGithubDeviceSessions.mockResolvedValue(1);
    cronMocks.cleanupExpiredOtpCodes.mockResolvedValue(2);
    cronMocks.cleanupExpiredRefreshTokens.mockResolvedValue(5);
    cronMocks.cleanupExpiredBroadcastSnapshots.mockResolvedValue(0);
    cronMocks.rollupAndPurgeExpiredTelemetryBeacons.mockResolvedValue(9);
    cronMocks.rollupAccountActivity.mockRejectedValue(new Error('connection reset'));

    const app = await loadCronApp();
    const originalCronSecret = process.env['CRON_SECRET'];
    process.env['CRON_SECRET'] = 'cron-secret';
    try {
      const response = await app.request('/cron/cleanup', {
        headers: { Authorization: 'Bearer cron-secret' },
      });

      // A rejected rollup must never 500 the whole sweep — it would mask the
      // six successful cleanup steps and make Vercel Cron treat the run as
      // failed.
      expect(response.status).toBe(200);
      const body = (await response.json()) as {
        cleaned: Record<string, number>;
        activityRollup: { error: string };
      };
      expect(body.cleaned).toEqual({
        deviceCodes: 3,
        githubDeviceSessions: 1,
        otpCodes: 2,
        refreshTokens: 5,
        broadcastSnapshots: 0,
        telemetryBeacons: 9,
      });
      expect(body.activityRollup).toEqual({ error: 'connection reset' });
    } finally {
      if (originalCronSecret === undefined) delete process.env['CRON_SECRET'];
      else process.env['CRON_SECRET'] = originalCronSecret;
    }
  });
});
