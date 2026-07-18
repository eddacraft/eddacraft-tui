import { Hono } from 'hono';
import { z } from 'zod';
import { zValidator } from '@hono/zod-validator';
import { getClient } from '../db/client.js';
import {
  findRefreshTokenByHash,
  revokeRefreshFamilyAndAccessTokensForUser,
  findUserById,
} from '../db/queries.js';
import { hashToken } from '../lib/token.js';
import { mintRotatedSession } from '../lib/session.js';
import { createDebugger } from '../lib/debug.js';

const debug = createDebugger('auth-session');

const refreshSchema = z.object({
  refreshToken: z.string().min(1).max(200),
});

const authSession = new Hono();

/**
 * POST /session/refresh
 *
 * Rotates a refresh token and issues a new JWT licence.
 * Implements family-based theft detection: if a consumed token is reused,
 * the entire token family is revoked.
 */
authSession.post('/refresh', zValidator('json', refreshSchema), async (c) => {
  const { refreshToken } = c.req.valid('json');
  debug('POST /session/refresh');

  const sql = getClient();
  const tokenHash = hashToken(refreshToken);

  const record = await findRefreshTokenByHash(sql, tokenHash);

  // Token not found
  if (!record) {
    debug('refresh token not found');
    return c.json({ error: 'Invalid refresh token' }, 401);
  }

  // Theft detection: token already consumed means it was cloned
  if (record.consumed_at) {
    debug('token reuse detected — revoking family', { familyId: record.family_id });
    await revokeRefreshFamilyAndAccessTokensForUser(sql, record.family_id, record.user_id);
    return c.json({ error: 'Token reuse detected' }, 401);
  }

  // Token revoked
  if (record.revoked_at) {
    debug('refresh token revoked');
    return c.json({ error: 'Invalid refresh token' }, 401);
  }

  // Token expired
  if (new Date(record.expires_at).getTime() < Date.now()) {
    debug('refresh token expired');
    return c.json({ error: 'Refresh token expired' }, 401);
  }

  // Verify user is still active
  const user = await findUserById(sql, record.user_id);
  if (!user || user.status !== 'active') {
    debug('user not active', { userId: record.user_id });
    return c.json({ error: 'User account is not active' }, 401);
  }

  // Atomically consume the old token and insert its replacement. A concurrent
  // request that loses the race (or triggers family revocation) cannot leave
  // a live post-revoke refresh token for the winner to return.
  const rotated = await mintRotatedSession(sql, {
    user,
    identity: { provider: 'email', id: null },
    familyId: record.family_id,
    oldTokenId: record.id,
  });
  if (!rotated.ok) {
    debug('concurrent refresh or family revoke — revoking family', {
      familyId: record.family_id,
    });
    await revokeRefreshFamilyAndAccessTokensForUser(sql, record.family_id, record.user_id);
    return c.json({ error: 'Token reuse detected' }, 401);
  }

  debug('session refreshed', { userId: user.id, familyId: record.family_id });

  return c.json(rotated.session);
});

export { authSession };
