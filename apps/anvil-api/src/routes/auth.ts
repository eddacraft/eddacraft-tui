import { Hono } from 'hono';
import { z } from 'zod';
import { zValidator } from '@hono/zod-validator';
import { getClient } from '../db/client.js';
import { findTokenByHash } from '../db/queries.js';
import { hashToken, isValidTokenFormat } from '../lib/token.js';

const verifySchema = z.object({
  token: z.string(),
});

const auth = new Hono();

/**
 * POST /auth/verify
 *
 * Validates a beta access token.
 * Always returns 200 — {valid: false} on any failure (no reason leakage).
 */
auth.post('/verify', zValidator('json', verifySchema), async (c) => {
  const { token } = c.req.valid('json');

  if (!isValidTokenFormat(token)) {
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
    return c.json({ valid: false });
  }

  // Check expiry
  if (new Date(record.expires_at) < new Date()) {
    return c.json({ valid: false });
  }

  // Check user status
  if (record.user_status !== 'active') {
    return c.json({ valid: false });
  }

  return c.json({
    valid: true,
    user: { email: record.email },
    scopes: record.scopes,
    expiresAt: record.expires_at,
  });
});

export { auth };
