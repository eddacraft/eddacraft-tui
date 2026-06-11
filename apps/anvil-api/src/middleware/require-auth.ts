import type { MiddlewareHandler } from 'hono';
import { verifyLicence, type LicenceClaims } from '../lib/licence.js';
import { createDebugger } from '../lib/debug.js';

const debug = createDebugger('require-auth');

/**
 * Hono middleware that requires a valid anvil-issued licence JWT in the
 * `Authorization: Bearer <jwt>` header. On success, attaches the verified
 * claims to the request context under the `authed` key and proceeds.
 *
 * Failure shapes:
 *
 * - **Missing / malformed header** → 401 (`Authorization header required` or
 *   `Authorization header must be Bearer <jwt>`).
 * - **Verification failure** (bad signature, expired, wrong issuer or
 *   audience, malformed claims) → 401 (`Invalid or expired licence`).
 * - **Server misconfiguration** (`LICENSE_PUBLIC_KEY` missing or
 *   unparseable — `verifyLicence` throws on key-load errors so silent
 *   downgrade-to-allow is impossible) → 500 (`Server misconfigured`).
 *   This case is mutually exclusive with the 401 paths; it surfaces a
 *   deploy-time problem, not a client-attribution problem, so callers can
 *   page on it.
 *
 * Callers downstream read the authed identity via `c.get('authed')` and
 * MUST NOT trust any caller-supplied email or user identifier in the
 * request body when an authenticated identity is available — that was the
 * exact CLAWP-2026-05-20 critical (issue #1779), where a device-flow
 * confirm route trusted a body-supplied email and let any caller mint
 * tokens for any active user.
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

    let claims: LicenceClaims | null;
    try {
      claims = await verifyLicence(jwt);
    } catch (err) {
      // Configuration error — LICENSE_PUBLIC_KEY missing or unparseable.
      // Distinct from a verification failure: this is a deploy-time issue
      // that must page operators, not a 401 attributable to the caller.
      debug('require-auth: licence verification threw (server misconfigured)', err);
      console.error('[require-auth] licence verification key unavailable:', err);
      return c.json({ error: 'Server misconfigured' }, 500);
    }
    if (!claims) {
      debug('require-auth: licence verification failed');
      return c.json({ error: 'Invalid or expired licence' }, 401);
    }

    c.set('authed', claims);
    return await next();
  };
}
