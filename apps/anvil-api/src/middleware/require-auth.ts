import type { MiddlewareHandler } from 'hono';
import { verifyLicence, type LicenceClaims } from '../lib/licence.js';
import { createDebugger } from '../lib/debug.js';

const debug = createDebugger('require-auth');

/**
 * Hono middleware that requires a valid anvil-issued licence JWT in the
 * `Authorization: Bearer <jwt>` header. On success, attaches the verified
 * claims to the request context under the `authed` key and proceeds. On
 * any failure (missing header, malformed Bearer, invalid signature,
 * expired, wrong issuer/audience) responds 401 and short-circuits.
 *
 * Callers downstream read the authed identity via `c.get('authed')` and
 * MUST NOT trust any caller-supplied email or user identifier in the
 * request body when an authenticated identity is available — that was the
 * exact CLAWP-2026-05-20 critical (issue #1779) where /device/confirm
 * trusted a body-supplied email and let any caller mint tokens for any
 * active user.
 */
export function requireAuth(): MiddlewareHandler<{
  Variables: { authed: LicenceClaims };
}> {
  return async (c, next) => {
    const header = c.req.header('Authorization');
    if (!header) {
      debug('require-auth: missing Authorization header');
      return c.json({ error: 'Authorization header required' }, 401);
    }
    const match = header.match(/^Bearer\s+(.+)$/);
    if (!match) {
      debug('require-auth: malformed Authorization header');
      return c.json({ error: 'Authorization header must be Bearer <jwt>' }, 401);
    }
    const jwt = match[1]!.trim();
    if (!jwt) {
      debug('require-auth: empty bearer');
      return c.json({ error: 'Authorization header must be Bearer <jwt>' }, 401);
    }

    const claims = await verifyLicence(jwt);
    if (!claims) {
      debug('require-auth: licence verification failed');
      return c.json({ error: 'Invalid or expired licence' }, 401);
    }

    c.set('authed', claims);
    return await next();
  };
}
