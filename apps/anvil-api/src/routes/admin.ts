import { randomBytes } from 'node:crypto';
import { Hono } from 'hono';
import { z } from 'zod';
import { zValidator } from '@hono/zod-validator';
import { adminAuth } from '../middleware/admin-auth.js';
import { adminRateLimit } from '../middleware/admin-rate-limit.js';
import { getClient } from '../db/client.js';
import {
  findUserByEmail,
  findUserWithTokens,
  insertAuditLog,
  upsertWaitlistWithName,
  findWaitlistEntryByEmail,
  findUnapprovedWaitlistEntries,
  findWaitlistBySource,
  findWaitlistPaginated,
  findAuditEntries,
  findRecentAuditForEmail,
  insertBroadcastSnapshot,
  consumeBroadcastSnapshot,
  findBroadcastSnapshot,
  findActiveScopesForUser,
  type AuthMethod,
  type SnapshotRecipient,
} from '../db/queries.js';
import { generateToken, hashToken } from '../lib/token.js';
import { sendBetaInvite, sendWaitlistMigration } from '../lib/email.js';
import { createDebugger } from '../lib/debug.js';
import { moveToApprovedAudience, removeFromBetaAudience } from '../lib/audience.js';
import { DEFAULT_APPROVAL_SCOPES, resolveApiScope } from '../lib/feature-flags.js';
import {
  inviteSchema,
  approveSchema,
  revokeSchema,
  migrationSchema,
  userEmailUpdateSchema,
  waitlistListQuerySchema,
  auditListQuerySchema,
} from './admin-schemas.js';
import { isUniqueViolation, isUserCodeCollision, withUserCodeRetry } from '../lib/device-code.js';

const debug = createDebugger('api');

const admin = new Hono();

// All admin routes require admin auth
admin.use('*', adminAuth);

// Per-actor rate limit on the whole admin surface. Caps a compromised
// per-operator key (or the shared-key bucket) to 60 requests/min before
// audit-log review can be expected to catch up.
admin.use('*', adminRateLimit({ windowMs: 60_000, max: 60, scope: 'all' }));

// Tighter dedicated cap on `/send-migration`: the operation triggers
// outbound email to waitlist segments, so we want a much smaller
// burst budget per actor even when the coarse cap allows traffic.
admin.use(
  '/send-migration',
  adminRateLimit({ windowMs: 60 * 60 * 1000, max: 5, scope: 'send-migration' })
);

/**
 * Resolve the admin actor identity for audit logging.
 *
 * Per ADMINCLIH-002, the identity is set by the admin-auth middleware —
 * either from the `admin_keys.actor_email` row (per-operator path) or the
 * sentinel `shared-key@anvil` (shared-key path). `X-Admin-Actor` is no
 * longer trusted on either path, which closes the attribution-forgery
 * vector.
 */
function resolveAdminActor(c: { get: (key: 'adminActor') => string | undefined }): string {
  const actor = c.get('adminActor');
  if (!actor) {
    // Middleware always sets this on successful auth, so an undefined here
    // is a programming error (e.g. route mounted without adminAuth). Fail
    // loud rather than silently fabricate an actor.
    throw new Error('resolveAdminActor: adminActor missing from context');
  }
  return actor;
}

/**
 * Resolve the auth method (`shared` | `per_operator`) for the current
 * request, for stamping into audit rows.
 */
function resolveAuthMethod(c: {
  get: (key: 'adminAuthMethod') => AuthMethod | undefined;
}): AuthMethod {
  const method = c.get('adminAuthMethod');
  if (!method) {
    throw new Error('resolveAuthMethod: adminAuthMethod missing from context');
  }
  return method;
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
  const { email, name, notes, days, scopes, tokenOnly, edict } = c.req.valid('json');
  debug('POST /admin/invite', { email, scopes, days, tokenOnly, edict });
  const sql = getClient();
  const actor = resolveAdminActor(c);
  const authMethod = resolveAuthMethod(c);

  // Flag gate: Zod already restricts scope values to the manifest names, but
  // the api.scope.* entitlement flags let operators disable a scope without
  // a redeploy. Reject here so a flipped flag is honoured on the hot path.
  for (const scope of scopes) {
    const resolution = resolveApiScope(scope);
    if (!resolution || !resolution.allowed) {
      return c.json(
        {
          error: 'scope_not_allowed',
          message: `Scope '${scope}' is currently disabled by feature flag`,
          scope,
          reason: resolution?.details.reason ?? 'unknown_scope',
        },
        403
      );
    }
  }

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
      sql`INSERT INTO access_tokens (user_id, token_hash, scopes, is_edict, expires_at)
          VALUES (
            (SELECT id FROM beta_users WHERE email = ${normalizedEmail}),
            ${hash}, ${scopes}, ${edict}, ${expiresAt.toISOString()}
          )
          RETURNING *`,
      sql`INSERT INTO audit_log (action, actor, metadata, auth_method)
          VALUES (${'token.created'}, ${actor}, ${JSON.stringify({ email: normalizedEmail, scopes, days, edict })}, ${authMethod})
          RETURNING *`,
    ]);

    const user = (txResult as unknown[][])[0]?.[0] as { email: string; id: string };

    return c.json(
      {
        token: rawToken,
        user: { email: user.email, id: user.id },
        expiresAt: expiresAt.toISOString(),
        scopes,
        edict,
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
      sql`INSERT INTO audit_log (action, actor, metadata, auth_method)
          VALUES (${'user.invited'}, ${actor}, ${JSON.stringify({ email: normalizedEmail, scopes, days })}, ${authMethod})
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
 * Revoke tokens by email (account-level) or by specific token (grant-level).
 *
 * SEC-007 / GH #1672: revocation must close every credential surface that
 * could be used to mint or refresh a licence, in a single Postgres
 * transaction.
 *
 * - Account-level (email): atomically revoke all `access_tokens` for the
 *   user, revoke all `refresh_tokens` for the user, and transition
 *   `beta_users.status` from `active` to `suspended`. The existing
 *   `user.status === 'active'` gates in the OAuth, OTP, device, and session
 *   refresh routes then block re-mint via every login path until an admin
 *   reapproves the account (`POST /admin/approve` flips status back to
 *   `active`).
 * - Grant-level (token): revoke the specific `access_tokens` row by hash
 *   and revoke all `refresh_tokens` for the owning user so the user cannot
 *   pivot through `/session/refresh` to mint a new access token. The user
 *   stays `active` and may re-authenticate to obtain a fresh grant.
 */
admin.post('/revoke', zValidator('json', revokeSchema), async (c) => {
  const { email, token } = c.req.valid('json');
  debug('POST /admin/revoke', { hasEmail: !!email, hasToken: !!token });
  const sql = getClient();
  const actor = resolveAdminActor(c);
  const authMethod = resolveAuthMethod(c);

  if (email) {
    const normalizedEmail = email.toLowerCase().trim();
    // Neon batch transaction — statements are interdependent and must be atomic.
    // Order matches the response unpacking below: [access, refresh, suspend, audit].
    const txResult = await sql.transaction([
      sql`UPDATE access_tokens SET revoked_at = now()
          WHERE user_id = (SELECT id FROM beta_users WHERE email = ${normalizedEmail})
            AND revoked_at IS NULL
          RETURNING id`,
      sql`UPDATE refresh_tokens SET revoked_at = now()
          WHERE user_id = (SELECT id FROM beta_users WHERE email = ${normalizedEmail})
            AND revoked_at IS NULL
          RETURNING id`,
      sql`UPDATE beta_users SET status = ${'suspended'}
          WHERE email = ${normalizedEmail}
            AND status = ${'active'}
          RETURNING id`,
      sql`INSERT INTO audit_log (action, actor, metadata, auth_method)
          VALUES (${'tokens.revoked'}, ${actor}, ${JSON.stringify({ email: normalizedEmail, accountLevel: true })}, ${authMethod})
          RETURNING *`,
    ]);
    const accessRows = (txResult as unknown[][])[0] ?? [];
    const refreshRows = (txResult as unknown[][])[1] ?? [];
    const suspendRows = (txResult as unknown[][])[2] ?? [];
    removeFromBetaAudience(normalizedEmail).catch((err) => {
      console.error('Failed to remove from audience (non-fatal):', err);
    });
    return c.json({
      revoked: accessRows.length,
      refreshSessionsRevoked: refreshRows.length,
      accountSuspended: suspendRows.length > 0,
    });
  }

  if (token) {
    const hash = hashToken(token);
    // Neon batch transaction — statements are interdependent and must be atomic.
    // The refresh-token UPDATE re-derives the owning user_id from the token
    // hash row; Neon batch statements cannot share a CTE so the subquery
    // repeats the hash. Both UPDATEs run in the same Postgres transaction.
    // The refresh sweep deliberately does NOT filter on access_tokens'
    // `revoked_at IS NULL` so an idempotent double-revoke (admin runs the
    // command twice) still scrubs any refresh sessions that were left over,
    // which matters when this PR rolls out against accounts that were
    // already revoked under the pre-fix code.
    const txResult = await sql.transaction([
      sql`UPDATE access_tokens SET revoked_at = now()
          WHERE token_hash = ${hash}
            AND revoked_at IS NULL
          RETURNING id`,
      sql`UPDATE refresh_tokens SET revoked_at = now()
          WHERE user_id = (SELECT user_id FROM access_tokens WHERE token_hash = ${hash})
            AND revoked_at IS NULL
          RETURNING id`,
      sql`INSERT INTO audit_log (action, actor, metadata, auth_method)
          VALUES (${'token.revoked'}, ${actor}, ${JSON.stringify({ revoked: true, tokenHash: hash })}, ${authMethod})
          RETURNING *`,
    ]);
    const accessRows = (txResult as unknown[][])[0] ?? [];
    const refreshRows = (txResult as unknown[][])[1] ?? [];
    return c.json({
      revoked: accessRows.length > 0 ? 1 : 0,
      refreshSessionsRevoked: refreshRows.length,
    });
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
  const authMethod = resolveAuthMethod(c);

  const ACTIVATE_BASE = process.env.ACTIVATE_URL ?? 'https://eddacraft.ai/auth/activate';

  async function approveOne(email: string): Promise<{ email: string; expiresAt: string }> {
    const normalizedEmail = email.toLowerCase().trim();

    // Verify waitlisted
    const waitlistEntry = await findWaitlistEntryByEmail(sql, normalizedEmail);
    if (!waitlistEntry) {
      throw new Error(`not_found:${normalizedEmail}`);
    }

    // Preserve any graded scopes the user already has from a prior
    // /admin/invite. Without this, approve always wrote the
    // DEFAULT_APPROVAL_SCOPES (`['beta']`) row and — when scope reads were
    // "most-recent-row" semantics — that newer row would silently shadow a
    // preview/internal grant. The /session/refresh fix in eae47b3d closed
    // the read side; this closes the write side. The query is best-effort:
    // if the user has no existing user row yet, we use the default scopes.
    const existingUser = await findUserByEmail(sql, normalizedEmail);
    const requestedScopes: string[] = existingUser
      ? Array.from(
          new Set([
            ...(await findActiveScopesForUser(sql, existingUser.id)),
            ...DEFAULT_APPROVAL_SCOPES,
          ])
        )
      : [...DEFAULT_APPROVAL_SCOPES];

    const grantedScopes: string[] = [];
    const droppedScopes: string[] = [];
    for (const scope of requestedScopes) {
      const resolution = resolveApiScope(scope);
      if (resolution?.allowed) {
        grantedScopes.push(scope);
      } else {
        droppedScopes.push(scope);
      }
    }

    if (grantedScopes.length === 0) {
      await insertAuditLog(
        sql,
        'user.approve.scopes_dropped',
        actor,
        { email: normalizedEmail, droppedScopes, grantedScopes: [] },
        authMethod
      ).catch((err) => {
        console.error('Failed to record dropped scopes audit (non-fatal):', err);
      });
      throw new Error(`no_scopes:${normalizedEmail}`);
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
      const statements = [
        sql`INSERT INTO beta_users (email, status)
            VALUES (${normalizedEmail}, ${'active'})
            ON CONFLICT (email) DO UPDATE SET status = ${'active'}
            RETURNING *`,
        sql`INSERT INTO access_tokens (user_id, token_hash, scopes, expires_at)
            VALUES (
              (SELECT id FROM beta_users WHERE email = ${normalizedEmail}),
              ${hash}, ${grantedScopes}, ${tokenExpiry.toISOString()}
            )
            RETURNING *`,
        sql`INSERT INTO device_codes (user_id, user_code, poll_token, expires_at)
            VALUES (
              (SELECT id FROM beta_users WHERE email = ${normalizedEmail}),
              ${code}, ${pollTokenHash}, ${deviceExpiry.toISOString()}
            )
            RETURNING *`,
        sql`INSERT INTO audit_log (action, actor, metadata, auth_method)
            VALUES (${'user.approved'}, ${actor}, ${JSON.stringify({ email: normalizedEmail })}, ${authMethod})
            RETURNING *`,
      ];
      if (droppedScopes.length > 0) {
        statements.push(sql`INSERT INTO audit_log (action, actor, metadata, auth_method)
            VALUES (
              ${'user.approve.scopes_dropped'}, ${actor},
              ${JSON.stringify({ email: normalizedEmail, droppedScopes, grantedScopes })},
              ${authMethod}
            )
            RETURNING *`);
      }
      await sql.transaction(statements);
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

  type SkipReason = 'not_found' | 'collision' | 'no_scopes' | 'error';

  async function recordCollision(email: string): Promise<void> {
    await insertAuditLog(sql, 'user.approve.collision', actor, { email }, authMethod).catch(
      (err) => {
        console.error('Failed to record collision audit (non-fatal):', err);
      }
    );
  }

  function classifySkip(err: unknown): SkipReason {
    if (err instanceof Error && err.message.startsWith('not_found:')) return 'not_found';
    if (err instanceof Error && err.message.startsWith('no_scopes:')) return 'no_scopes';
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
      if (reason === 'no_scopes') {
        return c.json({ error: 'No enabled API scopes available for approval' }, 409);
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
 * GET /admin/waitlist
 *
 * Paginated waitlist listing. Filter by approval status (pending /
 * approved / all), signup source (manual / website / import / all),
 * with limit and offset for pagination.
 *
 * Approval is derived by join against beta_users — an entry is
 * "approved" when a matching beta_users row exists.
 */
admin.get('/waitlist', zValidator('query', waitlistListQuerySchema), async (c) => {
  const { status, source, limit, offset } = c.req.valid('query');
  debug('GET /admin/waitlist', { status, source, limit, offset });
  const sql = getClient();

  const result = await findWaitlistPaginated(sql, { status, source, limit, offset });

  return c.json(result);
});

/**
 * GET /admin/audit
 *
 * Paginated audit log listing, most recent first. Optional action and
 * actor filters apply exact-match equality. Bounded pagination
 * (limit 1-200, default 50).
 */
admin.get('/audit', zValidator('query', auditListQuerySchema), async (c) => {
  const { action, actor, limit, offset } = c.req.valid('query');
  debug('GET /admin/audit', { action, actor, limit, offset });
  const sql = getClient();

  const result = await findAuditEntries(sql, { action, actor, limit, offset });

  return c.json(result);
});

/**
 * GET /admin/user/:email
 *
 * Lookup a user and their token info, plus up to 10 most-recent
 * audit entries for that email. Audit entries are matched via
 * metadata->>'email' OR actor = email (e.g. GitHub OAuth writes where
 * user.email is the actor), so events logged under either shape are
 * surfaced.
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

  // recentAudit is an enrichment — degrade gracefully if the audit
  // lookup fails so the primary user + tokens response still lands.
  let recentAudit: Awaited<ReturnType<typeof findRecentAuditForEmail>> = [];
  let auditError = false;
  try {
    recentAudit = await findRecentAuditForEmail(sql, email);
  } catch (err) {
    auditError = true;
    console.error('findRecentAuditForEmail failed (non-fatal):', err);
  }

  return c.json({
    user: result.user,
    tokens: result.tokens.map((t) => ({
      id: t.id,
      scopes: t.scopes,
      is_edict: t.is_edict,
      expires_at: t.expires_at,
      revoked_at: t.revoked_at,
      created_at: t.created_at,
    })),
    recentAudit,
    ...(auditError ? { auditError: true } : {}),
  });
});

// Snapshot token TTL. Short enough to bound stale-preview risk, long
// enough to accommodate a considered operator confirmation.
const SEND_MIGRATION_SNAPSHOT_TTL_SECONDS = 10 * 60;

function computeCohortDrift(
  snapshot: SnapshotRecipient[],
  current: SnapshotRecipient[]
): { added: string[]; removed: string[] } {
  const snapshotEmails = new Set(snapshot.map((r) => r.email));
  const currentEmails = new Set(current.map((r) => r.email));
  const added = [...currentEmails].filter((e) => !snapshotEmails.has(e));
  const removed = [...snapshotEmails].filter((e) => !currentEmails.has(e));
  return { added: added.sort(), removed: removed.sort() };
}

/**
 * POST /admin/send-migration
 *
 * Send migration email to waitlist users filtered by source
 * (default: 'import').
 *
 * Flow:
 *   1. Dry-run (`dryRun: true`) records a snapshot of the recipient set
 *      and returns it along with an opaque single-use `previewToken`
 *      (TTL 10 min, bound to the admin actor).
 *   2. Real-send (`dryRun: false`) requires that `previewToken`.
 *      The handler atomically consumes it, refetches the current cohort,
 *      and either sends to the snapshotted set or rejects with a
 *      cohort_drift error describing the diff.
 *
 * Error codes (JSON `code` field, tested in admin.test.ts):
 *   400 preview_token_required          — real-send with no token
 *   410 preview_token_missing           — token not found, or caller is
 *                                         not the creator (merged to avoid
 *                                         confirming existence to non-owners)
 *   410 preview_token_expired           — TTL passed
 *   410 preview_token_consumed          — token already used
 *   409 cohort_drift                    — body: DriftDiffResponse
 */
admin.post('/send-migration', zValidator('json', migrationSchema), async (c) => {
  const sql = getClient();
  const actor = resolveAdminActor(c);
  const authMethod = resolveAuthMethod(c);
  const { source, dryRun, limit, previewToken } = c.req.valid('json');

  if (dryRun) {
    const waitlistRows = await findWaitlistBySource(sql, source, limit);
    const recipients: SnapshotRecipient[] = waitlistRows.map((r) => ({
      email: r.email,
      name: r.name,
    }));

    const token = randomBytes(16).toString('hex');
    // /admin/send-migration is now a back-compat surface over the
    // generalised broadcast snapshot table. Until the EMAIL-006 shim
    // lands, supply the waitlist-migration template + waitlist:source
    // audience values inline so the row carries the same semantics as
    // a future /admin/broadcast call.
    const snapshot = await insertBroadcastSnapshot(sql, {
      token,
      template: 'waitlist-migration',
      templateProps: {},
      audienceKey: 'waitlist:source',
      audienceParams: { source },
      recipients,
      createdByActor: actor,
      ttlSeconds: SEND_MIGRATION_SNAPSHOT_TTL_SECONDS,
    });

    return c.json({
      dryRun: true,
      source,
      count: recipients.length,
      recipients,
      previewToken: snapshot.token,
      expiresAt: snapshot.expires_at,
    });
  }

  if (!previewToken) {
    return c.json(
      {
        code: 'preview_token_required',
        error: 'previewToken is required for real-sends; run with --dry-run first',
      },
      400
    );
  }

  const consumed = await consumeBroadcastSnapshot(sql, { token: previewToken, actor });

  if (!consumed) {
    // The atomic consume failed; figure out which distinct reason so the
    // CLI can surface a tailored recovery message. The find is scoped
    // to (token, actor) so a non-owner caller falls into the `missing`
    // branch and never learns that the token exists.
    const existing = await findBroadcastSnapshot(sql, { token: previewToken, actor });
    if (!existing) {
      return c.json(
        {
          code: 'preview_token_missing',
          error: 'preview token is unknown; re-run with --dry-run for a fresh preview',
        },
        410
      );
    }
    if (existing.consumed_at !== null) {
      return c.json(
        {
          code: 'preview_token_consumed',
          error: 'preview token has already been used; re-run with --dry-run',
        },
        410
      );
    }
    // Only expiry remains.
    return c.json(
      {
        code: 'preview_token_expired',
        error: 'preview token has expired; re-run with --dry-run for a fresh preview',
      },
      410
    );
  }

  // The snapshot row is the source of truth for both `source` and the
  // recipient set — the request's `source` field is redundant on the
  // real-send path and a mismatch would otherwise produce a
  // false-positive drift check against the wrong cohort. Under the
  // generalised snapshot schema, `source` lives inside audience_params.
  const snapshotSource = String(consumed.audience_params['source']);

  // Refetch the current cohort and compare to the snapshot. Use the
  // snapshot size (not the request limit) so the fresh query returns
  // an apples-to-apples slice — a caller-supplied limit smaller or
  // larger than what the snapshot captured would otherwise produce a
  // false-positive drift rejection.
  const freshLimit = Math.max(consumed.recipients.length, 1);
  const currentRows = await findWaitlistBySource(sql, snapshotSource, freshLimit);
  const currentRecipients: SnapshotRecipient[] = currentRows.map((r) => ({
    email: r.email,
    name: r.name,
  }));
  const drift = computeCohortDrift(consumed.recipients, currentRecipients);

  if (drift.added.length > 0 || drift.removed.length > 0) {
    return c.json(
      {
        code: 'cohort_drift',
        error: 'recipient set changed since preview; re-run with --dry-run',
        added: drift.added,
        removed: drift.removed,
      },
      409
    );
  }

  // Use the snapshot as the source of truth for who to email — not a
  // fresh query — so the operator's confirmation anchors the send.
  const results: { email: string; sent: boolean; error?: string }[] = [];
  for (const row of consumed.recipients) {
    const delivery = await sendWaitlistMigration(row.email, row.name ?? undefined);
    results.push({
      email: row.email,
      sent: delivery.sent,
      error: delivery.sent ? undefined : delivery.message,
    });
  }

  const sent = results.filter((r) => r.sent).length;
  const failed = results.filter((r) => !r.sent).length;

  await insertAuditLog(
    sql,
    'migration.email.sent',
    actor,
    {
      source: snapshotSource,
      sent,
      failed,
      previewToken,
    },
    authMethod
  );

  return c.json({
    source: snapshotSource,
    total: consumed.recipients.length,
    sent,
    failed,
    results,
  });
});

/**
 * POST /admin/user/email-update
 *
 * Update a beta user's email. Used when the email a user registered with
 * doesn't match any address verified on their GitHub account, and
 * self-service (adding the beta email to github.com/settings/emails) isn't
 * viable — typically because they've lost access to the original inbox.
 *
 * Scope: updates beta_users.email only. The waitlist row is left at the
 * original address as the historical signup record. Existing licence JWTs
 * continue to work (they authenticate on user.id, not email); new sign-ins
 * will carry the updated email in claims.
 */
admin.post('/user/email-update', zValidator('json', userEmailUpdateSchema), async (c) => {
  const { currentEmail, newEmail } = c.req.valid('json');
  debug('POST /admin/user/email-update');
  const sql = getClient();
  const actor = resolveAdminActor(c);
  const authMethod = resolveAuthMethod(c);

  const normalizedCurrent = currentEmail.toLowerCase().trim();
  const normalizedNew = newEmail.toLowerCase().trim();

  if (normalizedCurrent === normalizedNew) {
    return c.json({ error: 'New email matches current email' }, 400);
  }

  const existing = await findUserByEmail(sql, normalizedCurrent);
  if (!existing) {
    return c.json({ error: 'User not found' }, 404);
  }

  const collision = await findUserByEmail(sql, normalizedNew);
  if (collision) {
    return c.json({ error: 'New email already in use' }, 409);
  }

  let txResult: unknown[][];
  try {
    txResult = (await sql.transaction([
      sql`UPDATE beta_users SET email = ${normalizedNew}
          WHERE id = ${existing.id}
          RETURNING id, email, status`,
      sql`INSERT INTO audit_log (action, actor, metadata, auth_method)
          VALUES (
            ${'user.email.updated'},
            ${actor},
            ${JSON.stringify({
              userId: existing.id,
              email: normalizedNew,
              from: normalizedCurrent,
              to: normalizedNew,
            })},
            ${authMethod}
          )
          RETURNING *`,
    ])) as unknown[][];
  } catch (err) {
    if (isUniqueViolation(err)) {
      return c.json({ error: 'New email already in use' }, 409);
    }
    throw err;
  }

  const updated = txResult[0]?.[0] as { id: string; email: string; status: string } | undefined;

  if (!updated) {
    return c.json({ error: 'User was deleted during update' }, 404);
  }

  return c.json({
    user: { id: updated.id, email: updated.email, status: updated.status },
    previousEmail: normalizedCurrent,
  });
});

export { admin };
