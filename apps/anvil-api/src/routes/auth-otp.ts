import { Hono } from 'hono';
import { z } from 'zod';
import { zValidator } from '@hono/zod-validator';
import { randomBytes } from 'node:crypto';
import { getClient } from '../db/client.js';
import {
  findUserByEmail,
  countActiveOtpCodes,
  insertOtpCode,
  findActiveOtpCodes,
  incrementOtpAttemptsBatch,
  consumeOtpCode,
} from '../db/queries.js';
import { sendOtpCode } from '../lib/email.js';
import { mintSession } from '../lib/session.js';
import { hashToken } from '../lib/token.js';
import { createDebugger } from '../lib/debug.js';

const debug = createDebugger('api');

const OTP_EXPIRY_SECONDS = 600;
const MAX_ACTIVE_CODES = 3;
const MAX_ATTEMPTS = 3;

function generateOtpCode(): string {
  const num = (parseInt(randomBytes(4).toString('hex'), 16) % 900000) + 100000;
  return num.toString();
}

/** Hash an OTP code using the shared token hashing strategy. */
const hashOtp = hashToken;

const requestSchema = z.object({
  email: z.string().email().max(254),
});

const verifySchema = z.object({
  email: z.string().email().max(254),
  code: z.string().regex(/^\d{6}$/, 'Code must be exactly 6 digits'),
});

const SUCCESS_RESPONSE = { sent: true, expiresIn: OTP_EXPIRY_SECONDS } as const;
const INVALID_CODE_ERROR = { error: 'Invalid or expired code' } as const;

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
  const activeCount = await countActiveOtpCodes(sql, user.id);

  if (activeCount >= MAX_ACTIVE_CODES) {
    debug('otp rate limit reached', { userId: user.id });
    return c.json(SUCCESS_RESPONSE);
  }

  const code = generateOtpCode();
  const codeHash = hashOtp(code);
  const expiresAt = new Date(Date.now() + OTP_EXPIRY_SECONDS * 1000);

  await insertOtpCode(sql, user.id, codeHash, expiresAt);

  const delivery = await sendOtpCode(normalised, code);
  if (!delivery.sent) {
    debug('otp email delivery failed', { code: delivery.code });
  }

  return c.json(SUCCESS_RESPONSE);
});

/**
 * POST /auth/otp/verify
 *
 * Exchange a 6-digit OTP code for a JWT licence and refresh token.
 * Returns identical error shapes for all failure modes (anti-enumeration).
 */
authOtp.post('/verify', zValidator('json', verifySchema), async (c) => {
  debug('POST /auth/otp/verify');
  const { email, code } = c.req.valid('json');
  const normalised = email.toLowerCase().trim();

  const sql = getClient();
  const user = await findUserByEmail(sql, normalised);

  // Anti-enumeration: same error for unknown or inactive users
  if (!user || user.status !== 'active') {
    debug('otp verify for unknown or inactive email');
    return c.json(INVALID_CODE_ERROR, 400);
  }

  // Find active OTP codes for this user (unconsumed, unexpired)
  const activeCodes = await findActiveOtpCodes(sql, user.id);

  if (activeCodes.length === 0) {
    debug('no active otp codes', { userId: user.id });
    return c.json(INVALID_CODE_ERROR, 400);
  }

  const submittedHash = hashOtp(code);
  const match = activeCodes.find((row) => row.code_hash === submittedHash);

  // Check if any code has exceeded max attempts, or no match found
  if (!match || match.attempts >= MAX_ATTEMPTS) {
    // Increment attempts on all active codes
    const activeIds = activeCodes.map((row) => row.id);
    await incrementOtpAttemptsBatch(sql, activeIds);
    debug('otp verify failed', { userId: user.id, reason: match ? 'max_attempts' : 'no_match' });
    return c.json(INVALID_CODE_ERROR, 400);
  }

  // Consume the matched OTP code atomically so concurrent verification
  // requests cannot both use the same code.
  const consumed = await consumeOtpCode(sql, match.id);

  if (!consumed) {
    debug('otp verify failed', { userId: user.id, reason: 'already_consumed_or_expired' });
    return c.json(INVALID_CODE_ERROR, 400);
  }

  // Mint the licence + refresh token. Scope resolution carries a graded-scope
  // grant (e.g. `preview`) through the OTP path; defaults to `['beta']`
  // (FLAGM-005 — auth-otp was missed in the eae47b3d round).
  const session = await mintSession(sql, {
    user,
    identity: { provider: 'email', id: null },
  });

  debug('otp verified successfully', { userId: user.id });

  return c.json(session);
});

export { authOtp };
