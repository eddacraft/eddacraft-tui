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
  rollupAccountActivity,
} from '../db/queries.js';
import { createDebugger } from '../lib/debug.js';
import { getTelemetryRetentionDays } from '../lib/telemetry-retention.js';
import { DEFAULT_ROLLUP_LOOKBACK_DAYS, completedUtcDays } from '../lib/account-activity-rollup.js';

const debug = createDebugger('api');

const cron = new Hono();

/**
 * GET /cron/cleanup
 *
 * Purge expired device codes, OTP codes, and refresh tokens. Also rolls up
 * the trailing window of completed-UTC-day account-activity counts
 * (BACT-011, ADR-121 OQ-A) — see `lib/account-activity-rollup.ts`.
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
  // BACT-011 (ADR-121 OQ-A): daily historical-DAA rollup piggybacks on this
  // existing hourly sweep rather than a new scheduling mechanism. Recomputes
  // a small trailing window of completed UTC days on every run (upsert —
  // idempotent, never double-counts) so a short outage self-heals; a day
  // left un-rolled longer than the window is subject to the late-rollup
  // undercount documented in lib/account-activity-rollup.ts.
  //
  // Deliberately error-isolated from the six cleanup steps above: a rollup
  // rejection must never 500 the whole sweep (that would mask successful
  // cleanup and make Vercel Cron treat the whole run as failed). On failure
  // we still return 200 with the cleanup counts intact and surface the
  // failure in `activityRollup.error` instead.
  const rollupDays = completedUtcDays(new Date(), DEFAULT_ROLLUP_LOOKBACK_DAYS);
  let activityRollup: { days: number; rows: number } | { error: string };
  try {
    const activityRollupRows = await rollupAccountActivity(sql, rollupDays);
    activityRollup = { days: rollupDays.length, rows: activityRollupRows.length };
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    debug('activity rollup failed (isolated from cleanup)', { error: message });
    activityRollup = { error: message };
  }

  debug('cleanup complete', {
    deviceCodes: deviceCount,
    githubDeviceSessions: githubDeviceSessionCount,
    otpCodes: otpCount,
    refreshTokens: refreshCount,
    broadcastSnapshots: broadcastSnapshotCount,
    telemetryBeacons: telemetryBeaconCount,
    activityRollup,
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
    activityRollup,
  });
});

export { cron };
