import { timingSafeEqual } from 'node:crypto';
import { Hono } from 'hono';
import { getClient } from '../db/client.js';
import { createDebugger } from '../lib/debug.js';

const debug = createDebugger('api');

const cron = new Hono();

/**
 * GET /cron/cleanup
 *
 * Purge expired device codes and OTP codes.
 * Retains codes for 1 hour after expiry to allow for clock skew and
 * debugging before cleanup. Runs hourly via Vercel Cron.
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

  const deviceResult = await sql`
    DELETE FROM device_codes
    WHERE expires_at < now() - interval '1 hour'
    RETURNING id
  `;

  const otpResult = await sql`
    DELETE FROM otp_codes
    WHERE expires_at < now() - interval '1 hour'
    RETURNING id
  `;

  const deviceCount = Array.isArray(deviceResult) ? deviceResult.length : 0;
  const otpCount = Array.isArray(otpResult) ? otpResult.length : 0;

  debug('cleanup complete', { deviceCodes: deviceCount, otpCodes: otpCount });

  return c.json({
    cleaned: {
      deviceCodes: deviceCount,
      otpCodes: otpCount,
    },
  });
});

export { cron };
