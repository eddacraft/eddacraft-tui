import { randomBytes } from 'node:crypto';
import { Hono } from 'hono';
import { z } from 'zod';
import { zValidator } from '@hono/zod-validator';
import { adminAuth } from '../middleware/admin-auth.js';
import { getClient } from '../db/client.js';
import {
  findUserWithTokens,
  insertAuditLog,
  upsertWaitlistWithName,
  findWaitlistEntryByEmail,
  findUnapprovedWaitlistEntries,
  findWaitlistBySource,
} from '../db/queries.js';
import { generateToken, hashToken } from '../lib/token.js';
import { sendBetaInvite, sendWaitlistMigration } from '../lib/email.js';
import { createDebugger } from '../lib/debug.js';
import { moveToApprovedAudience, removeFromBetaAudience } from '../lib/audience.js';
import { isUserCodeCollision, withUserCodeRetry } from '../lib/device-code.js';

const debug = createDebugger('api');

const ALLOWED_SCOPES = ['beta', 'preview', 'internal'] as const;

const inviteSchema = z.object({
  email: z.string().email().max(254),
  name: z.string().max(200).optional(),
  notes: z.string().max(1000).optional(),
  days: z.number().int().positive().max(365).default(90),
  scopes: z.array(z.enum(ALLOWED_SCOPES)).default(['beta']),
  tokenOnly: z.boolean().default(false),
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
 * Invite a user to the beta. Two modes:
 *
 * Default (tokenOnly=false): insert into waitlist with source 'manual',
 * then run the full approve flow (upsert user, device code, invite email).
 *
 * tokenOnly=true: upsert user + waitlist entry, generate a raw access
 * token returned exactly once. For CI/service accounts.
 */
admin.post('/invite', zValidator('json', inviteSchema), async (c) => {
  const { email, name, notes, days, scopes, tokenOnly } = c.req.valid('json');
  debug('POST /admin/invite', { email, scopes, days, tokenOnly });
  const sql = getClient();
  const actor = resolveAdminActor(c);

  const normalizedEmail = email.toLowerCase().trim();

  // Always record in waitlist for tracking
  await upsertWaitlistWithName(sql, normalizedEmail, name ?? null, 'manual');

  if (tokenOnly) {
    // Direct token flow — for CI/service accounts
    const rawToken = generateToken();
    const hash = hashToken(rawToken);
    const expiresAt = new Date();
    expiresAt.setDate(expiresAt.getDate() + days);

    // Neon batch transaction — statements are interdependent and must be atomic
    const txResult = await sql.transaction([
      sql`INSERT INTO beta_users (email, name, notes, status)
          VALUES (${normalizedEmail}, ${name ?? null}, ${notes ?? null}, ${'active'})
          ON CONFLICT (email) DO UPDATE SET
            name = COALESCE(${name ?? null}, beta_users.name),
            notes = COALESCE(${notes ?? null}, beta_users.notes),
            status = ${'active'}
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
  }

  // Default flow — approve via device code + invite email
  const ACTIVATE_BASE = process.env.ACTIVATE_URL ?? 'https://eddacraft.ai/auth/activate';

  const pollToken = randomBytes(32).toString('hex');
  const pollTokenHash = hashToken(pollToken);
  const deviceExpiry = new Date(Date.now() + 48 * 60 * 60 * 1000);

  // Retry on user_code collision (23505). Entire transaction is re-run with
  // a fresh code because the INSERT is part of an atomic batch.
  const { userCode, txResult } = await withUserCodeRetry(async (code) => {
    const result = await sql.transaction([
      sql`INSERT INTO beta_users (email, name, notes, status)
          VALUES (${normalizedEmail}, ${name ?? null}, ${notes ?? null}, ${'active'})
          ON CONFLICT (email) DO UPDATE SET
            name = COALESCE(${name ?? null}, beta_users.name),
            notes = COALESCE(${notes ?? null}, beta_users.notes),
            status = ${'active'}
          RETURNING *`,
      sql`INSERT INTO device_codes (user_id, user_code, poll_token, expires_at)
          VALUES (
            (SELECT id FROM beta_users WHERE email = ${normalizedEmail}),
            ${code}, ${pollTokenHash}, ${deviceExpiry.toISOString()}
          )
          RETURNING *`,
      sql`INSERT INTO audit_log (action, actor, metadata)
          VALUES (${'user.invited'}, ${actor}, ${JSON.stringify({ email: normalizedEmail, scopes, days })})
          RETURNING *`,
    ]);
    return { userCode: code, txResult: result };
  });

  const user = (txResult as unknown[][])[0]?.[0] as { email: string; id: string };

  moveToApprovedAudience(normalizedEmail).catch((err) => {
    console.error('Failed to move audience (non-fatal):', err);
  });

  const activateUrl = `${ACTIVATE_BASE}?code=${userCode}`;
  await sendBetaInvite(normalizedEmail, userCode, activateUrl).catch((err) => {
    console.error('Failed to send invite email (non-fatal):', err);
  });

  return c.json(
    {
      user: { email: user.email, id: user.id },
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
    // Neon batch transaction — statements are interdependent and must be atomic
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
    // Neon batch transaction — statements are interdependent and must be atomic
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
    const waitlistEntry = await findWaitlistEntryByEmail(sql, normalizedEmail);
    if (!waitlistEntry) {
      throw new Error(`not_found:${normalizedEmail}`);
    }

    // Generate access token (90-day expiry)
    const rawToken = generateToken();
    const hash = hashToken(rawToken);
    const tokenExpiry = new Date(Date.now() + 90 * 24 * 60 * 60 * 1000);

    const pollToken = randomBytes(32).toString('hex');
    const pollTokenHash = hashToken(pollToken);
    const deviceExpiry = new Date(Date.now() + 48 * 60 * 60 * 1000);

    // Retry on user_code collision (23505). Entire transaction is re-run
    // with a fresh code because the INSERT is part of an atomic batch.
    const { userCode } = await withUserCodeRetry(async (code) => {
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
              ${code}, ${pollTokenHash}, ${deviceExpiry.toISOString()}
            )
            RETURNING *`,
        sql`INSERT INTO audit_log (action, actor, metadata)
            VALUES (${'user.approved'}, ${actor}, ${JSON.stringify({ email: normalizedEmail })})
            RETURNING *`,
      ]);
      return { userCode: code };
    });

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

  type SkipReason = 'not_found' | 'collision' | 'error';

  async function recordCollision(email: string): Promise<void> {
    await insertAuditLog(sql, 'user.approve.collision', actor, { email }).catch((err) => {
      console.error('Failed to record collision audit (non-fatal):', err);
    });
  }

  function classifySkip(err: unknown): SkipReason {
    if (err instanceof Error && err.message.startsWith('not_found:')) return 'not_found';
    if (isUserCodeCollision(err)) return 'collision';
    return 'error';
  }

  if ('email' in body) {
    try {
      const result = await approveOne(body.email);
      return c.json({ approved: [result] }, 200);
    } catch (err) {
      const reason = classifySkip(err);
      if (reason === 'not_found') {
        return c.json({ error: 'Email not found on waitlist' }, 404);
      }
      if (reason === 'collision') {
        await recordCollision(body.email.toLowerCase().trim());
        return c.json({ error: 'user_code collision after retries, try again' }, 503);
      }
      throw err;
    }
  }

  // Batch mode: oldest N unapproved waitlist entries
  const unapproved = await findUnapprovedWaitlistEntries(sql, body.batch);

  const approved: { email: string; expiresAt: string }[] = [];
  const skipped: { email: string; reason: SkipReason; message?: string }[] = [];

  for (const row of unapproved) {
    try {
      const result = await approveOne(row.email);
      approved.push(result);
    } catch (err) {
      const reason = classifySkip(err);
      const message = err instanceof Error ? err.message : String(err);
      debug('Batch approve skip', { email: row.email, reason, error: message });
      if (reason === 'collision') {
        await recordCollision(row.email.toLowerCase().trim());
      }
      skipped.push({ email: row.email, reason, message });
    }
  }

  return c.json({ approved, skipped }, 200);
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

/**
 * POST /admin/send-migration
 *
 * Send migration email to imported waitlist users.
 * Optional `source` filter (default: 'import').
 * Optional `dryRun` to preview without sending.
 */
const migrationSchema = z.object({
  source: z.enum(['import', 'website', 'manual']).default('import'),
  dryRun: z.boolean().default(false),
  limit: z.number().int().min(1).max(100).default(20),
});

admin.post('/send-migration', zValidator('json', migrationSchema), async (c) => {
  const sql = getClient();
  const actor = resolveAdminActor(c);

  const { source, dryRun, limit } = c.req.valid('json');

  const waitlistRows = await findWaitlistBySource(sql, source, limit);

  if (dryRun) {
    return c.json({
      dryRun: true,
      source,
      count: waitlistRows.length,
      recipients: waitlistRows.map((r) => ({ email: r.email, name: r.name })),
    });
  }

  const results: { email: string; sent: boolean; error?: string }[] = [];

  for (const row of waitlistRows) {
    const delivery = await sendWaitlistMigration(row.email, row.name ?? undefined);
    results.push({
      email: row.email,
      sent: delivery.sent,
      error: delivery.sent ? undefined : delivery.message,
    });
  }

  const sent = results.filter((r) => r.sent).length;
  const failed = results.filter((r) => !r.sent).length;

  await insertAuditLog(sql, 'migration.email.sent', actor, { source, sent, failed });

  return c.json({ source, total: waitlistRows.length, sent, failed, results });
});

export { admin };
