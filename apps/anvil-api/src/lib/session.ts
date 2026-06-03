import { randomBytes, randomUUID } from 'node:crypto';
import type { NeonClient } from '../db/client.js';
import { findActiveScopesForUser, insertRefreshToken } from '../db/queries.js';
import { signLicence, type LicenceClaims } from './licence.js';
import { hashToken } from './token.js';

/** Access-token (JWT licence) lifetime used by every interactive auth path. */
const DEFAULT_LICENCE_TTL_DAYS = 7;
/** Refresh-token lifetime used by every interactive auth path. */
const DEFAULT_REFRESH_TTL_DAYS = 90;

const DAY_MS = 24 * 60 * 60 * 1000;

export interface MintSessionInput {
  /** The authenticated, active beta user the session is for. */
  user: { id: string; email: string };
  /** Provider identity stamped into the licence claim (`github` or `email`). */
  identity: LicenceClaims['identity'];
  /** Licence (access-token) lifetime in days. Defaults to 7. */
  ttlDays?: number;
  /** Refresh-token lifetime in days. Defaults to 90. */
  refreshTtlDays?: number;
  /**
   * Refresh-token family to issue the new token under. Omit to start a fresh
   * family (device-code / OTP / GitHub first issue); pass an existing family to
   * rotate within it (session refresh).
   */
  familyId?: string;
}

export interface MintSessionResult {
  license: string;
  refreshToken: string;
  expiresAt: string;
}

/**
 * Mint an Anvil session for an already-validated, active user: resolve the
 * user's current scopes, sign a licence with the given identity, and issue a
 * rotating refresh token.
 *
 * This is the shared tail of every interactive auth path (`/auth/github/callback`,
 * `/auth/otp/verify`, the device-code activation, and `/session/refresh`). The
 * caller owns user lookup, the active-status gate, audit logging, and — for the
 * GitHub paths — revoking the upstream GitHub token; this helper owns only the
 * scope→claim→sign→refresh-token tail so those paths stay byte-identical.
 */
export async function mintSession(
  sql: NeonClient,
  input: MintSessionInput
): Promise<MintSessionResult> {
  const {
    user,
    identity,
    ttlDays = DEFAULT_LICENCE_TTL_DAYS,
    refreshTtlDays = DEFAULT_REFRESH_TTL_DAYS,
    familyId = randomUUID(),
  } = input;

  const scopes = await findActiveScopesForUser(sql, user.id);

  const claims: LicenceClaims = {
    sub: user.id,
    email: user.email,
    identity,
    org: null,
    tier: 'pro',
    scopes,
    seats: 1,
  };

  const license = await signLicence(claims, undefined, ttlDays);

  const rawRefreshToken = randomBytes(32).toString('hex');
  const refreshHash = hashToken(rawRefreshToken);
  const refreshExpiresAt = new Date(Date.now() + refreshTtlDays * DAY_MS);
  await insertRefreshToken(sql, user.id, refreshHash, familyId, refreshExpiresAt);

  const expiresAt = new Date(Date.now() + ttlDays * DAY_MS).toISOString();

  return { license, refreshToken: rawRefreshToken, expiresAt };
}
