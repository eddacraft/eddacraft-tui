import { Hono } from 'hono';
import { z } from 'zod';
import { zValidator } from '@hono/zod-validator';
import { adminAuth } from '../middleware/admin-auth.js';
import { getClient } from '../db/client.js';
import { findUserWithTokens } from '../db/queries.js';
import { generateToken, hashToken } from '../lib/token.js';

const ALLOWED_SCOPES = ['beta', 'preview', 'internal'] as const;

const inviteSchema = z.object({
  email: z.string().email().max(254),
  name: z.string().max(200).optional(),
  notes: z.string().max(1000).optional(),
  days: z.number().int().positive().max(365).default(90),
  scopes: z.array(z.enum(ALLOWED_SCOPES)).default(['beta']),
});

const revokeSchema = z
  .object({
    email: z.string().email().max(254).optional(),
    token: z.string().max(200).optional(),
  })
  .refine((data) => data.email || data.token, {
    message: 'Either email or token must be provided',
  });

const admin = new Hono();

// All admin routes require admin auth
admin.use('*', adminAuth);

/**
 * Resolve the admin actor identity for audit logging.
 * Checks X-Admin-Actor header first, then falls back to source IP.
 */
function resolveAdminActor(c: { req: { header: (name: string) => string | undefined } }): string {
  const actor = c.req.header('X-Admin-Actor');
  if (actor && actor.length <= 200) {
    // Sanitise: strip control characters, keep printable ASCII
    return actor.replace(/[^\x20-\x7E]/g, '').trim() || 'admin';
  }
  return 'admin';
}

/**
 * POST /admin/invite
 *
 * Creates or finds a user by email, generates a new token.
 * Returns the raw token exactly once — it is never stored.
 */
admin.post('/invite', zValidator('json', inviteSchema), async (c) => {
  const { email, name, notes, days, scopes } = c.req.valid('json');
  const sql = getClient();
  const actor = resolveAdminActor(c);

  const normalizedEmail = email.toLowerCase().trim();

  const rawToken = generateToken();
  const hash = hashToken(rawToken);
  const expiresAt = new Date();
  expiresAt.setDate(expiresAt.getDate() + days);

  // Transaction: upsert user + insert token + audit log atomically
  const txResult = await sql.transaction([
    sql`INSERT INTO beta_users (email, name, notes)
        VALUES (${normalizedEmail}, ${name ?? null}, ${notes ?? null})
        ON CONFLICT (email) DO UPDATE SET
          name = COALESCE(${name ?? null}, beta_users.name),
          notes = COALESCE(${notes ?? null}, beta_users.notes)
        RETURNING *`,
    sql`INSERT INTO access_tokens (user_id, token_hash, scopes, expires_at)
        VALUES (
          (SELECT id FROM beta_users WHERE email = ${normalizedEmail}),
          ${hash}, ${scopes}, ${expiresAt.toISOString()}
        )
        RETURNING *`,
    sql`INSERT INTO audit_log (action, actor, metadata)
        VALUES (${'token.created'}, ${actor}, ${JSON.stringify({ email: normalizedEmail, scopes, days })})
        RETURNING *`,
  ]);

  const user = (txResult as unknown[][])[0]?.[0] as { email: string; id: string };

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
  const actor = resolveAdminActor(c);

  if (email) {
    const normalizedEmail = email.toLowerCase().trim();
    // Transaction: revoke tokens + audit log atomically
    const txResult = await sql.transaction([
      sql`UPDATE access_tokens SET revoked_at = now()
          WHERE user_id = (SELECT id FROM beta_users WHERE email = ${normalizedEmail})
            AND revoked_at IS NULL
          RETURNING id`,
      sql`INSERT INTO audit_log (action, actor, metadata)
          VALUES (${'tokens.revoked'}, ${actor}, ${JSON.stringify({ email: normalizedEmail })})
          RETURNING *`,
    ]);
    const revokedRows = (txResult as unknown[][])[0] ?? [];
    return c.json({ revoked: revokedRows.length });
  }

  if (token) {
    const hash = hashToken(token);
    // Transaction: revoke token + audit log atomically
    const txResult = await sql.transaction([
      sql`UPDATE access_tokens SET revoked_at = now()
          WHERE token_hash = ${hash}
            AND revoked_at IS NULL
          RETURNING id`,
      sql`INSERT INTO audit_log (action, actor, metadata)
          VALUES (${'token.revoked'}, ${actor}, ${JSON.stringify({ revoked: true })})
          RETURNING *`,
    ]);
    const revokedRows = (txResult as unknown[][])[0] ?? [];
    return c.json({ revoked: revokedRows.length > 0 ? 1 : 0 });
  }

  return c.json({ error: 'Either email or token must be provided' }, 400);
});

/**
 * GET /admin/user/:email
 *
 * Lookup a user and their token info.
 */
admin.get('/user/:email', async (c) => {
  const rawEmail = c.req.param('email');
  const emailResult = z.string().email().max(254).safeParse(rawEmail);
  if (!emailResult.success) {
    return c.json({ error: 'Invalid email format' }, 400);
  }
  const email = emailResult.data.toLowerCase().trim();
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
