import { Hono } from 'hono';
import { z } from 'zod';
import { zValidator } from '@hono/zod-validator';
import { getClient } from '../db/client.js';
import { findTokenByHash, findActiveScopesForUser, findUserById } from '../db/queries.js';
import { hashToken, isValidTokenFormat } from '../lib/token.js';
import { createDebugger } from '../lib/debug.js';
import { signLicence, verifyLicence } from '../lib/licence.js';

const debug = createDebugger('api');

// Generous enough for an ES256 licence JWT (~700-900 chars), which /verify
// accepts since CIB-066; the old max(200) rejected licences at the schema
// boundary with a 400 before any verification ran.
const verifySchema = z.object({
  token: z.string().max(4096),
});

const auth = new Hono();

/**
 * POST /auth/verify
 *
 * Validates a credential and reports the identity behind it. Two forms:
 *
 * - `anvil_beta_…` access token: hashed DB lookup (revocation, expiry, and
 *   user-status checks) — returns a freshly signed licence.
 * - ES256 licence JWT (CIB-066): interactive logins (GitHub device flow /
 *   OTP) store the licence as the credential and the CLI's `whoami` sends it
 *   here. Verified via the licence verifying key; revocation parity comes
 *   from the account-status gate (an account-level revoke suspends the user,
 *   which fails the check — grant-level token revocation does not apply to
 *   licence credentials, which expire on their own 7-day TTL).
 *
 * Returns 200 — {valid: false} on any credential failure (no reason
 * leakage), with two exceptions: 400 when the request body fails schema
 * validation (zValidator), and 503 when the verifying key is unavailable
 * (server misconfiguration, not a caller failure). The licence-path
 * response omits `license` and `expiresAt` — those are access-token-path
 * fields (the caller already holds the licence; its expiry is inside the
 * JWT).
 */
auth.post('/verify', zValidator('json', verifySchema), async (c) => {
  debug('POST /auth/verify');
  const { token } = c.req.valid('json');

  if (!isValidTokenFormat(token)) {
    // Only structurally JWT-shaped credentials reach the verifying key:
    // arbitrary garbage stays {valid: false} and must not surface the 503
    // misconfiguration signal below — the caller didn't cause it, and the
    // key never needs loading to reject a string that cannot be a licence.
    if (!/^[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+$/.test(token)) {
      debug('not a valid access token or licence-shaped JWT');
      return c.json({ valid: false });
    }
    let claims;
    try {
      claims = await verifyLicence(token);
    } catch (err) {
      // loadVerifyingKey throws when LICENSE_PUBLIC_KEY is missing or
      // malformed — surface as server misconfiguration, never as
      // "your credentials are invalid".
      debug('licence verification unavailable', {
        error: err instanceof Error ? err.message : String(err),
      });
      return c.json({ error: 'verification_unavailable' }, 503);
    }
    if (!claims) {
      debug('not a valid access token or licence');
      return c.json({ valid: false });
    }

    const sql = getClient();
    const user = await findUserById(sql, claims.sub);
    if (!user || user.status !== 'active') {
      debug('licence subject not active', { status: user?.status });
      return c.json({ valid: false });
    }

    debug('licence verified successfully');
    // BACT-013: prefer the freshly-read DB plan over the (possibly stale)
    // licence claim — an account's plan can change after a licence was
    // minted. Fall back to the claim only when the row itself has no plan
    // (fixture tolerance; real rows always carry it post-021).
    return c.json({
      valid: true,
      isEdict: false,
      user: { email: claims.email, plan: user.plan ?? claims.plan },
      scopes: claims.scopes,
    });
  }

  const sql = getClient();
  const hash = hashToken(token);
  const record = await findTokenByHash(sql, hash);

  if (!record) {
    return c.json({ valid: false });
  }

  // Check revocation
  if (record.revoked_at) {
    debug('token revoked');
    return c.json({ valid: false });
  }

  // Check expiry
  if (new Date(record.expires_at).getTime() < Date.now()) {
    debug('token expired');
    return c.json({ valid: false });
  }

  // Check user status
  if (record.user_status !== 'active') {
    debug('user not active', { status: record.user_status });
    return c.json({ valid: false });
  }

  debug('token verified successfully');
  // Read live scopes via findActiveScopesForUser so the licence claim
  // matches what /session/refresh, /auth/device, /auth/github, and
  // /auth/otp issue. Using the single record.scopes was a stale-read
  // path: a user invited with `['preview', 'beta']` who later got an
  // additional `['beta']` token would have only one row's scopes
  // returned here, while the union endpoints return both.
  // identity.provider is 'email' to match the other token-issuance
  // surfaces — a token-verify request has no provider context (it is
  // not an OAuth callback), and 'github' was a copy-paste artefact.
  const scopes = await findActiveScopesForUser(sql, record.user_id);
  let licence: string;
  try {
    licence = await signLicence(
      {
        sub: record.user_id,
        email: record.email,
        identity: { provider: 'email', id: null },
        org: null,
        // BACT-013: the joined account plan (findTokenByHash SELECTs
        // u.plan); 'beta' default matches the column DEFAULT for rows
        // predating the join (fixture tolerance).
        plan: record.plan ?? 'beta',
        scopes,
        seats: 1,
      },
      record.expires_at
    );
  } catch (err) {
    debug('licence signing failed', { error: String(err) });
    return c.json({ valid: false });
  }

  return c.json({
    valid: true,
    isEdict: record.is_edict,
    user: { email: record.email, plan: record.plan ?? 'beta' },
    scopes,
    expiresAt: record.expires_at,
    license: licence,
  });
});

auth.post('/license/refresh', zValidator('json', verifySchema), async (c) => {
  debug('POST /auth/license/refresh');
  const { token } = c.req.valid('json');

  if (!isValidTokenFormat(token)) {
    return c.json({ valid: false });
  }

  const sql = getClient();
  const hash = hashToken(token);
  const record = await findTokenByHash(sql, hash);

  if (!record || record.revoked_at || record.user_status !== 'active') {
    return c.json({ valid: false, reason: record?.revoked_at ? 'revoked' : 'invalid' });
  }

  if (new Date(record.expires_at).getTime() < Date.now()) {
    return c.json({ valid: false, reason: 'expired' });
  }

  // Same scope-union + identity-provider fix as /auth/verify above.
  const scopes = await findActiveScopesForUser(sql, record.user_id);
  let licence: string;
  try {
    licence = await signLicence(
      {
        sub: record.user_id,
        email: record.email,
        identity: { provider: 'email', id: null },
        org: null,
        // BACT-013: see the identical comment on /auth/verify above.
        plan: record.plan ?? 'beta',
        scopes,
        seats: 1,
      },
      record.expires_at
    );
  } catch (err) {
    debug('licence signing failed', { error: String(err) });
    return c.json({ error: 'internal' }, 500);
  }

  return c.json({ license: licence });
});

export { auth };
