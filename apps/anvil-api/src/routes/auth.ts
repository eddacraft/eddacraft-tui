import { Hono } from 'hono';
import { z } from 'zod';
import { zValidator } from '@hono/zod-validator';
import { getClient } from '../db/client.js';
import { findTokenByHash } from '../db/queries.js';
import { hashToken, isValidTokenFormat } from '../lib/token.js';
import { createDebugger } from '../lib/debug.js';

const debug = createDebugger('api');

const verifySchema = z.object({
  token: z.string().max(200),
});

const auth = new Hono();

/**
 * POST /auth/verify
 *
 * Validates a beta access token.
 * Always returns 200 — {valid: false} on any failure (no reason leakage).
 */
auth.post('/verify', zValidator('json', verifySchema), async (c) => {
  debug('POST /auth/verify');
  const { token } = c.req.valid('json');

  if (!isValidTokenFormat(token)) {
    debug('invalid token format');
    return c.json({ valid: false });
  }

  const sql = getClient();
  const hash = hashToken(token);
  const record = await findTokenByHash(sql, hash);

  if (!record) {
    return c.json({ valid: false });
  }

  // Check revocation
  if (record.revoked_at) {
    debug('token revoked');
    return c.json({ valid: false });
  }

  // Check expiry
  if (new Date(record.expires_at).getTime() < Date.now()) {
    debug('token expired');
    return c.json({ valid: false });
  }

  // Check user status
  if (record.user_status !== 'active') {
    debug('user not active', { status: record.user_status });
    return c.json({ valid: false });
  }

  debug('token verified successfully');
  return c.json({
    valid: true,
    user: { email: record.email },
    scopes: record.scopes,
    expiresAt: record.expires_at,
  });
});

export { auth };
