import { Hono } from 'hono';
import { z } from 'zod';
import { zValidator } from '@hono/zod-validator';
import { randomBytes, createHash } from 'node:crypto';
import { getClient } from '../db/client.js';
import { findUserByEmail } from '../db/queries.js';
import { sendOtpCode } from '../lib/email.js';
import { createDebugger } from '../lib/debug.js';

const debug = createDebugger('api');

const OTP_EXPIRY_SECONDS = 600;
const MAX_ACTIVE_CODES = 3;

function generateOtpCode(): string {
  const num = parseInt(randomBytes(4).toString('hex'), 16) % 900000 + 100000;
  return num.toString();
}

function hashOtp(code: string): string {
  const pepper = process.env['TOKEN_PEPPER'] ?? '';
  return createHash('sha256').update(pepper + code).digest('hex');
}

const requestSchema = z.object({
  email: z.string().email().max(254),
});

const SUCCESS_RESPONSE = { sent: true, expiresIn: OTP_EXPIRY_SECONDS } as const;

const authOtp = new Hono();

/**
 * POST /auth/otp/request
 *
 * Request a one-time verification code via email.
 * Always returns the same success shape regardless of whether the email
 * exists — this prevents user enumeration.
 */
authOtp.post('/request', zValidator('json', requestSchema), async (c) => {
  debug('POST /auth/otp/request');
  const { email } = c.req.valid('json');
  const normalised = email.toLowerCase().trim();

  const sql = getClient();
  const user = await findUserByEmail(sql, normalised);

  // Anti-enumeration: silently succeed for unknown or inactive users
  if (!user || user.status !== 'active') {
    debug('otp request for unknown or inactive email');
    return c.json(SUCCESS_RESPONSE);
  }

  // Rate-limit: cap active (unconsumed, unexpired) codes per user
  const activeRows = (await sql`
    SELECT COUNT(*)::int AS count FROM otp_codes
    WHERE user_id = ${user.id}
      AND consumed_at IS NULL
      AND expires_at > now()
  `) as { count: number }[];

  if (activeRows[0] && activeRows[0].count >= MAX_ACTIVE_CODES) {
    debug('otp rate limit reached', { userId: user.id });
    return c.json(SUCCESS_RESPONSE);
  }

  const code = generateOtpCode();
  const codeHash = hashOtp(code);
  const expiresAt = new Date(Date.now() + OTP_EXPIRY_SECONDS * 1000);

  await sql`
    INSERT INTO otp_codes (user_id, code_hash, expires_at)
    VALUES (${user.id}, ${codeHash}, ${expiresAt.toISOString()})
  `;

  const delivery = await sendOtpCode(normalised, code);
  if (!delivery.sent) {
    debug('otp email delivery failed', { code: delivery.code });
  }

  return c.json(SUCCESS_RESPONSE);
});

export { authOtp };
