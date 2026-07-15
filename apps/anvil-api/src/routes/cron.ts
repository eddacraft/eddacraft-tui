import { timingSafeEqual } from 'node:crypto';
import { Hono } from 'hono';
import { getClient } from '../db/client.js';
import {
  cleanupExpiredBroadcastSnapshots,
  cleanupExpiredDeviceCodes,
  cleanupExpiredGithubDeviceSessions,
  cleanupExpiredOtpCodes,
  cleanupExpiredRefreshTokens,
  rollupAndPurgeExpiredTelemetryBeacons,
} from '../db/queries.js';
import { createDebugger } from '../lib/debug.js';
import { getTelemetryRetentionDays } from '../lib/telemetry-retention.js';

const debug = createDebugger('api');

const cron = new Hono();

/**
 * GET /cron/cleanup
 *
 * Purge expired device codes, OTP codes, and refresh tokens.
 * Retains codes for 1 hour after expiry to allow for clock skew and
 * debugging before cleanup. Revoked refresh tokens are kept for 7 days
 * for audit purposes. Runs hourly via Vercel Cron.
 * Protected by CRON_SECRET for Vercel Cron compatibility.
 *
 * Vercel Cron config (vercel.json):
 *   { "path": "/api/v1/cron/cleanup", "schedule": "0 * * * *" }
 */
cron.get('/cleanup', async (c) => {
  const cronSecret = process.env.CRON_SECRET;
  if (!cronSecret) {
    return c.json({ error: 'Unauthorized' }, 401);
  }

  const authHeader = c.req.header('Authorization') ?? '';
  const expected = `Bearer ${cronSecret}`;
  const a = Buffer.from(authHeader, 'utf-8');
  const b = Buffer.from(expected, 'utf-8');
  if (a.length !== b.length || !timingSafeEqual(a, b)) {
    return c.json({ error: 'Unauthorized' }, 401);
  }

  debug('GET /cron/cleanup');
  const sql = getClient();

  const deviceCount = await cleanupExpiredDeviceCodes(sql);
  const githubDeviceSessionCount = await cleanupExpiredGithubDeviceSessions(sql);
  const otpCount = await cleanupExpiredOtpCodes(sql);
  const refreshCount = await cleanupExpiredRefreshTokens(sql);
  const broadcastSnapshotCount = await cleanupExpiredBroadcastSnapshots(sql);
  // FLEET-005 (ADR-107 §6): raw beacons older than the retention window are
  // rolled up into the daily aggregate tables, then purged. The window is
  // configuration (default 90 days; TELEMETRY_RETENTION_DAYS overrides).
  const telemetryBeaconCount = await rollupAndPurgeExpiredTelemetryBeacons(
    sql,
    getTelemetryRetentionDays()
  );

  debug('cleanup complete', {
    deviceCodes: deviceCount,
    githubDeviceSessions: githubDeviceSessionCount,
    otpCodes: otpCount,
    refreshTokens: refreshCount,
    broadcastSnapshots: broadcastSnapshotCount,
    telemetryBeacons: telemetryBeaconCount,
  });

  return c.json({
    cleaned: {
      deviceCodes: deviceCount,
      githubDeviceSessions: githubDeviceSessionCount,
      otpCodes: otpCount,
      refreshTokens: refreshCount,
      broadcastSnapshots: broadcastSnapshotCount,
      telemetryBeacons: telemetryBeaconCount,
    },
  });
});

export { cron };
