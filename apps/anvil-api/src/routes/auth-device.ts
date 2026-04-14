import { Hono } from 'hono';
import { z } from 'zod';
import { zValidator } from '@hono/zod-validator';
import { randomBytes, randomUUID } from 'node:crypto';
import { getClient } from '../db/client.js';
import {
  findUserByEmail,
  findUserById,
  insertDeviceCode,
  insertDummyDeviceCode,
  findPendingDeviceCodeWithEmail,
  confirmDeviceCode,
  pollDeviceCode,
  deviceCodeExistsByPollToken,
  consumeDeviceCode,
  insertRefreshToken,
} from '../db/queries.js';
import { createDebugger } from '../lib/debug.js';
import { signLicence, type LicenceClaims } from '../lib/licence.js';
import { hashToken } from '../lib/token.js';

const debug = createDebugger('auth-device');

const REFRESH_TOKEN_EXPIRY_DAYS = 90;
const POLL_INTERVAL_S = 5;

const MAX_USER_CODE_RETRIES = 3;

function generateUserCode(): string {
  return 'ANVIL-' + randomBytes(4).toString('hex').toUpperCase();
}

function generatePollToken(): string {
  return randomBytes(32).toString('hex');
}

const startSchema = z.object({
  email: z.string().email().max(254),
});

const confirmSchema = z.object({
  userCode: z.string().min(1).max(20),
  email: z.string().email().max(254),
});

const pollSchema = z.object({
  pollToken: z.string().min(1).max(200),
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
  debug('POST /device/start', { hasEmail: Boolean(email) });

  const normalised = email.toLowerCase().trim();
  const sql = getClient();

  const user = await findUserByEmail(sql, normalised);
  const isValid = user && user.status === 'active';

  const pollToken = generatePollToken();
  const pollTokenHash = hashToken(pollToken);
  const expiresAt = new Date(Date.now() + 900_000); // 15 minutes

  let userCode = '';
  for (let attempt = 0; attempt < MAX_USER_CODE_RETRIES; attempt++) {
    userCode = generateUserCode();
    try {
      if (isValid) {
        await insertDeviceCode(sql, user.id, userCode, pollTokenHash, expiresAt);
        debug('device code created', { userCode });
      } else {
        // F-C-003: insert a dummy row so /confirm has identical DB-query timing
        // for non-existent users. The null user_id ensures the JOIN in /confirm
        // never matches, and the row expires normally via cron cleanup.
        await insertDummyDeviceCode(sql, userCode, pollTokenHash, expiresAt);
        debug('anti-enumeration dummy device code created');
      }
      break;
    } catch (err: unknown) {
      const isUniqueViolation =
        err instanceof Error && 'code' in err && (err as { code: string }).code === '23505';
      if (!isUniqueViolation || attempt === MAX_USER_CODE_RETRIES - 1) throw err;
      debug('user_code collision, retrying', { attempt });
    }
  }

  const verificationUrl = process.env.ACTIVATE_URL ?? 'https://eddacraft.ai/auth/activate';

  return c.json({
    userCode,
    verificationUrl,
    pollToken,
    expiresIn: 900,
    interval: POLL_INTERVAL_S,
  });
});

/**
 * POST /device/confirm
 *
 * Confirms a device code from the browser activation page.
 * Anti-enumeration: returns identical error for all failure modes.
 */
authDevice.post('/confirm', zValidator('json', confirmSchema), async (c) => {
  const { userCode, email } = c.req.valid('json');
  debug('POST /device/confirm');

  const normalisedCode = userCode.toUpperCase().trim();
  const normalisedEmail = email.toLowerCase().trim();
  const sql = getClient();

  const result = await findPendingDeviceCodeWithEmail(sql, normalisedCode);

  if (!result || result.user_email !== normalisedEmail) {
    return c.json({ error: 'Invalid or expired code' }, 400);
  }

  await confirmDeviceCode(sql, result.id);

  return c.json({ confirmed: true });
});

/**
 * POST /auth/device/poll
 *
 * Polls for the status of a device code flow. The CLI calls this repeatedly
 * until the user confirms the code on the web or the code expires.
 *
 * Rate limiting: per-token cooldown enforced via `last_polled_at` column.
 * Returns 429 with `slow_down` error if polled within the 5-second interval.
 */
authDevice.post('/poll', zValidator('json', pollSchema), async (c) => {
  debug('POST /auth/device/poll');
  const { pollToken } = c.req.valid('json');
  const sql = getClient();
  const pollTokenHash = hashToken(pollToken);

  // F-C-006: atomic per-token rate limiting — single UPDATE-with-WHERE
  // eliminates TOCTOU race between reading and stamping last_polled_at
  const deviceCode = await pollDeviceCode(sql, pollTokenHash, POLL_INTERVAL_S);

  if (!deviceCode) {
    // Either the token doesn't exist or the cooldown hasn't elapsed.
    // Distinguish "not found" from "rate limited" with a read-only check.
    const exists = await deviceCodeExistsByPollToken(sql, pollTokenHash);

    if (exists) {
      debug('poll rate limited (too frequent)');
      return c.json({ error: 'slow_down', retryAfter: POLL_INTERVAL_S }, 429);
    }

    // Anti-enumeration is handled upstream: /start inserts dummy rows for
    // inactive users (F-C-003), so both valid and invalid users get identical
    // polling behaviour. A token with no row was never issued — return expired.
    debug('device code not found (no matching row — treating as expired)');
    return c.json({ status: 'expired' });
  }

  // Check expiry BEFORE any further side-effects (session minting)
  const expiresAt = new Date(deviceCode.expires_at);
  if (expiresAt.getTime() < Date.now()) {
    debug('device code expired');
    return c.json({ status: 'expired' });
  }

  if (!deviceCode.confirmed_at) {
    debug('device code pending');
    return c.json({ status: 'pending' });
  }

  // Confirmed — atomically consume the device code so concurrent polls
  // cannot both mint sessions (DELETE ... RETURNING ensures single-use)
  const consumed = await consumeDeviceCode(sql, pollTokenHash);

  if (!consumed) {
    debug('device code already consumed by concurrent request');
    return c.json({ status: 'expired' });
  }

  debug('device code confirmed, issuing licence');
  const userId = consumed.user_id;

  const user = await findUserById(sql, userId);

  if (!user || user.status !== 'active') {
    debug('user not found or not active for confirmed device code');
    return c.json({ status: 'expired' });
  }

  const claims: LicenceClaims = {
    sub: user.id,
    email: user.email,
    identity: { provider: 'email', id: null },
    org: null,
    tier: 'pro',
    scopes: ['beta'],
    seats: 1,
  };
  const license = await signLicence(claims, undefined, 7);

  const rawRefreshToken = randomBytes(32).toString('hex');
  const refreshHash = hashToken(rawRefreshToken);
  const familyId = randomUUID();
  const refreshExpiresAt = new Date(Date.now() + REFRESH_TOKEN_EXPIRY_DAYS * 24 * 60 * 60 * 1000);

  await insertRefreshToken(sql, userId, refreshHash, familyId, refreshExpiresAt);

  const jwtExpiresAt = new Date(Date.now() + 7 * 24 * 60 * 60 * 1000);
  debug('device code consumed, licence issued');

  return c.json({
    status: 'confirmed',
    license,
    refreshToken: rawRefreshToken,
    expiresAt: jwtExpiresAt.toISOString(),
  });
});

export { authDevice };
