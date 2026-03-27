import { randomBytes } from 'node:crypto';
import { Hono } from 'hono';
import { z } from 'zod';
import { zValidator } from '@hono/zod-validator';
import { adminAuth } from '../middleware/admin-auth.js';
import { getClient } from '../db/client.js';
import { findUserWithTokens } from '../db/queries.js';
import { generateToken, hashToken } from '../lib/token.js';
import { sendBetaInvite } from '../lib/email.js';
import { createDebugger } from '../lib/debug.js';
import { moveToApprovedAudience, removeFromBetaAudience } from '../lib/audience.js';

const debug = createDebugger('api');

const ALLOWED_SCOPES = ['beta', 'preview', 'internal'] as const;

const inviteSchema = z.object({
  email: z.string().email().max(254),
  name: z.string().max(200).optional(),
  notes: z.string().max(1000).optional(),
  days: z.number().int().positive().max(365).default(90),
  scopes: z.array(z.enum(ALLOWED_SCOPES)).default(['beta']),
});

const approveSchema = z.union([
  z.object({ email: z.string().email().max(254) }),
  z.object({ batch: z.number().int().min(1).max(100) }),
]);

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
 * Checks X-Admin-Actor header first, then falls back to 'admin'.
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
  debug('POST /admin/invite', { email, scopes, days });
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
  debug('POST /admin/revoke', { hasEmail: !!email, hasToken: !!token });
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
    removeFromBetaAudience(normalizedEmail).catch((err) => {
      console.error('Failed to remove from audience (non-fatal):', err);
    });
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
 * POST /admin/approve
 *
 * Approve a waitlisted email or batch of oldest unapproved entries.
 * Generates access token + device code, sends invite email.
 */
admin.post('/approve', zValidator('json', approveSchema), async (c) => {
  const body = c.req.valid('json');
  debug('POST /admin/approve', { hasEmail: 'email' in body, hasBatch: 'batch' in body });
  const sql = getClient();
  const actor = resolveAdminActor(c);

  const ACTIVATE_BASE = process.env.ACTIVATE_URL ?? 'https://eddacraft.ai/auth/activate';

  async function approveOne(email: string): Promise<{ email: string; expiresAt: string }> {
    const normalizedEmail = email.toLowerCase().trim();

    // Verify waitlisted
    const waitlistRows = await sql`SELECT id FROM waitlist WHERE email = ${normalizedEmail}`;
    const waitlistEntry = (waitlistRows as Record<string, unknown>[])[0];
    if (!waitlistEntry) {
      throw new Error(`not_found:${normalizedEmail}`);
    }

    // Generate access token (90-day expiry)
    const rawToken = generateToken();
    const hash = hashToken(rawToken);
    const tokenExpiry = new Date();
    tokenExpiry.setDate(tokenExpiry.getDate() + 90);

    // Generate device code for invite email (48-hour expiry)
    const userCode = 'ANVIL-' + randomBytes(2).toString('hex').toUpperCase();
    const pollToken = randomBytes(32).toString('hex');
    const deviceExpiry = new Date();
    deviceExpiry.setTime(deviceExpiry.getTime() + 48 * 60 * 60 * 1000);

    // Transaction: upsert user + insert token + insert device code + audit log
    await sql.transaction([
      sql`INSERT INTO beta_users (email, status)
          VALUES (${normalizedEmail}, ${'active'})
          ON CONFLICT (email) DO UPDATE SET status = ${'active'}
          RETURNING *`,
      sql`INSERT INTO access_tokens (user_id, token_hash, scopes, expires_at)
          VALUES (
            (SELECT id FROM beta_users WHERE email = ${normalizedEmail}),
            ${hash}, ${['beta']}, ${tokenExpiry.toISOString()}
          )
          RETURNING *`,
      sql`INSERT INTO device_codes (user_id, user_code, poll_token, expires_at)
          VALUES (
            (SELECT id FROM beta_users WHERE email = ${normalizedEmail}),
            ${userCode}, ${pollToken}, ${deviceExpiry.toISOString()}
          )
          RETURNING *`,
      sql`INSERT INTO audit_log (action, actor, metadata)
          VALUES (${'user.approved'}, ${actor}, ${JSON.stringify({ email: normalizedEmail })})
          RETURNING *`,
    ]);

    // Move from waitlist to beta audience (best-effort)
    moveToApprovedAudience(normalizedEmail).catch((err) => {
      console.error('Failed to move audience (non-fatal):', err);
    });

    // Best-effort email — don't fail the approval if email fails
    const activateUrl = `${ACTIVATE_BASE}?code=${userCode}`;
    await sendBetaInvite(normalizedEmail, userCode, activateUrl).catch((err) => {
      console.error('Failed to send invite email (non-fatal):', err);
    });

    return { email: normalizedEmail, expiresAt: tokenExpiry.toISOString() };
  }

  if ('email' in body) {
    try {
      const result = await approveOne(body.email);
      return c.json({ approved: [result] }, 200);
    } catch (err) {
      if (err instanceof Error && err.message.startsWith('not_found:')) {
        return c.json({ error: 'Email not found on waitlist' }, 404);
      }
      throw err;
    }
  }

  // Batch mode: oldest N unapproved waitlist entries
  const unapproved = (await sql`
    SELECT w.email FROM waitlist w
    LEFT JOIN beta_users bu ON bu.email = w.email
    WHERE bu.id IS NULL
    ORDER BY w.created_at ASC
    LIMIT ${body.batch}
  `) as { email: string }[];

  const approved: { email: string; expiresAt: string }[] = [];
  for (const row of unapproved) {
    try {
      const result = await approveOne(row.email);
      approved.push(result);
    } catch (err) {
      debug('Batch approve skip', { email: row.email, error: String(err) });
    }
  }

  return c.json({ approved }, 200);
});

/**
 * GET /admin/user/:email
 *
 * Lookup a user and their token info.
 */
admin.get('/user/:email', async (c) => {
  debug('GET /admin/user/:email');
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
