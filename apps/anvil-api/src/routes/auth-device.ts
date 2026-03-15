import { Hono } from 'hono';
import { z } from 'zod';
import { zValidator } from '@hono/zod-validator';
import { randomBytes } from 'node:crypto';
import { getClient } from '../db/client.js';
import { findUserByEmail } from '../db/queries.js';
import { createDebugger } from '../lib/debug.js';

const debug = createDebugger('auth-device');

function generateUserCode(): string {
  return 'ANVIL-' + randomBytes(2).toString('hex').toUpperCase();
}

function generatePollToken(): string {
  return randomBytes(32).toString('hex');
}

const startSchema = z.object({
  email: z.string().email().max(254),
});

const authDevice = new Hono();

/**
 * POST /device/start
 *
 * Initiates the device code authentication flow.
 * Anti-enumeration: always returns the same response shape regardless of
 * whether the email belongs to an active user.
 */
authDevice.post('/start', zValidator('json', startSchema), async (c) => {
  const { email } = c.req.valid('json');
  debug('POST /device/start', { email });

  const normalised = email.toLowerCase().trim();
  const sql = getClient();

  const user = await findUserByEmail(sql, normalised);
  const isValid = user && user.status === 'active';

  const userCode = generateUserCode();
  const pollToken = generatePollToken();
  const expiresAt = new Date(Date.now() + 900_000); // 15 minutes

  if (isValid) {
    await sql`
      INSERT INTO device_codes (user_id, user_code, poll_token, expires_at)
      VALUES (${user.id}, ${userCode}, ${pollToken}, ${expiresAt.toISOString()})
    `;
    debug('device code created', { userCode });
  } else {
    debug('anti-enumeration response for unknown/inactive email');
  }

  const verificationUrl =
    process.env.ACTIVATE_URL ?? 'https://eddacraft.ai/auth/activate';

  return c.json({
    userCode,
    verificationUrl,
    pollToken,
    expiresIn: 900,
    interval: 5,
  });
});

export { authDevice };
