import { randomBytes } from 'node:crypto';
import { Hono } from 'hono';
import { z } from 'zod';
import { zValidator } from '@hono/zod-validator';
import { adminAuth } from '../middleware/admin-auth.js';
import { adminRateLimit } from '../middleware/admin-rate-limit.js';
import { getClient, type NeonClient } from '../db/client.js';
import {
  findUserByEmail,
  findUserWithTokens,
  insertAuditLog,
  upsertWaitlistWithName,
  findWaitlistEntryByEmail,
  findUnapprovedWaitlistEntries,
  findWaitlistPaginated,
  findAuditEntries,
  findRecentAuditForEmail,
  insertBroadcastSnapshot,
  consumeBroadcastSnapshot,
  findBroadcastSnapshot,
  findActiveScopesForUser,
  type AuthMethod,
  type BroadcastSnapshot,
  type SnapshotRecipient,
} from '../db/queries.js';
import { generateToken, hashToken } from '../lib/token.js';
import { sendBetaInvite } from '../lib/email.js';
import { createDebugger } from '../lib/debug.js';
import { moveToApprovedAudience, removeFromBetaAudience } from '../lib/audience.js';
import { DEFAULT_APPROVAL_SCOPES, resolveApiScope } from '../lib/feature-flags.js';
import {
  inviteSchema,
  approveSchema,
  revokeSchema,
  migrationSchema,
  broadcastSchema,
  userEmailUpdateSchema,
  userNameUpdateSchema,
  waitlistListQuerySchema,
  auditListQuerySchema,
} from './admin-schemas.js';
import {
  AUDIENCE_KEYS,
  type AudienceKey,
  type AudienceRow,
  resolveAudience,
} from '../lib/broadcast-audiences.js';
import { EMAIL_REGISTRY, type TemplateKey } from '../lib/email-registry.js';
import { isUniqueViolation } from '../lib/device-code.js';
import { findFleetOverview } from '../lib/fleet-overview.js';

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

// Same envelope for `/broadcast` — operator-initiated bulk email needs a
// per-actor cap independent of the coarse one. Scope is on the endpoint
// (not per template) so alternating templates can't dodge the budget.
admin.use('/broadcast', adminRateLimit({ windowMs: 60 * 60 * 1000, max: 5, scope: 'broadcast' }));

/**
 * GET /admin/fleet
 *
 * Current operator snapshot over retained identity-bearing beacon rows. This
 * deliberately lives on the authenticated, rate-limited admin router rather
 * than the public telemetry ingest router, and never returns install IDs.
 */
admin.get('/fleet', async (c) => {
  debug('GET /admin/fleet');
  const result = await findFleetOverview(getClient());
  return c.json(result);
});

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
 * then run the full approve flow (upsert user, invite email). Activation
 * happens on the user's first `anvil auth login` (GitHub device flow or
 * --otp) — no per-invite device code is generated (GHCLIAUTH-007).
 *
 * tokenOnly=true: upsert user + waitlist entry, generate a raw access
 * token returned exactly once. For CI/service accounts.
 */
admin.post('/invite', zValidator('json', inviteSchema), async (c) => {
  const { email, name, notes, days, scopes, tokenOnly, edict } = c.req.valid('json');
  debug('POST /admin/invite', { hasEmail: Boolean(email), scopes, days, tokenOnly, edict });
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
      sql`UPDATE waitlist
          SET approved_at = COALESCE(approved_at, NOW()), updated_at = NOW()
          WHERE email = ${normalizedEmail}
          RETURNING email`,
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

    // Index 0 is the waitlist stamp; beta_users INSERT is index 1.
    const user = (txResult as unknown[][])[1]?.[0] as { email: string; id: string };

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

  // Default flow — mark the user active and send the invite email. The user
  // activates on first `anvil auth login` (GitHub device flow links
  // github_id, or --otp proves the invited email) — ADR-066 decision 7.
  // Stamp waitlist.approved_at (first grant wins) so admin list/Neon filters
  // show operator admission, not merely beta_users existence.
  const txResult = await sql.transaction([
    sql`UPDATE waitlist
        SET approved_at = COALESCE(approved_at, NOW()), updated_at = NOW()
        WHERE email = ${normalizedEmail}
        RETURNING email`,
    sql`INSERT INTO beta_users (email, name, notes, status)
        VALUES (${normalizedEmail}, ${name ?? null}, ${notes ?? null}, ${'active'})
        ON CONFLICT (email) DO UPDATE SET
          name = COALESCE(${name ?? null}, beta_users.name),
          notes = COALESCE(${notes ?? null}, beta_users.notes),
          status = ${'active'}
        RETURNING *`,
    sql`INSERT INTO audit_log (action, actor, metadata, auth_method)
        VALUES (${'user.invited'}, ${actor}, ${JSON.stringify({ email: normalizedEmail, scopes, days })}, ${authMethod})
        RETURNING *`,
  ]);

  const user = (txResult as unknown[][])[1]?.[0] as { email: string; id: string };

  moveToApprovedAudience(normalizedEmail).catch((err) => {
    console.error('Failed to move audience (non-fatal):', err);
  });

  await sendBetaInvite(normalizedEmail).catch((err) => {
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
 * Records the scope grant and sends the invite email; activation happens
 * on the user's first `anvil auth login` (GHCLIAUTH-007 — no per-invite
 * device code).
 */
admin.post('/approve', zValidator('json', approveSchema), async (c) => {
  const body = c.req.valid('json');
  debug('POST /admin/approve', { hasEmail: 'email' in body, hasBatch: 'batch' in body });
  const sql = getClient();
  const actor = resolveAdminActor(c);
  const authMethod = resolveAuthMethod(c);

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

    // Record the scope grant (90-day expiry). The raw token is discarded —
    // this access_tokens row is the scope record findActiveScopesForUser
    // reads when the licence is minted at first login, not a usable bearer
    // token.
    const rawToken = generateToken();
    const hash = hashToken(rawToken);
    const tokenExpiry = new Date(Date.now() + 90 * 24 * 60 * 60 * 1000);

    const statements = [
      sql`UPDATE waitlist
          SET approved_at = COALESCE(approved_at, NOW()), updated_at = NOW()
          WHERE email = ${normalizedEmail}
          RETURNING email`,
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

    // Move from waitlist to beta audience (best-effort)
    moveToApprovedAudience(normalizedEmail).catch((err) => {
      console.error('Failed to move audience (non-fatal):', err);
    });

    // Best-effort email — don't fail the approval if email fails
    await sendBetaInvite(normalizedEmail).catch((err) => {
      console.error('Failed to send invite email (non-fatal):', err);
    });

    return { email: normalizedEmail, expiresAt: tokenExpiry.toISOString() };
  }

  type SkipReason = 'not_found' | 'no_scopes' | 'error';

  function classifySkip(err: unknown): SkipReason {
    if (err instanceof Error && err.message.startsWith('not_found:')) return 'not_found';
    if (err instanceof Error && err.message.startsWith('no_scopes:')) return 'no_scopes';
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
      debug('Batch approve skip', { reason, error: message });
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
 * Approval is waitlist.approved_at (operator grant via approve/invite).
 * pending = NULL; approved = NOT NULL. Independent of beta_users existence.
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
  debug('GET /admin/audit', { action, hasActor: Boolean(actor), limit, offset });
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
const BROADCAST_SNAPSHOT_TTL_SECONDS = 10 * 60;

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

interface BroadcastSendResult {
  email: string;
  sent: boolean;
  error?: string;
}

type BroadcastSendOutcome =
  | { type: 'invalid_template' }
  | { type: 'drift'; added: string[]; removed: string[] }
  | {
      type: 'sent';
      total: number;
      sent: number;
      failed: number;
      results: BroadcastSendResult[];
    };

/**
 * Given a consumed broadcast snapshot, re-resolve the audience against
 * the snapshot's stored key + params, drift-check against the recorded
 * recipient set, and dispatch the template's registered sender per row.
 *
 * Snapshot is the source of truth for who to email and what props to
 * send — never the request body. Per-recipient send failures surface in
 * `results[]` without aborting the batch.
 *
 * `invalid_template` is a defensive branch: snapshots are only created
 * for broadcast templates with audiences in AUDIENCE_KEYS, but the
 * registry / audience set could change between snapshot and consume
 * (template removed mid-flight, or DB written directly). Callers
 * translate it to an HTTP response. The same outcome covers an unknown
 * audience_key so the consume side never throws a TypeError off the end
 * of the resolveAudience switch.
 */
async function executeBroadcastFromSnapshot(
  sql: NeonClient,
  consumed: BroadcastSnapshot
): Promise<BroadcastSendOutcome> {
  const entry = EMAIL_REGISTRY[consumed.template as TemplateKey];
  if (!entry || entry.kind !== 'broadcast') {
    return { type: 'invalid_template' };
  }
  if (!(AUDIENCE_KEYS as readonly string[]).includes(consumed.audience_key)) {
    return { type: 'invalid_template' };
  }

  // freshLimit = snapshot size + 1 so cohort GROWTH (a new active user
  // joined between snapshot and consume) surfaces as a single extra row
  // and trips computeCohortDrift's `added` path. Without the +1, a
  // grown cohort would return exactly snapshot-size rows and look
  // identical. We only need to detect that drift exists — the operator
  // re-previews to see the full diff.
  const freshLimit = Math.max(consumed.recipients.length, 1) + 1;
  const currentRows = await resolveAudience(sql, consumed.audience_key as AudienceKey, {
    limit: freshLimit,
    params: consumed.audience_params as Record<string, string>,
  });
  const currentRecipients: SnapshotRecipient[] = currentRows.map((r) => ({
    email: r.email,
    name: r.name,
  }));
  const drift = computeCohortDrift(consumed.recipients, currentRecipients);
  if (drift.added.length > 0 || drift.removed.length > 0) {
    return { type: 'drift', added: drift.added, removed: drift.removed };
  }

  const results: BroadcastSendResult[] = [];
  for (const row of consumed.recipients) {
    // Per-recipient try/catch so an unhandled SDK throw doesn't abort
    // the batch and strand the remaining recipients with a consumed
    // snapshot and no audit row. The senders already return
    // {sent: false} for caught errors; this is belt-and-braces for
    // future SDK regressions.
    let delivery: { sent: boolean; code?: string; message?: string };
    try {
      delivery = await entry.sender(
        // SnapshotRecipient doesn't store user_id; sender contract takes
        // an AudienceRow. Pass null — current senders only use email + name.
        { email: row.email, name: row.name, user_id: null } satisfies AudienceRow,
        consumed.template_props
      );
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      console.error('Broadcast sender threw — continuing batch:', message);
      delivery = { sent: false, code: 'sender_threw', message };
    }
    results.push({
      email: row.email,
      sent: delivery.sent,
      error: delivery.sent ? undefined : delivery.message,
    });
  }
  const sent = results.filter((r) => r.sent).length;
  const failed = results.filter((r) => !r.sent).length;
  return {
    type: 'sent',
    total: consumed.recipients.length,
    sent,
    failed,
    results,
  };
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

  // /admin/send-migration is a back-compat shim over the generalised
  // /admin/broadcast flow: it maps {source} to the equivalent
  // (template: waitlist-migration, audience: waitlist:source) call,
  // reuses the same audience resolver, registry sender, snapshot
  // queries, and post-consume helper, then translates the result back
  // to the legacy migration response shape and audit-log entry
  // (`migration.email.sent`).
  //
  // Per EMAIL-001 design decision 2, `waitlist:source` excludes
  // addresses already in beta_users, narrowing the cohort vs. the
  // pre-EMAIL-006 behaviour of findWaitlistBySource.

  if (dryRun) {
    const audienceRows = await resolveAudience(sql, 'waitlist:source', {
      limit,
      params: { source },
    });
    const recipients: SnapshotRecipient[] = audienceRows.map((r) => ({
      email: r.email,
      name: r.name,
    }));

    const token = randomBytes(16).toString('hex');
    const snapshot = await insertBroadcastSnapshot(sql, {
      token,
      template: 'waitlist-migration',
      templateProps: {},
      audienceKey: 'waitlist:source',
      audienceParams: { source },
      recipients,
      createdByActor: actor,
      ttlSeconds: BROADCAST_SNAPSHOT_TTL_SECONDS,
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
  // real-send path. Under the generalised schema, source lives inside
  // audience_params. Fall back to '' rather than String(undefined) so
  // a malformed snapshot doesn't leak the literal 'undefined' into the
  // audit log + response.
  const snapshotSourceValue = consumed.audience_params['source'];
  const snapshotSource = typeof snapshotSourceValue === 'string' ? snapshotSourceValue : '';

  // Audit the dispatch BEFORE the send loop runs (see broadcast handler
  // for rationale). Legacy event name kept for the migration surface.
  // Audit metadata records the SHA-256 hash of the preview token (matching
  // the token_hash column in send_broadcast_snapshots) so an investigator
  // can correlate audit rows to snapshot rows without storing the bearer
  // in plain text. Logging the raw token would defeat migration 014's
  // at-rest hashing.
  const previewTokenHash = hashToken(previewToken);

  await insertAuditLog(
    sql,
    'migration.email.dispatch_started',
    actor,
    {
      source: snapshotSource,
      recipientCount: consumed.recipients.length,
      previewTokenHash,
    },
    authMethod
  );

  const outcome = await executeBroadcastFromSnapshot(sql, consumed);
  if (outcome.type === 'invalid_template') {
    // Should be impossible — snapshots are only created above with
    // template='waitlist-migration' + audience='waitlist:source', both
    // registered. If this branch fires, the registry / audience set
    // mutated between snapshot and consume. Match /admin/broadcast's
    // code so a client distinguishes this from cohort drift (which
    // would re-preview-loop on retry).
    await insertAuditLog(
      sql,
      'migration.email.blocked',
      actor,
      { reason: 'invalid_template', source: snapshotSource, previewTokenHash },
      authMethod
    );
    return c.json(
      {
        code: 'template_kind_not_broadcastable',
        error: 'snapshot is no longer broadcastable; registry or audience set changed',
      },
      400
    );
  }
  if (outcome.type === 'drift') {
    await insertAuditLog(
      sql,
      'migration.email.blocked',
      actor,
      {
        reason: 'cohort_drift',
        source: snapshotSource,
        added: outcome.added,
        removed: outcome.removed,
        previewTokenHash,
      },
      authMethod
    );
    return c.json(
      {
        code: 'cohort_drift',
        error: 'recipient set changed since preview; re-run with --dry-run',
        added: outcome.added,
        removed: outcome.removed,
      },
      409
    );
  }

  await insertAuditLog(
    sql,
    'migration.email.sent',
    actor,
    {
      source: snapshotSource,
      sent: outcome.sent,
      failed: outcome.failed,
      previewTokenHash,
      failedRecipients: outcome.results
        .filter((r) => !r.sent)
        .map((r) => ({ email: r.email, error: r.error })),
    },
    authMethod
  );

  return c.json({
    source: snapshotSource,
    total: outcome.total,
    sent: outcome.sent,
    failed: outcome.failed,
    results: outcome.results,
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

/**
 * POST /admin/user/name-update
 *
 * Operator enrichment for display name (and optional beta-user notes) without
 * invite/approve side effects: no token issue, no status change, no email.
 *
 * - Overwrites `waitlist.name` when a waitlist row exists.
 * - Overwrites `beta_users.name` when a beta user exists; optional `notes`
 *   updates only when a beta user exists (400 if notes are sent for a
 *   waitlist-only email).
 * - 404 when neither waitlist nor beta_users has the email.
 */
admin.post('/user/name-update', zValidator('json', userNameUpdateSchema), async (c) => {
  const { email, name, notes } = c.req.valid('json');
  debug('POST /admin/user/name-update');
  const sql = getClient();
  const actor = resolveAdminActor(c);
  const authMethod = resolveAuthMethod(c);

  const normalizedEmail = email.toLowerCase().trim();
  const waitlist = await findWaitlistEntryByEmail(sql, normalizedEmail);
  const existing = await findUserByEmail(sql, normalizedEmail);

  if (!waitlist && !existing) {
    return c.json({ error: 'User not found' }, 404);
  }

  if (notes !== undefined && !existing) {
    return c.json(
      {
        error: 'notes require an existing beta user; waitlist-only entries store name only',
      },
      400
    );
  }

  // Updates first; derive waitlistUpdated/userUpdated from RETURNING so the
  // response and audit match what actually changed (race-safe).
  const statements = [];

  if (waitlist) {
    statements.push(
      sql`UPDATE waitlist
          SET name = ${name}, updated_at = NOW()
          WHERE email = ${normalizedEmail}
          RETURNING email, name`
    );
  }

  if (existing) {
    if (notes !== undefined) {
      statements.push(
        sql`UPDATE beta_users
            SET name = ${name}, notes = ${notes}, updated_at = NOW()
            WHERE id = ${existing.id}
            RETURNING id, email, name, notes, status`
      );
    } else {
      statements.push(
        sql`UPDATE beta_users
            SET name = ${name}, updated_at = NOW()
            WHERE id = ${existing.id}
            RETURNING id, email, name, notes, status`
      );
    }
  }

  const txResult = (await sql.transaction(statements)) as unknown[][];

  let waitlistName: string | null = null;
  let waitlistUpdated = false;
  let userRow: {
    id: string;
    email: string;
    name: string | null;
    notes: string | null;
    status: string;
  } | null = null;

  let idx = 0;
  if (waitlist) {
    const row = txResult[idx]?.[0] as { email: string; name: string | null } | undefined;
    waitlistUpdated = Boolean(row);
    waitlistName = row?.name ?? null;
    idx += 1;
  }
  if (existing) {
    userRow =
      (txResult[idx]?.[0] as
        | {
            id: string;
            email: string;
            name: string | null;
            notes: string | null;
            status: string;
          }
        | undefined) ?? null;
  }

  const userUpdated = Boolean(userRow);
  if (!waitlistUpdated && !userUpdated) {
    return c.json({ error: 'User was deleted during update' }, 404);
  }

  await insertAuditLog(
    sql,
    'user.name.updated',
    actor,
    {
      email: normalizedEmail,
      name,
      notesProvided: notes !== undefined,
      waitlistUpdated,
      userUpdated,
      userId: userRow?.id ?? existing?.id ?? null,
    },
    authMethod
  );

  return c.json({
    email: normalizedEmail,
    name: userRow?.name ?? waitlistName ?? name,
    waitlistUpdated,
    userUpdated,
    user: userRow
      ? {
          id: userRow.id,
          email: userRow.email,
          name: userRow.name,
          notes: userRow.notes,
          status: userRow.status,
        }
      : null,
  });
});

/**
 * POST /admin/broadcast
 *
 * Generalised mail-to-many endpoint. Dispatches any registered broadcast
 * template (`release-announcement`, `waitlist-migration`, ...) to any
 * named audience (`beta:active`, `waitlist:source`, ...) under the same
 * snapshot/preview/consume + cohort-drift contract that
 * /admin/send-migration established under ADMINCLIH-001.
 *
 * Bait-and-switch defence: once a snapshot exists, the snapshot is the
 * source of truth for template, templateProps, audience_key, and
 * audience_params. The request body on real-send is ignored for those
 * fields — only previewToken matters.
 *
 * Error codes (JSON `code` field, tested in admin-broadcast.test.ts):
 *   400 template_unknown                — template not in EMAIL_REGISTRY
 *   400 template_kind_not_broadcastable — registry says kind=transactional
 *   400 template_props_invalid          — templateProps fails propsSchema
 *   400 audience_unknown                — audience not in AUDIENCE_KEYS
 *   400 audience_params_missing         — resolver needs a param the
 *                                         request did not supply
 *   400 preview_token_required          — real-send without a token
 *   410 preview_token_missing           — token unknown (or cross-actor)
 *   410 preview_token_expired           — TTL passed
 *   410 preview_token_consumed          — already used
 *   409 cohort_drift                    — re-resolved set differs
 */
admin.post('/broadcast', zValidator('json', broadcastSchema), async (c) => {
  const sql = getClient();
  const actor = resolveAdminActor(c);
  const authMethod = resolveAuthMethod(c);
  const { template, audience, audienceParams, templateProps, limit, dryRun, previewToken } =
    c.req.valid('json');

  // ---- Dry-run -------------------------------------------------------------
  // Request-time template / audience / templateProps are validated and
  // snapshotted ONLY on the dry-run leg. On a real-send the consumed
  // preview snapshot is the source of truth (EMAIL-010 / #1926), so these
  // request fields are neither required nor trusted there — validating them
  // before snapshot consumption would reject a valid preview-token-only
  // real-send (the contract-mismatch Clawpatch flagged).
  if (dryRun) {
    // The schema refine guarantees template + audience are present when
    // dryRun is true; assert for the type narrowing.
    if (!template || !audience) {
      return c.json(
        {
          code: 'audience_params_missing',
          error: 'template and audience are required when dryRun is true',
        },
        400
      );
    }

    // ---- Validate template -------------------------------------------------
    // `Object.hasOwn` rather than the `in` operator: `in` matches inherited
    // properties (e.g. `toString`, `__proto__`), letting `template: 'toString'`
    // pass this guard and then index the registry to a non-template value.
    if (!Object.hasOwn(EMAIL_REGISTRY, template)) {
      return c.json({ code: 'template_unknown', error: `unknown template: ${template}` }, 400);
    }
    const entry = EMAIL_REGISTRY[template as TemplateKey];
    if (entry.kind !== 'broadcast') {
      return c.json(
        {
          code: 'template_kind_not_broadcastable',
          error: `template '${template}' is transactional and cannot be broadcast`,
        },
        400
      );
    }

    // ---- Validate audience -------------------------------------------------
    if (!(AUDIENCE_KEYS as readonly string[]).includes(audience)) {
      return c.json({ code: 'audience_unknown', error: `unknown audience: ${audience}` }, 400);
    }
    if (audience === 'waitlist:source' && !audienceParams['source']) {
      return c.json(
        {
          code: 'audience_params_missing',
          error: "audience 'waitlist:source' requires audienceParams.source",
        },
        400
      );
    }

    // ---- Validate templateProps --------------------------------------------
    const propsParse = entry.propsSchema.safeParse(templateProps);
    if (!propsParse.success) {
      return c.json(
        {
          code: 'template_props_invalid',
          error: propsParse.error.message,
        },
        400
      );
    }
    const validatedProps = propsParse.data as Record<string, unknown>;

    const audienceRows = await resolveAudience(sql, audience as AudienceKey, {
      limit,
      params: audienceParams,
    });
    const recipients: SnapshotRecipient[] = audienceRows.map((r) => ({
      email: r.email,
      name: r.name,
    }));

    const token = randomBytes(16).toString('hex');
    const snapshot = await insertBroadcastSnapshot(sql, {
      token,
      template,
      templateProps: validatedProps,
      audienceKey: audience,
      audienceParams,
      recipients,
      createdByActor: actor,
      ttlSeconds: BROADCAST_SNAPSHOT_TTL_SECONDS,
    });

    return c.json({
      dryRun: true,
      template,
      audience,
      count: recipients.length,
      recipients,
      templateProps: validatedProps,
      previewToken: snapshot.token,
      expiresAt: snapshot.expires_at,
    });
  }

  // ---- Real-send: token required -------------------------------------------
  if (!previewToken) {
    return c.json(
      {
        code: 'preview_token_required',
        error: 'previewToken is required for real-sends; run with dryRun: true first',
      },
      400
    );
  }

  // ---- Real-send: consume snapshot atomically ------------------------------
  const consumed = await consumeBroadcastSnapshot(sql, { token: previewToken, actor });
  if (!consumed) {
    const existing = await findBroadcastSnapshot(sql, { token: previewToken, actor });
    if (!existing) {
      return c.json(
        {
          code: 'preview_token_missing',
          error: 'preview token is unknown; re-run with dryRun: true for a fresh preview',
        },
        410
      );
    }
    if (existing.consumed_at !== null) {
      return c.json(
        {
          code: 'preview_token_consumed',
          error: 'preview token has already been used; re-run with dryRun: true',
        },
        410
      );
    }
    return c.json(
      {
        code: 'preview_token_expired',
        error: 'preview token has expired; re-run with dryRun: true for a fresh preview',
      },
      410
    );
  }

  // ---- Real-send: snapshot is source of truth ------------------------------
  // Audit the dispatch BEFORE the send loop runs so a mid-loop timeout
  // / crash still leaves a forensic record: at minimum we know template,
  // audience, and recipient count even if `broadcast.email.sent` (the
  // completion row below) never fires.
  //
  // Audit metadata records the SHA-256 hash of the preview token
  // (matching the token_hash column in send_broadcast_snapshots) so
  // an investigator can correlate audit rows to snapshot rows without
  // storing the bearer in plain text.
  const previewTokenHash = hashToken(previewToken);
  await insertAuditLog(
    sql,
    'broadcast.email.dispatch_started',
    actor,
    {
      template: consumed.template,
      audience: consumed.audience_key,
      audienceParams: consumed.audience_params,
      recipientCount: consumed.recipients.length,
      previewTokenHash,
    },
    authMethod
  );

  const outcome = await executeBroadcastFromSnapshot(sql, consumed);
  if (outcome.type === 'invalid_template') {
    // Should be impossible — snapshots are only created above for
    // broadcast templates with audiences in AUDIENCE_KEYS — but defend
    // in case the registry or audience set changed between snapshot
    // and consume. The snapshot is already consumed (state change),
    // so emit an audit row so the operator can find the orphaned send.
    await insertAuditLog(
      sql,
      'broadcast.email.blocked',
      actor,
      {
        reason: 'invalid_template',
        template: consumed.template,
        audience: consumed.audience_key,
        previewTokenHash,
      },
      authMethod
    );
    return c.json(
      {
        code: 'template_kind_not_broadcastable',
        error: `snapshot template '${consumed.template}' is no longer a broadcast template`,
      },
      400
    );
  }
  if (outcome.type === 'drift') {
    await insertAuditLog(
      sql,
      'broadcast.email.blocked',
      actor,
      {
        reason: 'cohort_drift',
        template: consumed.template,
        audience: consumed.audience_key,
        audienceParams: consumed.audience_params,
        added: outcome.added,
        removed: outcome.removed,
        previewTokenHash,
      },
      authMethod
    );
    return c.json(
      {
        code: 'cohort_drift',
        error: 'recipient set changed since preview; re-run with dryRun: true',
        added: outcome.added,
        removed: outcome.removed,
      },
      409
    );
  }

  await insertAuditLog(
    sql,
    'broadcast.email.sent',
    actor,
    {
      template: consumed.template,
      audience: consumed.audience_key,
      audienceParams: consumed.audience_params,
      sent: outcome.sent,
      failed: outcome.failed,
      previewTokenHash,
      // Per-recipient failure detail for forensic recovery if the
      // client loses the HTTP response. Bounded by limit=80.
      failedRecipients: outcome.results
        .filter((r) => !r.sent)
        .map((r) => ({ email: r.email, error: r.error })),
    },
    authMethod
  );

  return c.json({
    template: consumed.template,
    audience: consumed.audience_key,
    total: outcome.total,
    sent: outcome.sent,
    failed: outcome.failed,
    results: outcome.results,
  });
});

export { admin };
