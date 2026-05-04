import type { MiddlewareHandler } from 'hono';
import { createHmac, timingSafeEqual } from 'node:crypto';
import { getClient } from '../db/client.js';
import { findAdminKeyByHash, insertAuditLog, type AuthMethod } from '../db/queries.js';
import { createDebugger } from '../lib/debug.js';

const debug = createDebugger('api');

// Sentinel actor used when the caller authenticated via the shared
// `ADMIN_KEY`. The spec mandates that shared-key requests ignore the
// client-controlled `X-Admin-Actor` header entirely, so audit rows are
// attributable to the key category rather than a self-reported string.
export const SHARED_KEY_ACTOR = 'shared-key@anvil';

// Sentinel actor used when admin auth fails (unknown bearer, revoked key).
// `audit_log.actor` is NOT NULL so we can't leave it blank.
const ADMIN_AUTH_FAILURE_ACTOR = 'admin-auth-failure@anvil';

declare module 'hono' {
  interface ContextVariableMap {
    adminActor: string;
    adminAuthMethod: AuthMethod;
    adminKeyId?: string;
  }
}

function perOperatorKeysEnabled(): boolean {
  const v = process.env['ADMIN_PER_OPERATOR_KEYS'];
  if (!v) return false;
  return v === '1' || v.toLowerCase() === 'true';
}

// Dedicated pepper for admin-key hashing, kept separate from TOKEN_PEPPER
// (which is used for access-token hashing) so rotation of either doesn't
// invalidate the other. Returns null when unset/empty so callers can treat
// that as a misconfiguration rather than silently hashing with ''.
function pepper(): string | null {
  const v = process.env['ADMIN_KEY_PEPPER'];
  if (v === undefined || v === '') return null;
  return v;
}

function logPepperMisconfig(): void {
  // Log every request this fires rather than once per process: this is a
  // prod misconfiguration we WANT to be loud about — silencing after one
  // line would let the condition hide in a long-running deployment.
  console.error(
    '[admin-auth] ADMIN_PER_OPERATOR_KEYS is enabled but ADMIN_KEY_PEPPER is unset/empty; ' +
      'skipping per-operator lookup and falling through to shared-key path. ' +
      'Provisioned per-operator keys will NOT authenticate until the pepper is set.'
  );
}

function hashBearer(bearer: string, secret: string): string {
  return createHmac('sha256', secret).update(bearer).digest('hex');
}

function sharedKeyMatches(adminKey: string, provided: string): boolean {
  const a = Buffer.from(adminKey, 'utf-8');
  const b = Buffer.from(provided, 'utf-8');
  if (a.length !== b.length) return false;
  return timingSafeEqual(a, b);
}

async function auditAuthFailure(
  outcome: 'rejected_unknown' | 'rejected_revoked' | 'rejected_malformed',
  hashedBearer: string | null,
  authMethod: AuthMethod,
  extra: Record<string, unknown> = {}
): Promise<void> {
  try {
    const sql = getClient();
    await insertAuditLog(
      sql,
      'admin.auth.failed',
      ADMIN_AUTH_FAILURE_ACTOR,
      { outcome, hashed_bearer: hashedBearer, ...extra },
      authMethod
    );
  } catch (err) {
    // Don't let an audit-write failure mask the auth rejection itself.
    debug(`admin auth: failed to write audit row for ${outcome}`, err);
  }
}

/**
 * Admin authentication middleware.
 *
 * Authenticates the presented bearer via (in priority order):
 *
 * 1. Per-operator key: HMAC-SHA-256(pepper, bearer) is looked up in the
 *    `admin_keys` table by its single UNIQUE index. Active rows authenticate
 *    the caller under the key's `actor_email` with `auth_method:
 *    'per_operator'`. Revoked rows reject with 401 `admin_key_revoked` and
 *    write an audit failure row.
 *
 * 2. Shared `ADMIN_KEY`: constant-time comparison against the env-configured
 *    shared admin key. Matches authenticate under the sentinel
 *    `shared-key@anvil` with `auth_method: 'shared'`. The
 *    `X-Admin-Actor` header is intentionally ignored on this path to
 *    eliminate the attribution-forgery vector during the dual-auth
 *    rollout window.
 *
 * A DB error during the per-operator lookup does not fail the request:
 * the middleware falls through to the shared-key path and stamps
 * `auth_method: 'shared'`. This prevents a DB hiccup from taking down the
 * entire admin surface.
 */
export const adminAuth: MiddlewareHandler = async (c, next) => {
  const adminKey = process.env['ADMIN_KEY'];
  if (!adminKey) {
    debug('admin auth: ADMIN_KEY not configured');
    return c.json({ error: 'Server misconfigured' }, 500);
  }

  const header = c.req.header('Authorization');
  if (!header) {
    debug('admin auth: missing Authorization header');
    await auditAuthFailure('rejected_malformed', null, 'shared', { reason: 'missing_header' });
    return c.json({ error: 'Authorization header required' }, 401);
  }

  const match = header.match(/^Bearer\s+(.+)$/);
  if (!match) {
    debug('admin auth: invalid authorization format');
    await auditAuthFailure('rejected_malformed', null, 'shared', { reason: 'bad_format' });
    return c.json({ error: 'Invalid authorization format' }, 401);
  }

  const provided = match[1]!;

  // Per-operator lookup only runs when the feature flag is on AND a pepper
  // is configured. Hashing with an empty pepper would (a) produce
  // predictable hashes and (b) never match provisioned rows (which are
  // hashed with the real pepper), so we refuse to hash without one.
  const pepperValue = pepper();
  const perOperatorActive = perOperatorKeysEnabled() && pepperValue !== null;
  if (perOperatorKeysEnabled() && pepperValue === null) {
    logPepperMisconfig();
  }

  if (perOperatorActive) {
    const hashed = hashBearer(provided, pepperValue);
    let adminKeyRow = null;
    try {
      const sql = getClient();
      adminKeyRow = await findAdminKeyByHash(sql, hashed);
    } catch (err) {
      // DB error: fall through to shared-key path. The spec requires this
      // so a DB hiccup can't take down the admin surface.
      debug('admin auth: admin_keys lookup failed, falling back to shared key', err);
    }

    if (adminKeyRow) {
      if (adminKeyRow.revoked_at) {
        debug('admin auth: revoked per-operator key presented');
        await auditAuthFailure('rejected_revoked', hashed, 'per_operator', {
          admin_key_id: adminKeyRow.id,
          actor_email: adminKeyRow.actor_email,
        });
        return c.json({ error: 'Admin key revoked', code: 'admin_key_revoked' }, 401);
      }
      debug('admin auth: authorized via per-operator key');
      c.set('adminActor', adminKeyRow.actor_email);
      c.set('adminAuthMethod', 'per_operator');
      c.set('adminKeyId', adminKeyRow.id);
      return await next();
    }
    // No match — fall through to shared-key path. Unknown per-operator
    // bearers might still be valid as the shared key during rollout.
  }

  if (sharedKeyMatches(adminKey, provided)) {
    debug('admin auth: authorized via shared key');
    c.set('adminActor', SHARED_KEY_ACTOR);
    c.set('adminAuthMethod', 'shared');
    return await next();
  }

  debug('admin auth: unknown bearer');
  const hashedForAudit = perOperatorActive ? hashBearer(provided, pepperValue) : null;
  await auditAuthFailure(
    'rejected_unknown',
    hashedForAudit,
    perOperatorActive ? 'per_operator' : 'shared'
  );
  return c.json({ error: 'Forbidden' }, 403);
};
