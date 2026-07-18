import { Hono } from 'hono';
import { z } from 'zod';
import { zValidator } from '@hono/zod-validator';
import { randomBytes } from 'node:crypto';
import { getClient } from '../db/client.js';
import {
  findUserByEmail,
  insertOtpCodeIfUnderLimit,
  registerActiveOtpAttempts,
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

  // Rate-limit: cap active (unconsumed, unexpired) codes per user. The insert
  // is conditional and advisory-locked so concurrent requests cannot all
  // observe a sub-cap count and overshoot MAX_ACTIVE_CODES.
  const code = generateOtpCode();
  const codeHash = hashOtp(code);
  const expiresAt = new Date(Date.now() + OTP_EXPIRY_SECONDS * 1000);

  const inserted = await insertOtpCodeIfUnderLimit(
    sql,
    user.id,
    codeHash,
    expiresAt,
    MAX_ACTIVE_CODES
  );
  if (!inserted) {
    debug('otp rate limit reached', { userId: user.id });
    return c.json(SUCCESS_RESPONSE);
  }

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
 *
 * ACKNOWLEDGED RESIDUAL (self-lockout DoS): an unauthenticated caller who knows
 * only a victim's email can exhaust the victim's active codes (via /request)
 * and burn MAX_ATTEMPTS wrong guesses against them, locking OTP login for that
 * mailbox until the codes expire (OTP_EXPIRY_SECONDS). This is inherent to any
 * max-attempts scheme and is pre-existing — the CIB-142 fix only makes the cap
 * race-free, it does not (and cannot) remove the lockout primitive. GitHub
 * device-flow remains an unaffected alternative sign-in path.
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

  // Atomically register this verification attempt against every active
  // (unconsumed, unexpired) code that is still below MAX_ATTEMPTS, and get
  // back only the codes eligible for comparison. The increment and the
  // below-cap guard are a SINGLE conditional UPDATE, evaluated BEFORE the code
  // comparison, so N concurrent guesses cannot all read a stale attempts count
  // and slip past the cap (CIB-142). A code already at the cap is not returned,
  // so its hash is never compared — the guess is rejected without evaluating
  // the code.
  const eligibleCodes = await registerActiveOtpAttempts(sql, user.id, MAX_ATTEMPTS);

  if (eligibleCodes.length === 0) {
    // Either no active codes exist, or every active code has hit the attempt
    // cap. Both collapse to the same anti-enumeration error (and neither
    // reveals which, nor evaluates any code).
    debug('no eligible otp codes', { userId: user.id });
    return c.json(INVALID_CODE_ERROR, 400);
  }

  const submittedHash = hashOtp(code);
  const match = eligibleCodes.find((row) => row.code_hash === submittedHash);

  if (!match) {
    debug('otp verify failed', { userId: user.id, reason: 'no_match' });
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
