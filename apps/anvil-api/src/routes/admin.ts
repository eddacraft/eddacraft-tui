import { Hono } from 'hono';
import { z } from 'zod';
import { zValidator } from '@hono/zod-validator';
import { adminAuth } from '../middleware/admin-auth.js';
import { getClient } from '../db/client.js';
import {
  upsertUser,
  insertToken,
  revokeTokensByEmail,
  revokeTokenByHash,
  findUserWithTokens,
  insertAuditLog,
} from '../db/queries.js';
import { generateToken, hashToken } from '../lib/token.js';

const inviteSchema = z.object({
  email: z.string().email(),
  name: z.string().optional(),
  notes: z.string().optional(),
  days: z.number().int().positive().default(90),
  scopes: z.array(z.string()).default(['beta']),
});

const revokeSchema = z
  .object({
    email: z.string().email().optional(),
    token: z.string().optional(),
  })
  .refine((data) => data.email || data.token, {
    message: 'Either email or token must be provided',
  });

const admin = new Hono();

// All admin routes require admin auth
admin.use('*', adminAuth);

/**
 * POST /admin/invite
 *
 * Creates or finds a user by email, generates a new token.
 * Returns the raw token exactly once — it is never stored.
 */
admin.post('/invite', zValidator('json', inviteSchema), async (c) => {
  const { email, name, notes, days, scopes } = c.req.valid('json');
  const sql = getClient();

  const normalizedEmail = email.toLowerCase().trim();
  const user = await upsertUser(sql, normalizedEmail, name, notes);

  const rawToken = generateToken();
  const hash = hashToken(rawToken);
  const expiresAt = new Date();
  expiresAt.setDate(expiresAt.getDate() + days);

  await insertToken(sql, user.id, hash, scopes, expiresAt);

  await insertAuditLog(sql, 'token.created', 'admin', {
    email: normalizedEmail,
    scopes,
    days,
  });

  return c.json(
    {
      token: rawToken,
      user: { email: user.email, id: user.id },
      expiresAt: expiresAt.toISOString(),
      scopes,
    },
    201
  );
});

/**
 * POST /admin/revoke
 *
 * Revoke tokens by email (all tokens) or by specific token.
 */
admin.post('/revoke', zValidator('json', revokeSchema), async (c) => {
  const { email, token } = c.req.valid('json');
  const sql = getClient();

  if (email) {
    const count = await revokeTokensByEmail(sql, email.toLowerCase().trim());
    await insertAuditLog(sql, 'tokens.revoked', 'admin', {
      email: email.toLowerCase().trim(),
      count,
    });
    return c.json({ revoked: count });
  }

  if (token) {
    const hash = hashToken(token);
    const revoked = await revokeTokenByHash(sql, hash);
    await insertAuditLog(sql, 'token.revoked', 'admin', {
      revoked,
    });
    return c.json({ revoked: revoked ? 1 : 0 });
  }

  return c.json({ error: 'Either email or token must be provided' }, 400);
});

/**
 * GET /admin/user/:email
 *
 * Lookup a user and their token info.
 */
admin.get('/user/:email', async (c) => {
  const email = c.req.param('email').toLowerCase().trim();
  const sql = getClient();

  const result = await findUserWithTokens(sql, email);
  if (!result) {
    return c.json({ error: 'User not found' }, 404);
  }

  return c.json({
    user: result.user,
    tokens: result.tokens.map((t) => ({
      id: t.id,
      scopes: t.scopes,
      expires_at: t.expires_at,
      revoked_at: t.revoked_at,
      created_at: t.created_at,
    })),
  });
});

export { admin };
