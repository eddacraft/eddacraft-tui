import { randomBytes, randomUUID } from 'node:crypto';
import type { NeonClient } from '../db/client.js';
import {
  consumeAndRotateRefreshToken,
  findActiveScopesForUser,
  insertRefreshToken,
  stampUserActivity,
  stampUserLogin,
  type LoginMethod,
} from '../db/queries.js';
import { signLicence, type LicenceClaims } from './licence.js';
import { hashToken } from './token.js';

export type { LoginMethod };

/** Access-token (JWT licence) lifetime used by every interactive auth path. */
const DEFAULT_LICENCE_TTL_DAYS = 7;
/** Refresh-token lifetime used by every interactive auth path. */
const DEFAULT_REFRESH_TTL_DAYS = 90;

const DAY_MS = 24 * 60 * 60 * 1000;

/** BACT-013: matches the `beta_users.plan` column DEFAULT (migration 021). */
const DEFAULT_PLAN = 'beta';

export interface MintSessionInput {
  /**
   * The authenticated, active beta user the session is for. `plan` is
   * optional here only for narrow test/legacy callers — real rows always
   * carry `plan` (BACT-008 column DEFAULT 'beta'); omit it and the licence
   * still gets a safe 'beta' default (BACT-013).
   */
  user: { id: string; email: string; plan?: string | null };
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
  /**
   * BACT-002: when set, stamp first/last interactive login on `beta_users`.
   * Omit for paths that mint sessions without a fresh interactive login
   * (none today — refresh uses `mintRotatedSession` and never stamps).
   */
  loginMethod?: LoginMethod;
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
 * This is the shared tail of every **interactive** auth path (`/auth/github/callback`,
 * `/auth/otp/verify`, GitHub device poll, and legacy device-code activation).
 * Session refresh uses `mintRotatedSession` instead and does **not** stamp
 * BACT login fields. The caller owns user lookup, the active-status gate,
 * audit logging, and — for the GitHub paths — revoking the upstream GitHub
 * token; this helper owns only the scope→claim→sign→refresh-token tail (plus
 * optional login stamps when `loginMethod` is set).
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
    loginMethod,
  } = input;

  const scopes = await findActiveScopesForUser(sql, user.id);

  const claims: LicenceClaims = {
    sub: user.id,
    email: user.email,
    identity,
    org: null,
    plan: user.plan ?? DEFAULT_PLAN,
    scopes,
    seats: 1,
  };

  const license = await signLicence(claims, undefined, ttlDays);

  const rawRefreshToken = randomBytes(32).toString('hex');
  const refreshHash = hashToken(rawRefreshToken);
  const refreshExpiresAt = new Date(Date.now() + refreshTtlDays * DAY_MS);
  await insertRefreshToken(sql, user.id, refreshHash, familyId, refreshExpiresAt);

  if (loginMethod) {
    await stampUserLogin(sql, user.id, loginMethod);
  }

  const expiresAt = new Date(Date.now() + ttlDays * DAY_MS).toISOString();

  return { license, refreshToken: rawRefreshToken, expiresAt };
}

export type MintRotatedSessionResult = { ok: true; session: MintSessionResult } | { ok: false };

/**
 * Mint a session by rotating an existing refresh token. Consumes the old
 * token and inserts the replacement in one atomic statement so concurrent
 * family revocation cannot leave a live post-revoke refresh token.
 *
 * On `{ ok: false }` the caller should treat the rotation as theft-detection
 * failure (revoke the family and return 401) — the same response used when
 * the non-atomic consume previously lost its race.
 */
export async function mintRotatedSession(
  sql: NeonClient,
  input: MintSessionInput & { oldTokenId: string; familyId: string }
): Promise<MintRotatedSessionResult> {
  const {
    user,
    identity,
    ttlDays = DEFAULT_LICENCE_TTL_DAYS,
    refreshTtlDays = DEFAULT_REFRESH_TTL_DAYS,
    familyId,
    oldTokenId,
  } = input;

  const rawRefreshToken = randomBytes(32).toString('hex');
  const refreshHash = hashToken(rawRefreshToken);
  const refreshExpiresAt = new Date(Date.now() + refreshTtlDays * DAY_MS);

  const rotated = await consumeAndRotateRefreshToken(sql, {
    oldTokenId,
    userId: user.id,
    newTokenHash: refreshHash,
    familyId,
    expiresAt: refreshExpiresAt,
  });
  if (rotated.status !== 'rotated') {
    return { ok: false };
  }

  // BACT-008 / ADR-121 decision 4: a successful refresh is account activity
  // for token-era users who never re-run interactive login, but it is never
  // a login — only `stampUserActivity` (kind `refresh`) runs here, never
  // `stampUserLogin`. Best-effort: this runs after the refresh token is
  // already atomically rotated (the old token is consumed), so a stamp
  // failure must not cost the caller the new session — that would force
  // re-auth for no reason (e.g. migration drift mid-rollout).
  try {
    await stampUserActivity(sql, user.id, 'refresh');
  } catch (err) {
    console.error('session refresh activity stamp failed (non-fatal):', err);
  }

  const scopes = await findActiveScopesForUser(sql, user.id);

  const claims: LicenceClaims = {
    sub: user.id,
    email: user.email,
    identity,
    org: null,
    plan: user.plan ?? DEFAULT_PLAN,
    scopes,
    seats: 1,
  };

  const license = await signLicence(claims, undefined, ttlDays);
  const expiresAt = new Date(Date.now() + ttlDays * DAY_MS).toISOString();

  return {
    ok: true,
    session: { license, refreshToken: rawRefreshToken, expiresAt },
  };
}
