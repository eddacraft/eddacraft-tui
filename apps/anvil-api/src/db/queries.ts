import { z } from 'zod';
import type { NeonClient } from './client.js';
import { hashToken } from '../lib/token.js';
import type { GitHubIdentity } from '../lib/github-user.js';
import { validateTelemetryRetentionDays } from '../lib/telemetry-retention.js';

const IdSchema = z.union([z.string(), z.number(), z.bigint()]).transform((v) => String(v));

const DateStringSchema = z
  .union([z.string(), z.date()])
  .transform((v) => (v instanceof Date ? v.toISOString() : v));

const TextArraySchema = z.union([z.array(z.string()), z.string()]).transform((v) => {
  if (Array.isArray(v)) return v;

  // Handle Postgres array text format: {beta} / {"beta","preview"}
  const raw = v.trim();
  if (raw.startsWith('{') && raw.endsWith('}')) {
    const inner = raw.slice(1, -1).trim();
    if (!inner) return [];
    return inner.split(',').map((item) => {
      const unquoted = item.replace(/^"(.*)"$/, '$1');
      return unquoted.replace(/\\"/g, '"');
    });
  }

  // Fallback for unexpected string payloads
  return [raw];
});

const BetaUserSchema = z.object({
  id: IdSchema,
  email: z.string(),
  name: z.string().nullable(),
  status: z.string(),
  notes: z.string().nullable(),
  // GHCLIAUTH-003: GitHub numeric id, linked on first GitHub login. The Neon
  // driver may surface a bigint as a string; coerce to number (GitHub ids are
  // well under 2^53). `z.null()` is listed first so an explicit NULL is not
  // coerced to 0. `.optional()` keeps the output type assignable from row
  // fixtures that predate this column (real rows always carry it post-015).
  github_id: z.union([z.null(), z.coerce.number()]).optional(),
  // BACT-002: interactive login stamps. Optional/null for pre-migration rows
  // and invite-only accounts that have never completed a session mint.
  first_login_at: z.union([DateStringSchema, z.null()]).optional(),
  last_login_at: z.union([DateStringSchema, z.null()]).optional(),
  last_login_method: z.union([z.enum(['github', 'otp', 'device']), z.null()]).optional(),
  created_at: DateStringSchema,
  updated_at: DateStringSchema,
});

const AccessTokenSchema = z.object({
  id: IdSchema,
  user_id: IdSchema,
  token_hash: z.string(),
  scopes: TextArraySchema,
  is_edict: z.boolean().default(false),
  expires_at: DateStringSchema,
  revoked_at: z.union([DateStringSchema, z.null()]),
  created_at: DateStringSchema,
});

const AuthMethodSchema = z.enum(['shared', 'per_operator']);

export type AuthMethod = z.infer<typeof AuthMethodSchema>;

const AuditEntrySchema = z.object({
  id: IdSchema,
  action: z.string(),
  actor: z.string(),
  metadata: z
    .union([z.record(z.string(), z.unknown()), z.null(), z.undefined()])
    .transform((v) => v ?? {}),
  // `auth_method` is added in migration 009. Old rows written before the
  // migration defaulted to `'shared'`; new rows are written by
  // admin-auth middleware. Optional here so tests with legacy fixtures
  // don't need to backfill the column.
  auth_method: z.union([AuthMethodSchema, z.null(), z.undefined()]).optional(),
  created_at: DateStringSchema,
});

export type BetaUser = z.infer<typeof BetaUserSchema>;
export type AccessToken = z.infer<typeof AccessTokenSchema>;
export type AuditEntry = z.infer<typeof AuditEntrySchema>;

// Helper to cast Neon query results (returns union type) to row array
function rows(result: unknown): Record<string, unknown>[] {
  return result as Record<string, unknown>[];
}

/** Closed set of interactive login methods stamped by BACT-002. */
export type LoginMethod = 'github' | 'otp' | 'device';

/**
 * Record an interactive login for a beta user (BACT-002).
 *
 * Sets `first_login_at` only when still null; always refreshes
 * `last_login_at` and `last_login_method`. Invite/approve token mint must
 * not call this — only real session mint paths.
 */
export async function stampUserLogin(
  sql: NeonClient,
  userId: string,
  method: LoginMethod
): Promise<void> {
  await sql`
    UPDATE beta_users
    SET
      first_login_at = COALESCE(first_login_at, now()),
      last_login_at = now(),
      last_login_method = ${method},
      updated_at = now()
    WHERE id = ${userId}
  `;
}

export async function findUserByEmail(sql: NeonClient, email: string): Promise<BetaUser | null> {
  const r = rows(
    await sql`
    SELECT * FROM beta_users WHERE email = ${email} LIMIT 1
  `
  );
  if (!r[0]) return null;
  return BetaUserSchema.parse(r[0]);
}

export async function upsertUser(
  sql: NeonClient,
  email: string,
  name?: string,
  notes?: string
): Promise<BetaUser> {
  const r = rows(
    await sql`
    INSERT INTO beta_users (email, name, notes)
    VALUES (${email}, ${name ?? null}, ${notes ?? null})
    ON CONFLICT (email) DO UPDATE SET
      name = COALESCE(${name ?? null}, beta_users.name),
      notes = COALESCE(${notes ?? null}, beta_users.notes)
    RETURNING *
  `
  );
  return BetaUserSchema.parse(r[0]);
}

export async function insertToken(
  sql: NeonClient,
  userId: string,
  tokenHash: string,
  scopes: string[],
  expiresAt: Date
): Promise<AccessToken> {
  const r = rows(
    await sql`
    INSERT INTO access_tokens (user_id, token_hash, scopes, expires_at)
    VALUES (${userId}, ${tokenHash}, ${scopes}, ${expiresAt.toISOString()})
    RETURNING *
  `
  );
  return AccessTokenSchema.parse(r[0]);
}

export async function findTokenByHash(
  sql: NeonClient,
  tokenHash: string
): Promise<(AccessToken & { email: string; user_status: string }) | null> {
  const r = rows(
    await sql`
    SELECT t.*, u.email, u.status AS user_status
    FROM access_tokens t
    JOIN beta_users u ON u.id = t.user_id
    WHERE t.token_hash = ${tokenHash}
    LIMIT 1
  `
  );
  if (!r[0]) return null;
  const TokenWithUserSchema = AccessTokenSchema.extend({
    email: z.string(),
    user_status: z.coerce.string(),
  });
  return TokenWithUserSchema.parse(r[0]);
}

/**
 * Look up the scopes a user is currently entitled to, returning the UNION of
 * all `scopes` arrays across every non-revoked, non-expired `access_tokens`
 * row for the user.
 *
 * Returning the union — not the most-recent row's scopes — is required for
 * correctness. A user invited with `['preview', 'beta']` via `/admin/invite`
 * who later receives any narrower-scoped token row (e.g. the legacy
 * `/admin/approve` defaulting to `['beta']`, a re-invite, a CI service token)
 * would otherwise have the broader scope silently shadowed by the
 * later-inserted narrower one — re-introducing the same FLAGM-005 downgrade
 * the route-level fix in eae47b3d intended to close.
 *
 * The JWT licence carries `scopes` claims, but those claims are issued at
 * sign time — `/session/refresh` and the device/github/otp first-issuance
 * paths read from this function so scope changes flow into the next licence.
 *
 * Returns `['beta']` as a conservative default only when no active
 * access_tokens row exists — that path covers self-signup users who have not
 * yet been issued a graded scope. If active rows exist but all unnest to zero
 * scopes, return [] so callers do not silently re-grant beta.
 */
export async function findActiveScopesForUser(sql: NeonClient, userId: string): Promise<string[]> {
  const r = rows(
    await sql`
    SELECT
      COUNT(*)::int AS active_token_count,
      COALESCE(
      ARRAY(
        SELECT DISTINCT scope
        FROM access_tokens, unnest(scopes) AS scope
        WHERE user_id = ${userId}
          AND revoked_at IS NULL
          AND expires_at > now()
      ),
      ARRAY[]::text[]
    ) AS scopes
    FROM access_tokens
    WHERE user_id = ${userId}
      AND revoked_at IS NULL
      AND expires_at > now()
  `
  );
  if (!r[0]) {
    return ['beta'];
  }
  const ScopeSchema = z.object({
    active_token_count: z.coerce.number().default(0),
    scopes: z.array(z.string()).default(['beta']),
  });
  const parsed = ScopeSchema.parse(r[0]);
  return parsed.active_token_count > 0 ? parsed.scopes : ['beta'];
}

// SEC-007 / GH #1672: `revokeTokensByEmail`, `revokeTokenByHash`, and
// `revokeAccessTokensByUserId` were removed. They only touched
// `access_tokens`, leaving `refresh_tokens` usable, and had no callers.
// `POST /admin/revoke` in `routes/admin.ts` now performs the atomic
// access + refresh + status update in a single Neon batch transaction;
// `revokeRefreshFamilyAndAccessTokensForUser` (below) still covers the
// theft-detection path on `/session/refresh`.

export async function findUserWithTokens(
  sql: NeonClient,
  email: string
): Promise<{ user: BetaUser; tokens: AccessToken[] } | null> {
  const user = await findUserByEmail(sql, email);
  if (!user) return null;

  const r = rows(
    await sql`
    SELECT * FROM access_tokens
    WHERE user_id = ${user.id}
    ORDER BY created_at DESC
  `
  );
  return { user, tokens: z.array(AccessTokenSchema).parse(r) };
}

export async function insertAuditLog(
  sql: NeonClient,
  action: string,
  actor: string,
  metadata: Record<string, unknown> = {},
  authMethod: AuthMethod = 'shared'
): Promise<AuditEntry> {
  const r = rows(
    await sql`
    INSERT INTO audit_log (action, actor, metadata, auth_method)
    VALUES (${action}, ${actor}, ${JSON.stringify(metadata)}, ${authMethod})
    RETURNING *
  `
  );
  return AuditEntrySchema.parse(r[0]);
}

// ---------------------------------------------------------------------------
// Device codes
// ---------------------------------------------------------------------------

const DeviceCodeSchema = z.object({
  id: IdSchema,
  user_id: z.union([IdSchema, z.null()]),
  user_code: z.string(),
  poll_token: z.string(),
  confirmed_at: z.union([DateStringSchema, z.null()]),
  expires_at: DateStringSchema,
  last_polled_at: z.union([DateStringSchema, z.null()]),
  attempts: z.coerce.number(),
  created_at: DateStringSchema,
});

export type DeviceCode = z.infer<typeof DeviceCodeSchema>;

export async function insertDeviceCode(
  sql: NeonClient,
  userId: string,
  userCode: string,
  pollTokenHash: string,
  expiresAt: Date
): Promise<DeviceCode> {
  const r = rows(
    await sql`
    INSERT INTO device_codes (user_id, user_code, poll_token, expires_at)
    VALUES (${userId}, ${userCode}, ${pollTokenHash}, ${expiresAt.toISOString()})
    RETURNING *
  `
  );
  return DeviceCodeSchema.parse(r[0]);
}

export async function findDeviceCodeByUserCode(
  sql: NeonClient,
  userCode: string
): Promise<DeviceCode | null> {
  const r = rows(
    await sql`
    SELECT * FROM device_codes WHERE user_code = ${userCode} LIMIT 1
  `
  );
  if (!r[0]) return null;
  return DeviceCodeSchema.parse(r[0]);
}

export async function findDeviceCodeByPollToken(
  sql: NeonClient,
  pollTokenHash: string
): Promise<DeviceCode | null> {
  const r = rows(
    await sql`
    SELECT * FROM device_codes WHERE poll_token = ${pollTokenHash} LIMIT 1
  `
  );
  if (!r[0]) return null;
  return DeviceCodeSchema.parse(r[0]);
}

// ---------------------------------------------------------------------------
// GitHub device-flow sessions (GHCLIAUTH-004/-005, ADR-066)
// ---------------------------------------------------------------------------

const GithubDeviceSessionSchema = z.object({
  id: IdSchema,
  poll_token_hash: z.string(),
  // Encrypted (AES-256-GCM keyed off the client-held poll_token), not hashed:
  // the poll broker must recover the plaintext device_code for the RFC 8628
  // token exchange. See `lib/github-device-crypto.ts`.
  github_device_code_enc: z.string(),
  interval_s: z.coerce.number(),
  expires_at: DateStringSchema,
  last_polled_at: z.union([DateStringSchema, z.null()]),
  minted_at: z.union([DateStringSchema, z.null()]),
  minted_session_enc: z.union([z.string(), z.null()]),
  created_at: DateStringSchema,
});

export type GithubDeviceSession = z.infer<typeof GithubDeviceSessionSchema>;

export async function findGithubDeviceSessionByPollTokenHash(
  sql: NeonClient,
  pollTokenHash: string
): Promise<GithubDeviceSession | null> {
  const r = rows(
    await sql`
    SELECT * FROM github_device_sessions
    WHERE poll_token_hash = ${pollTokenHash}
    LIMIT 1
  `
  );
  if (!r[0]) return null;
  return GithubDeviceSessionSchema.parse(r[0]);
}

/**
 * Cross-instance poll gate (ADR-066 ops precondition): atomically claim the
 * right to exchange this session's device_code with GitHub for the current
 * interval window. At most one caller — across all Vercel instances — wins per
 * `interval_s`; losers are rate-limited by the route. Minted and expired
 * sessions are never claimable.
 */
export async function claimGithubDevicePoll(
  sql: NeonClient,
  pollTokenHash: string
): Promise<GithubDeviceSession | null> {
  const r = rows(
    await sql`
    UPDATE github_device_sessions
    SET last_polled_at = now()
    WHERE poll_token_hash = ${pollTokenHash}
      AND minted_at IS NULL
      AND expires_at > now()
      AND (last_polled_at IS NULL OR last_polled_at <= now() - make_interval(secs => interval_s))
    RETURNING *
  `
  );
  if (!r[0]) return null;
  return GithubDeviceSessionSchema.parse(r[0]);
}

/**
 * Single-use mint claim (reuses the `consumeDeviceCode` atomicity model with
 * UPDATE-where-unminted instead of DELETE, so the minted session stays
 * re-returnable within TTL): records the encrypted minted session exactly
 * once. Returns false when a concurrent caller already minted — the loser
 * must re-read and re-return the winner's stored session.
 */
export async function storeGithubDeviceMint(
  sql: NeonClient,
  pollTokenHash: string,
  mintedSessionEnc: string
): Promise<boolean> {
  const r = rows(
    await sql`
    UPDATE github_device_sessions
    SET minted_at = now(), minted_session_enc = ${mintedSessionEnc}
    WHERE poll_token_hash = ${pollTokenHash}
      AND minted_at IS NULL
    RETURNING id
  `
  );
  return r.length > 0;
}

/**
 * Persist a brokered GitHub device-flow session. Deliberately binds NO user —
 * the bound user is derived solely from the GitHub token at poll-confirmation
 * time (ADR-066 security invariant). The device_code is stored encrypted
 * (AES-256-GCM keyed off the client-held poll_token), not hashed: the poll
 * broker must recover the plaintext for the RFC 8628 token exchange. See
 * `lib/github-device-crypto.ts`.
 */
export async function insertGithubDeviceSession(
  sql: NeonClient,
  session: {
    pollTokenHash: string;
    deviceCodeEnc: string;
    intervalS: number;
    expiresAt: Date;
  }
): Promise<void> {
  await sql`
    INSERT INTO github_device_sessions
      (poll_token_hash, github_device_code_enc, interval_s, expires_at)
    VALUES
      (${session.pollTokenHash}, ${session.deviceCodeEnc}, ${session.intervalS},
       ${session.expiresAt.toISOString()})
  `;
}

// ---------------------------------------------------------------------------
// OTP codes
// ---------------------------------------------------------------------------

const OtpCodeSchema = z.object({
  id: IdSchema,
  user_id: IdSchema,
  code_hash: z.string(),
  attempts: z.coerce.number(),
  expires_at: DateStringSchema,
  consumed_at: z.union([DateStringSchema, z.null()]),
  created_at: DateStringSchema,
});

export type OtpCode = z.infer<typeof OtpCodeSchema>;

export async function insertOtpCode(
  sql: NeonClient,
  userId: string,
  codeHash: string,
  expiresAt: Date
): Promise<OtpCode> {
  const r = rows(
    await sql`
    INSERT INTO otp_codes (user_id, code_hash, expires_at)
    VALUES (${userId}, ${codeHash}, ${expiresAt.toISOString()})
    RETURNING *
  `
  );
  return OtpCodeSchema.parse(r[0]);
}

/**
 * Atomically register a verification attempt against a user's active OTP
 * codes and return only the codes still eligible for comparison.
 *
 * The attempt counter is incremented and the below-cap guard is applied in a
 * SINGLE `UPDATE ... WHERE attempts < $max RETURNING` statement. Because the
 * read of `attempts`, the cap check, and the write happen inside one
 * statement, there is no check-then-increment window: concurrent
 * verifications serialise on each row's write lock, so at most `maxAttempts`
 * guesses are ever incremented — and therefore ever returned for code
 * comparison — per code. This is the fix for the CIB-142 race, where the old
 * find-then-increment flow let N concurrent guesses all read the same stale
 * `attempts` snapshot and slip past the cap.
 *
 * A code that has already reached the cap fails the `attempts < $max`
 * predicate, so it is neither incremented nor returned; its `code_hash` is
 * never handed back to the caller and the guess is rejected WITHOUT the code
 * being evaluated.
 *
 * Atomicity note: the Neon serverless driver issues each tagged-template call
 * as its own autocommit statement over HTTP. A single `UPDATE` is atomic under
 * PostgreSQL read-committed semantics — a concurrent updater blocks on the row
 * lock and re-evaluates `attempts < $max` against the freshly committed value,
 * never against a stale snapshot. Same single-statement primitive as
 * `consumeOtpCode` and `claimGithubDevicePoll` above.
 */
export async function registerActiveOtpAttempts(
  sql: NeonClient,
  userId: string,
  maxAttempts: number
): Promise<OtpCode[]> {
  const r = rows(
    await sql`
    UPDATE otp_codes
    SET attempts = attempts + 1
    WHERE user_id = ${userId}
      AND consumed_at IS NULL
      AND expires_at > now()
      AND attempts < ${maxAttempts}
    RETURNING *
  `
  );
  return z.array(OtpCodeSchema).parse(r);
}

export async function incrementOtpAttempts(sql: NeonClient, id: string): Promise<number> {
  const r = rows(
    await sql`
    UPDATE otp_codes SET attempts = attempts + 1
    WHERE id = ${id}
    RETURNING attempts
  `
  );
  return z.coerce.number().parse(r[0]?.attempts);
}

/**
 * Atomically consume an OTP code. Includes expires_at > now() guard
 * to prevent consuming expired codes in a race window.
 */
export async function consumeOtpCode(sql: NeonClient, id: string): Promise<boolean> {
  const r = rows(
    await sql`
    UPDATE otp_codes
    SET consumed_at = now()
    WHERE id = ${id}
      AND consumed_at IS NULL
      AND expires_at > now()
    RETURNING id
  `
  );
  return r.length > 0;
}

// ---------------------------------------------------------------------------
// Refresh tokens
// ---------------------------------------------------------------------------

const RefreshTokenSchema = z.object({
  id: IdSchema,
  user_id: IdSchema,
  token_hash: z.string(),
  family_id: z.string(),
  expires_at: DateStringSchema,
  revoked_at: z.union([DateStringSchema, z.null()]),
  consumed_at: z.union([DateStringSchema, z.null()]),
  created_at: DateStringSchema,
});

export type RefreshToken = z.infer<typeof RefreshTokenSchema>;

export async function insertRefreshToken(
  sql: NeonClient,
  userId: string,
  tokenHash: string,
  familyId: string,
  expiresAt: Date
): Promise<RefreshToken> {
  const r = rows(
    await sql`
    INSERT INTO refresh_tokens (user_id, token_hash, family_id, expires_at)
    VALUES (${userId}, ${tokenHash}, ${familyId}, ${expiresAt.toISOString()})
    RETURNING *
  `
  );
  return RefreshTokenSchema.parse(r[0]);
}

export async function findRefreshTokenByHash(
  sql: NeonClient,
  tokenHash: string
): Promise<RefreshToken | null> {
  const r = rows(
    await sql`
    SELECT * FROM refresh_tokens WHERE token_hash = ${tokenHash} LIMIT 1
  `
  );
  if (!r[0]) return null;
  return RefreshTokenSchema.parse(r[0]);
}

export async function consumeRefreshToken(sql: NeonClient, id: string): Promise<boolean> {
  const r = rows(
    await sql`
    UPDATE refresh_tokens SET consumed_at = now()
    WHERE id = ${id}
      AND consumed_at IS NULL
    RETURNING id
  `
  );
  return r.length > 0;
}

/**
 * Atomically consume a refresh token and insert its replacement in the same
 * family. Used by `/session/refresh` so a concurrent family-revocation (theft
 * detection on a racing request) cannot leave a live replacement token after
 * the consume has already succeeded.
 *
 * Implemented as a single data-modifying CTE statement rather than a multi-
 * statement Neon batch: empty INSERT is not an error, so a batch would still
 * commit a consume-without-insert partial. The CTE only mutates when both
 * the family is clear and the old token is still consumable.
 */
export type RefreshRotateResult = { status: 'rotated'; token: RefreshToken } | { status: 'failed' };

export async function consumeAndRotateRefreshToken(
  sql: NeonClient,
  args: {
    oldTokenId: string;
    userId: string;
    newTokenHash: string;
    familyId: string;
    expiresAt: Date;
  }
): Promise<RefreshRotateResult> {
  const r = rows(
    await sql`
    WITH family_clear AS (
      SELECT 1 AS ok
      WHERE NOT EXISTS (
        SELECT 1 FROM refresh_tokens
        WHERE family_id = ${args.familyId}
          AND revoked_at IS NOT NULL
      )
    ),
    consumed AS (
      UPDATE refresh_tokens rt
      SET consumed_at = now()
      FROM family_clear
      WHERE rt.id = ${args.oldTokenId}
        AND rt.user_id = ${args.userId}
        AND rt.family_id = ${args.familyId}
        AND rt.consumed_at IS NULL
        AND rt.revoked_at IS NULL
      RETURNING rt.id
    ),
    inserted AS (
      INSERT INTO refresh_tokens (user_id, token_hash, family_id, expires_at)
      SELECT ${args.userId}, ${args.newTokenHash}, ${args.familyId}, ${args.expiresAt.toISOString()}
      FROM consumed
      RETURNING *
    )
    SELECT * FROM inserted
  `
  );
  if (!r[0]) return { status: 'failed' };
  return { status: 'rotated', token: RefreshTokenSchema.parse(r[0]) };
}

export async function revokeRefreshTokenFamily(sql: NeonClient, familyId: string): Promise<number> {
  const r = rows(
    await sql`
    UPDATE refresh_tokens SET revoked_at = now()
    WHERE family_id = ${familyId}
      AND revoked_at IS NULL
    RETURNING id
  `
  );
  return r.length;
}

export async function revokeRefreshFamilyAndAccessTokensForUser(
  sql: NeonClient,
  familyId: string,
  userId: string
): Promise<{ refreshTokensRevoked: number; accessTokensRevoked: number }> {
  const txResult = await sql.transaction([
    sql`UPDATE refresh_tokens SET revoked_at = now()
        WHERE family_id = ${familyId}
          AND revoked_at IS NULL
        RETURNING id`,
    sql`UPDATE access_tokens SET revoked_at = now()
        WHERE user_id = ${userId}
          AND revoked_at IS NULL
        RETURNING id`,
  ]);
  const refreshRows = (txResult as unknown[][])[0] ?? [];
  const accessRows = (txResult as unknown[][])[1] ?? [];
  return { refreshTokensRevoked: refreshRows.length, accessTokensRevoked: accessRows.length };
}

// ---------------------------------------------------------------------------
// Extended queries (centralised from route files)
// ---------------------------------------------------------------------------

export async function findUserById(sql: NeonClient, id: string): Promise<BetaUser | null> {
  const r = rows(await sql`SELECT * FROM beta_users WHERE id = ${id} LIMIT 1`);
  if (!r[0]) return null;
  return BetaUserSchema.parse(r[0]);
}

/**
 * Insert a device code with no user_id (anti-enumeration dummy row).
 * The null user_id means the row is never backed by an active user, and
 * consumeDeviceCode's `confirmed_at IS NOT NULL` guard makes it unmintable —
 * it exists purely so /poll behaves identically for unknown emails (F-C-003).
 */
export async function insertDummyDeviceCode(
  sql: NeonClient,
  userCode: string,
  pollTokenHash: string,
  expiresAt: Date
): Promise<void> {
  await sql`
    INSERT INTO device_codes (user_code, poll_token, expires_at)
    VALUES (${userCode}, ${pollTokenHash}, ${expiresAt.toISOString()})
  `;
}

/**
 * Atomic per-token rate-limited poll update. Returns the device code row
 * if the cooldown has elapsed, or null if rate-limited or not found.
 */
export async function pollDeviceCode(
  sql: NeonClient,
  pollTokenHash: string,
  intervalSeconds: number
): Promise<DeviceCode | null> {
  const r = rows(
    await sql`
    UPDATE device_codes
    SET last_polled_at = now()
    WHERE poll_token = ${pollTokenHash}
      AND expires_at > now()
      AND (last_polled_at IS NULL OR last_polled_at <= now() - make_interval(secs => ${intervalSeconds}))
    RETURNING *
  `
  );
  if (!r[0]) return null;
  return DeviceCodeSchema.parse(r[0]);
}

/**
 * Check whether an unexpired device code exists for the given poll token.
 * Read-only — used to distinguish "not found" from "rate limited".
 */
export async function deviceCodeExistsByPollToken(
  sql: NeonClient,
  pollTokenHash: string
): Promise<boolean> {
  const r = rows(
    await sql`
    SELECT id FROM device_codes
    WHERE poll_token = ${pollTokenHash}
      AND expires_at > now()
    LIMIT 1
  `
  );
  return r.length > 0;
}

/**
 * Atomically consume a confirmed device code (DELETE ... RETURNING).
 * Ensures concurrent polls cannot both mint sessions.
 */
export async function consumeDeviceCode(
  sql: NeonClient,
  pollTokenHash: string
): Promise<{ user_id: string } | null> {
  const r = rows(
    await sql`
    DELETE FROM device_codes
    WHERE poll_token = ${pollTokenHash}
      AND confirmed_at IS NOT NULL
    RETURNING user_id
  `
  );
  if (!r[0]) return null;
  return { user_id: String(r[0].user_id) };
}

/**
 * Insert a new user with status 'pending'. Uses ON CONFLICT DO NOTHING
 * to handle concurrent signups. Returns the inserted id or null if
 * the email already existed.
 */
export async function insertPendingUser(
  sql: NeonClient,
  email: string,
  name: string | null,
  notes: string | null
): Promise<string | null> {
  const r = rows(
    await sql`
    INSERT INTO beta_users (email, name, status, notes)
    VALUES (${email}, ${name}, 'pending', ${notes})
    ON CONFLICT (email) DO NOTHING
    RETURNING id
  `
  );
  if (!r[0]) return null;
  return String(r[0].id);
}

/** Look up a beta user by their linked GitHub numeric id (authoritative). */
export async function findUserByGitHubId(
  sql: NeonClient,
  githubId: number
): Promise<BetaUser | null> {
  const r = rows(
    await sql`
    SELECT * FROM beta_users WHERE github_id = ${githubId} LIMIT 1
  `
  );
  if (!r[0]) return null;
  return BetaUserSchema.parse(r[0]);
}

/**
 * Find an **active** beta user whose email matches ANY of the supplied
 * (already verified, lowercased) GitHub emails. Used to first-link a GitHub
 * identity to a pre-existing email-invited record. Restricted to `active` so a
 * pending/suspended row is never auto-linked, and so an active invite is never
 * shadowed by a freshly created pending duplicate (GHCLIAUTH-003 / ADR-066).
 *
 * May return a row that is already linked (non-null `github_id`). The caller
 * (`linkOrCreateGitHubUser`) inspects `github_id` and **rejects** a match whose
 * row is already bound to a different account rather than re-binding it —
 * `github_id` is authoritative once linked (ADR-066 decision 4). This is the
 * fail-closed guard for the email-moved-between-GitHub-accounts vector.
 */
export async function findActiveUserByAnyEmail(
  sql: NeonClient,
  emails: string[]
): Promise<BetaUser | null> {
  if (emails.length === 0) return null;
  const r = rows(
    await sql`
    SELECT * FROM beta_users
    WHERE status = 'active' AND email = ANY(${emails})
    ORDER BY created_at ASC
    LIMIT 1
  `
  );
  if (!r[0]) return null;
  return BetaUserSchema.parse(r[0]);
}

/**
 * First-link the GitHub numeric id onto a row that is **not yet linked**.
 * Guarded `WHERE github_id IS NULL` so an already-linked row is never re-bound
 * (`github_id` is authoritative once set — ADR-066 decision 4) and so concurrent
 * first-links race safely (only one UPDATE matches). Returns `null` when the row
 * was already linked (0 rows updated), letting the caller fail closed.
 */
export async function linkGitHubIdToUser(
  sql: NeonClient,
  userId: string,
  githubId: number
): Promise<BetaUser | null> {
  const r = rows(
    await sql`
    UPDATE beta_users
    SET github_id = ${githubId}, updated_at = now()
    WHERE id = ${userId} AND github_id IS NULL
    RETURNING *
  `
  );
  if (!r[0]) return null;
  return BetaUserSchema.parse(r[0]);
}

// Canonical definition lives with the fetch that produces it; re-exported so
// linking callers and tests share one nominal type.
export type { GitHubIdentity } from '../lib/github-user.js';

/**
 * A verified GitHub email resolved to an **active row already linked to a
 * different `github_id`** (e.g. the email was removed from one GitHub account
 * and re-verified on another). `github_id` is authoritative once linked
 * (ADR-066 decision 4), so the email no longer controls binding and the login
 * is rejected fail-closed rather than re-binding or minting that account.
 */
export class GitHubAccountLinkConflictError extends Error {
  constructor() {
    super('github identity conflicts with an existing linked account');
    this.name = 'GitHubAccountLinkConflictError';
  }
}

/**
 * Resolve the beta_users row for a GitHub identity, linking or creating as
 * needed. Precedence (GHCLIAUTH-003 / ADR-066 decision 4):
 *   1. Match on `github_id` (authoritative for returning users).
 *   2. Else first-link: match an **active** invited row by ANY verified email,
 *      store `github_id`, and return it.
 *   3. Else create (or surface an existing non-active) `pending` row keyed on
 *      the primary email — the caller's active-status gate then rejects it.
 *
 * The caller owns the active-status gate, audit logging, and session mint; this
 * helper only resolves identity → row so `/auth/github/callback` and the
 * device-flow poll path (GHCLIAUTH-005) share one linking implementation.
 */
export async function linkOrCreateGitHubUser(
  sql: NeonClient,
  ghUser: GitHubIdentity
): Promise<{ user: BetaUser; isNewPending: boolean; didFirstLink: boolean }> {
  const byId = await findUserByGitHubId(sql, ghUser.id);
  if (byId) return { user: byId, isNewPending: false, didFirstLink: false };

  const byEmail = await findActiveUserByAnyEmail(sql, ghUser.verifiedEmails);
  if (byEmail) {
    if (byEmail.github_id != null) {
      // The matched active row is already linked. `findUserByGitHubId` above
      // would have returned it if it were THIS account, so a non-null
      // `github_id` here means a *different* GitHub account is presenting a
      // verified email that now resolves to someone else's linked row.
      // `github_id` is authoritative once linked (ADR-066 decision 4) — never
      // re-bind, never mint. Fail closed.
      throw new GitHubAccountLinkConflictError();
    }
    // First-link: bind this GitHub id to a pre-existing, unlinked active invite.
    // This is the one moment ADR-066's accepted "verified-email control ==
    // account control" residual risk materialises, so the caller audit-logs it.
    const linked = await linkGitHubIdToUser(sql, byEmail.id, ghUser.id);
    if (linked) return { user: linked, isNewPending: false, didFirstLink: true };
    // Lost a concurrent first-link race (the row was linked between the SELECT
    // and the guarded UPDATE). Re-resolve by github_id and accept it only if it
    // bound to THIS account; otherwise fail closed.
    const now = await findUserByGitHubId(sql, ghUser.id);
    if (now) return { user: now, isNewPending: false, didFirstLink: false };
    throw new GitHubAccountLinkConflictError();
  }

  // No github_id match and no active invite: create a pending row, or surface an
  // existing (non-active) row with the same email for the caller to reject.
  const insertedId = await insertPendingUser(
    sql,
    ghUser.email,
    ghUser.login,
    `GitHub OAuth signup (github:${ghUser.id})`
  );
  const user = insertedId
    ? await findUserById(sql, insertedId)
    : await findUserByEmail(sql, ghUser.email);
  if (!user) {
    throw new Error('failed to create or resolve beta user for GitHub identity');
  }
  return { user, isNewPending: insertedId !== null, didFirstLink: false };
}

/**
 * Count active (unconsumed, unexpired) OTP codes for a user.
 * Used for rate-limiting OTP requests.
 */
export async function countActiveOtpCodes(sql: NeonClient, userId: string): Promise<number> {
  const r = rows(
    await sql`
    SELECT COUNT(*)::int AS count FROM otp_codes
    WHERE user_id = ${userId}
      AND consumed_at IS NULL
      AND expires_at > now()
  `
  );
  return z.coerce.number().parse(r[0]?.count ?? 0);
}

/**
 * Insert an OTP code only when the user is still under `maxActive` active
 * (unconsumed, unexpired) codes. Serialises concurrent requestors for the same
 * user with a transaction-scoped advisory lock so two racing `/auth/otp/request`
 * calls cannot both observe a sub-cap count and both insert.
 *
 * Returns the inserted row, or `null` when the cap is already reached (caller
 * should still return the anti-enumeration success shape).
 */
export async function insertOtpCodeIfUnderLimit(
  sql: NeonClient,
  userId: string,
  codeHash: string,
  expiresAt: Date,
  maxActive: number
): Promise<OtpCode | null> {
  const txResult = await sql.transaction([
    // hashtext yields int4; advisory locks are session/xact scoped. Pairing the
    // lock with the conditional INSERT in one Neon batch makes the cap check
    // race-free for a given user_id under concurrent HTTP requests.
    sql`SELECT pg_advisory_xact_lock(hashtext(${userId}))`,
    sql`
      INSERT INTO otp_codes (user_id, code_hash, expires_at)
      SELECT ${userId}, ${codeHash}, ${expiresAt.toISOString()}
      WHERE (
        SELECT COUNT(*)::int FROM otp_codes
        WHERE user_id = ${userId}
          AND consumed_at IS NULL
          AND expires_at > now()
      ) < ${maxActive}
      RETURNING *
    `,
  ]);
  const insertRows = (txResult as unknown[][])[1] ?? [];
  if (!insertRows[0]) return null;
  return OtpCodeSchema.parse(insertRows[0]);
}

// ---------------------------------------------------------------------------
// Waitlist
// ---------------------------------------------------------------------------

const WaitlistEntrySchema = z.object({
  id: IdSchema,
  email: z.string(),
  created_at: DateStringSchema,
  is_new: z.coerce.boolean(),
});

export type WaitlistEntry = z.infer<typeof WaitlistEntrySchema>;

/**
 * Upsert a waitlist entry. Uses Postgres xmax=0 trick to detect
 * whether the row was newly inserted or already existed.
 */
export async function upsertWaitlistEntry(
  sql: NeonClient,
  email: string,
  source: string = 'website'
): Promise<WaitlistEntry> {
  const r = rows(
    await sql`
    INSERT INTO waitlist (email, source)
    VALUES (${email}, ${source})
    ON CONFLICT (email) DO UPDATE SET updated_at = NOW()
    RETURNING id, email, created_at, (xmax = 0) AS is_new
  `
  );
  return WaitlistEntrySchema.parse(r[0]);
}

/**
 * Upsert a waitlist entry with optional name (used by admin invite).
 */
export async function upsertWaitlistWithName(
  sql: NeonClient,
  email: string,
  name: string | null,
  source: string = 'manual'
): Promise<void> {
  await sql`
    INSERT INTO waitlist (email, name, source)
    VALUES (${email}, ${name}, ${source})
    ON CONFLICT (email) DO UPDATE SET
      name = COALESCE(${name}, waitlist.name),
      updated_at = NOW()
  `;
}

export async function findWaitlistEntryByEmail(
  sql: NeonClient,
  email: string
): Promise<{ id: string } | null> {
  const r = rows(await sql`SELECT id FROM waitlist WHERE email = ${email}`);
  if (!r[0]) return null;
  return { id: String(r[0].id) };
}

export async function findUnapprovedWaitlistEntries(
  sql: NeonClient,
  limit: number
): Promise<{ email: string }[]> {
  const r = rows(
    await sql`
    SELECT w.email FROM waitlist w
    WHERE w.approved_at IS NULL
    ORDER BY w.created_at ASC
    LIMIT ${limit}
  `
  );
  return r.map((row) => ({ email: String(row.email) }));
}

export async function findWaitlistBySource(
  sql: NeonClient,
  source: string,
  limit: number
): Promise<{ email: string; name: string | null }[]> {
  const r = rows(
    await sql`
    SELECT email, name FROM waitlist WHERE source = ${source}
    ORDER BY created_at ASC
    LIMIT ${limit}
  `
  );
  return r.map((row) => ({
    email: String(row.email),
    name: row.name as string | null,
  }));
}

// ---------------------------------------------------------------------------
// Broadcast snapshots (EMAIL-002 — generalisation of the ADMINCLIH-001
// send-migration snapshot table). `template`, `template_props`,
// `audience_key`, and `audience_params` were added so /admin/broadcast can
// store the full preview state alongside the recipient set. The
// /admin/send-migration handler keeps working as a back-compat shim
// (EMAIL-006) that supplies waitlist-migration values for the new
// columns.
// ---------------------------------------------------------------------------

export interface SnapshotRecipient {
  email: string;
  name: string | null;
}

export interface BroadcastSnapshot {
  // Raw token on `insertBroadcastSnapshot`'s return (so the route can
  // return it to the operator); SHA-256 hash on `findBroadcastSnapshot` /
  // `consumeBroadcastSnapshot` returns (whatever the DB stored). The
  // route doesn't read `token` post-consume, so this asymmetry is safe;
  // any future code that needs the raw token after consume cannot
  // recover it from the DB and must thread it through from the
  // request body.
  token: string;
  template: string;
  template_props: Record<string, unknown>;
  audience_key: string;
  audience_params: Record<string, string>;
  recipients: SnapshotRecipient[];
  created_by_actor: string;
  created_at: string;
  expires_at: string;
  consumed_at: string | null;
}

const SnapshotRecipientsSchema = z.array(
  z.object({ email: z.string(), name: z.union([z.string(), z.null()]) })
);

function parseJsonField(raw: unknown): unknown {
  // Neon returns JSONB as either an object or a string depending on driver
  // version; normalise to the JS shape we wrote in.
  return typeof raw === 'string' ? JSON.parse(raw) : raw;
}

// Belt-and-braces: even though insertBroadcastSnapshot only writes
// Record<string, string> for audience_params, parse-on-read enforces
// the type contract from the DB side too, so a future caller that
// bypasses the route schema and writes non-string values gets caught
// at consume time rather than blowing up the SQL parameter binding
// downstream in resolveAudience.
const AudienceParamsSchema = z.record(z.string(), z.string());

function parseSnapshotRow(row: Record<string, unknown>): BroadcastSnapshot {
  const recipients = SnapshotRecipientsSchema.parse(parseJsonField(row['recipients']));
  const templateProps = parseJsonField(row['template_props']) as Record<string, unknown>;
  const audienceParams = AudienceParamsSchema.parse(parseJsonField(row['audience_params']));
  const createdAt = row['created_at'];
  const expiresAt = row['expires_at'];
  const consumedAt = row['consumed_at'];
  return {
    // Returned value is the hashed key from the DB on find/consume.
    // insertBroadcastSnapshot overrides this with the raw token before
    // returning to its caller.
    token: String(row['token_hash']),
    template: String(row['template']),
    template_props: templateProps,
    audience_key: String(row['audience_key']),
    audience_params: audienceParams,
    recipients,
    created_by_actor: String(row['created_by_actor']),
    created_at: createdAt instanceof Date ? createdAt.toISOString() : String(createdAt),
    expires_at: expiresAt instanceof Date ? expiresAt.toISOString() : String(expiresAt),
    consumed_at:
      consumedAt === null || consumedAt === undefined
        ? null
        : consumedAt instanceof Date
          ? consumedAt.toISOString()
          : String(consumedAt),
  };
}

export async function insertBroadcastSnapshot(
  sql: NeonClient,
  params: {
    token: string;
    template: string;
    templateProps: Record<string, unknown>;
    audienceKey: string;
    // Tightened from Record<string, unknown>: the route schema only
    // ever validates Record<string, string>, the resolver only reads
    // strings, and parseSnapshotRow enforces string values on read.
    // Making this match the runtime contract closes a defensive gap
    // where a future caller could write non-string values.
    audienceParams: Record<string, string>;
    recipients: SnapshotRecipient[];
    createdByActor: string;
    ttlSeconds: number;
  }
): Promise<BroadcastSnapshot> {
  // Lazy reap: any snapshot past a day old is uninteresting and exists
  // only to consume index space. Best-effort — a failure here must not
  // block the primary insert. The row count is logged so on-call can
  // baseline snapshot-table growth from the function logs.
  try {
    const reapResult = await sql`
      DELETE FROM send_broadcast_snapshots
      WHERE expires_at < now() - interval '1 day'
      RETURNING token_hash
    `;
    const rowsDeleted = Array.isArray(reapResult) ? reapResult.length : 0;
    if (rowsDeleted > 0) {
      // console.warn rather than console.log: the project's lint rule
      // restricts console use to warn/error. Reap activity is operational
      // signal worth surfacing even though it's not an error condition.
      console.warn('send_broadcast_snapshots reap deleted rows', { rowsDeleted });
    }
  } catch (err) {
    // Swallow: reap is opportunistic. The row will get reaped by a
    // later successful sweep.
    const message = err instanceof Error ? err.message : String(err);
    console.error('send_broadcast_snapshots reap failed:', message);
  }

  // SHA-256 the bearer token before storage. Raw token is returned only
  // once, from this function. Mirrors access_tokens / refresh_tokens.
  const tokenHash = hashToken(params.token);

  const r = rows(
    await sql`
    INSERT INTO send_broadcast_snapshots
      (token_hash, template, template_props, audience_key, audience_params,
       recipients, created_by_actor, expires_at)
    VALUES (
      ${tokenHash},
      ${params.template},
      ${JSON.stringify(params.templateProps)}::jsonb,
      ${params.audienceKey},
      ${JSON.stringify(params.audienceParams)}::jsonb,
      ${JSON.stringify(params.recipients)}::jsonb,
      ${params.createdByActor},
      now() + make_interval(secs => ${params.ttlSeconds})
    )
    RETURNING *
  `
  );
  if (!r[0]) {
    throw new Error('insertBroadcastSnapshot: INSERT returned no row');
  }
  // Override the parsed `token` (which is the hash) with the raw token
  // so the caller can return it to the operator. Any future code that
  // needs the raw post-consume must thread it through from the
  // request — the DB no longer holds it.
  return { ...parseSnapshotRow(r[0]), token: params.token };
}

/**
 * Look up a snapshot by (token, actor) WITHOUT consuming it. Used to
 * disambiguate expired vs consumed after an atomic-consume attempt
 * fails. The actor filter is intentional: a caller with the wrong
 * actor is indistinguishable from a caller with a wrong token, so
 * the failure surface is uniform (`preview_token_missing`) and the
 * server never confirms a token's existence to a non-owner.
 */
/**
 * Wrap parseSnapshotRow so callers can distinguish 'no such row' (null)
 * from 'row found but malformed' (logs + null). A malformed row — e.g.
 * non-string audience_params from a manual DB write — would otherwise
 * propagate a zod error all the way up to a generic 500. Logging here
 * gives on-call a signal; returning null lets the handler classify the
 * caller as `preview_token_missing` rather than crashing.
 */
function safeParseSnapshotRow(row: Record<string, unknown>): BroadcastSnapshot | null {
  try {
    return parseSnapshotRow(row);
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    console.error('send_broadcast_snapshots row failed to parse', {
      token_hash: row['token_hash'],
      message,
    });
    return null;
  }
}

export async function findBroadcastSnapshot(
  sql: NeonClient,
  params: { token: string; actor: string }
): Promise<BroadcastSnapshot | null> {
  const tokenHash = hashToken(params.token);
  const r = rows(
    await sql`
    SELECT * FROM send_broadcast_snapshots
    WHERE token_hash = ${tokenHash}
      AND created_by_actor = ${params.actor}
    LIMIT 1
  `
  );
  if (!r[0]) return null;
  return safeParseSnapshotRow(r[0]);
}

/**
 * Atomically consume a snapshot: set consumed_at iff the token exists,
 * belongs to the caller, has not been consumed, and has not expired.
 * Returns the snapshot row on success; returns null on any failure so
 * the caller can refetch (see findBroadcastSnapshot) to produce a
 * specific error code.
 */
export async function consumeBroadcastSnapshot(
  sql: NeonClient,
  params: { token: string; actor: string }
): Promise<BroadcastSnapshot | null> {
  const tokenHash = hashToken(params.token);
  const r = rows(
    await sql`
    UPDATE send_broadcast_snapshots
    SET consumed_at = now()
    WHERE token_hash = ${tokenHash}
      AND created_by_actor = ${params.actor}
      AND consumed_at IS NULL
      AND expires_at > now()
    RETURNING *
  `
  );
  if (!r[0]) return null;
  return safeParseSnapshotRow(r[0]);
}

export type WaitlistStatus = 'pending' | 'approved' | 'all';
export type WaitlistSource = 'manual' | 'website' | 'import' | 'all';

export interface WaitlistListFilters {
  status: WaitlistStatus;
  source: WaitlistSource;
  limit: number;
  offset: number;
}

export interface WaitlistListEntry {
  email: string;
  name: string | null;
  source: string;
  created_at: string;
  approved_at: string | null;
}

/**
 * Paginated waitlist listing for the admin CLI.
 *
 * Approval is the durable `waitlist.approved_at` column (set by admin
 * approve / invite). Pending = NULL; approved = NOT NULL. This is not
 * "beta_users row exists" — pending GitHub OAuth signups stay pending
 * until an operator grants access.
 *
 * Total and items are computed from a single query via `COUNT(*) OVER ()`
 * so pagination metadata matches the page contents under concurrent writes.
 */
export async function findWaitlistPaginated(
  sql: NeonClient,
  filters: WaitlistListFilters
): Promise<{ total: number; items: WaitlistListEntry[] }> {
  const { status, source, limit, offset } = filters;

  // Build status/source predicates as SQL fragments so the driver
  // keeps parameter binding. `sql` is a tagged template; literal
  // SQL fragments are composed via nested template calls.
  const statusPred =
    status === 'pending'
      ? sql`AND w.approved_at IS NULL`
      : status === 'approved'
        ? sql`AND w.approved_at IS NOT NULL`
        : sql``;
  const sourcePred = source === 'all' ? sql`` : sql`AND w.source = ${source}`;

  const result = rows(
    await sql`
      SELECT
        w.email,
        w.name,
        w.source,
        w.created_at,
        w.approved_at,
        COUNT(*) OVER () AS total
      FROM waitlist w
      WHERE 1=1 ${statusPred} ${sourcePred}
      ORDER BY w.created_at ASC
      LIMIT ${limit} OFFSET ${offset}
    `
  );

  const totalRaw = result[0]?.total;
  const total = typeof totalRaw === 'number' ? totalRaw : Number(totalRaw ?? 0);

  return {
    total,
    items: result.map((row) => ({
      email: String(row.email),
      name: row.name as string | null,
      source: String(row.source),
      created_at:
        row.created_at instanceof Date ? row.created_at.toISOString() : String(row.created_at),
      approved_at:
        row.approved_at == null
          ? null
          : row.approved_at instanceof Date
            ? row.approved_at.toISOString()
            : String(row.approved_at),
    })),
  };
}

export interface AuditListFilters {
  action?: string | undefined;
  actor?: string | undefined;
  limit: number;
  offset: number;
}

/**
 * Paginated audit log listing, most recent first. Optional action
 * and actor filters apply exact-match equality. Total and items are
 * computed from a single query via `COUNT(*) OVER ()` so pagination
 * metadata matches the page contents under concurrent writes.
 */
export async function findAuditEntries(
  sql: NeonClient,
  filters: AuditListFilters
): Promise<{ total: number; items: AuditEntry[] }> {
  const { action, actor, limit, offset } = filters;

  const actionPred = action == null ? sql`` : sql`AND action = ${action}`;
  const actorPred = actor == null ? sql`` : sql`AND actor = ${actor}`;

  const r = rows(
    await sql`
      SELECT id, action, actor, metadata, created_at,
             COUNT(*) OVER () AS total
      FROM audit_log
      WHERE 1=1 ${actionPred} ${actorPred}
      ORDER BY created_at DESC
      LIMIT ${limit} OFFSET ${offset}
    `
  );

  const totalRaw = r[0]?.total;
  const total = typeof totalRaw === 'number' ? totalRaw : Number(totalRaw ?? 0);

  return {
    total,
    items: z.array(AuditEntrySchema).parse(r.map(({ total: _t, ...rest }) => rest)),
  };
}

/**
 * Recent audit entries relating to a user, capped at 10, most recent first.
 *
 * Matches rows where either `metadata->>'email'` (admin-authored events
 * like user.invited, tokens.revoked, migration.email.sent) or `actor`
 * (github OAuth events which store the email as the actor) equals the
 * lookup email. Uses `LOWER()` on the metadata expression so historical
 * mixed-case rows still match the lowercased input.
 */
export async function findRecentAuditForEmail(
  sql: NeonClient,
  email: string
): Promise<AuditEntry[]> {
  const r = rows(
    await sql`
      SELECT id, action, actor, metadata, created_at
      FROM audit_log
      WHERE LOWER(metadata->>'email') = ${email}
         OR actor = ${email}
      ORDER BY created_at DESC
      LIMIT 10
    `
  );
  return z.array(AuditEntrySchema).parse(r);
}

// ---------------------------------------------------------------------------
// Cleanup (cron)
// ---------------------------------------------------------------------------

/** Delete device codes expired more than 1 hour ago. */
export async function cleanupExpiredDeviceCodes(sql: NeonClient): Promise<number> {
  const r = rows(
    await sql`
    DELETE FROM device_codes
    WHERE expires_at < now() - interval '1 hour'
    RETURNING id
  `
  );
  return r.length;
}

/** Delete GitHub device-flow sessions expired more than 1 hour ago. */
export async function cleanupExpiredGithubDeviceSessions(sql: NeonClient): Promise<number> {
  const r = rows(
    await sql`
    DELETE FROM github_device_sessions
    WHERE expires_at < now() - interval '1 hour'
    RETURNING id
  `
  );
  return r.length;
}

/** Delete OTP codes expired more than 1 hour ago. */
export async function cleanupExpiredOtpCodes(sql: NeonClient): Promise<number> {
  const r = rows(
    await sql`
    DELETE FROM otp_codes
    WHERE expires_at < now() - interval '1 hour'
    RETURNING id
  `
  );
  return r.length;
}

/**
 * Delete refresh tokens that are expired (1-hour grace) or
 * revoked more than 7 days ago (audit retention).
 */
export async function cleanupExpiredRefreshTokens(sql: NeonClient): Promise<number> {
  const r = rows(
    await sql`
    DELETE FROM refresh_tokens
    WHERE (expires_at < now() - interval '1 hour')
       OR (revoked_at IS NOT NULL AND revoked_at < now() - interval '7 days')
    RETURNING id
  `
  );
  return r.length;
}

/**
 * Delete broadcast snapshots expired more than 1 hour ago. The
 * lazy reap inside `insertBroadcastSnapshot` only fires on insert;
 * at low broadcast cadence rows would accumulate indefinitely without
 * a periodic sweep. The cron handler invokes this alongside the
 * other expired-state cleanups.
 */
export async function cleanupExpiredBroadcastSnapshots(sql: NeonClient): Promise<number> {
  const r = rows(
    await sql`
    DELETE FROM send_broadcast_snapshots
    WHERE expires_at < now() - interval '1 hour'
    RETURNING token_hash
  `
  );
  return r.length;
}

// ---------------------------------------------------------------------------
// Fleet telemetry (FLEET-005, ADR-107)
// ---------------------------------------------------------------------------

/**
 * A validated schema-version-1 beacon, exactly the ADR-107 §3 allowlist.
 * Defined here (not imported from the route schema) so the storage layer
 * carries its own contract: nothing outside these fields can ever reach an
 * insert. Structurally satisfied by `TelemetryBeacon` from
 * `routes/telemetry-schemas.ts`.
 */
export interface TelemetryBeaconRecord {
  schema_version: number;
  install_id: string;
  version: string;
  install_method: string;
  platform: string;
  channel: string;
  flag_snapshot_version: string;
  features: ReadonlyArray<{ key: string; count: number }>;
}

/**
 * Store one beacon. Privacy invariants (ADR-107 §3), by construction:
 *
 *   - The caller passes ONLY the validated allowlist record — this function
 *     never sees the request, so no IP (or any header) can reach a row; the
 *     table has no ip column to receive one anyway.
 *   - No timestamp is supplied: `received_on` is a DATE column defaulting to
 *     `current_date`, so arrival time coarsens to a date at the row.
 *
 * The beacon row and its feature-usage rows land in ONE statement (a
 * data-modifying CTE), so a partial write cannot strand usage counts
 * without their beacon or vice versa.
 */
export async function insertTelemetryBeacon(
  sql: NeonClient,
  beacon: TelemetryBeaconRecord
): Promise<void> {
  await sql`
    WITH beacon AS (
      INSERT INTO telemetry_beacons
        (schema_version, install_id, version, install_method, platform, channel, flag_snapshot_version)
      VALUES
        (${beacon.schema_version}, ${beacon.install_id}, ${beacon.version},
         ${beacon.install_method}, ${beacon.platform}, ${beacon.channel},
         ${beacon.flag_snapshot_version})
      RETURNING id
    )
    INSERT INTO telemetry_beacon_features (beacon_id, feature_key, usage_count)
    SELECT beacon.id, f.key, f.count
    FROM beacon,
         jsonb_to_recordset(${JSON.stringify(beacon.features)}::jsonb)
           AS f(key text, count int)
  `;
}

/**
 * Retention sweep (ADR-107 §6): roll raw beacon rows older than the
 * retention window up into the kept-indefinitely daily aggregate tables,
 * then delete them. Runs from the hourly cron cleanup.
 *
 * All three statements share one transaction, so a failed rollup never
 * loses raw rows. The rollup recomputes each expired day's groups from the
 * still-present raw rows and overwrites on conflict, which makes a re-run
 * after a failed delete idempotent (no double counting).
 *
 * `retentionDays` is configuration (see lib/telemetry-retention.ts) and is
 * bound as a parameter — the window is never a literal in the SQL.
 */
export async function rollupAndPurgeExpiredTelemetryBeacons(
  sql: NeonClient,
  retentionDays: number
): Promise<number> {
  validateTelemetryRetentionDays(retentionDays);
  const txResult = await sql.transaction([
    sql`INSERT INTO telemetry_daily_installs
          (day, version, install_method, platform, channel, install_count)
        SELECT received_on, version, install_method, platform, channel,
               COUNT(DISTINCT install_id)
        FROM telemetry_beacons
        WHERE received_on < current_date - (${retentionDays}::int - 1)
        GROUP BY received_on, version, install_method, platform, channel
        ON CONFLICT (day, version, install_method, platform, channel)
        DO UPDATE SET install_count = EXCLUDED.install_count`,
    sql`INSERT INTO telemetry_daily_feature_usage
          (day, feature_key, usage_count, install_count)
        SELECT b.received_on, f.feature_key, SUM(f.usage_count),
               COUNT(DISTINCT b.install_id)
        FROM telemetry_beacon_features f
        JOIN telemetry_beacons b ON b.id = f.beacon_id
        WHERE b.received_on < current_date - (${retentionDays}::int - 1)
        GROUP BY b.received_on, f.feature_key
        ON CONFLICT (day, feature_key)
        DO UPDATE SET usage_count = EXCLUDED.usage_count,
                      install_count = EXCLUDED.install_count`,
    sql`DELETE FROM telemetry_beacons
        WHERE received_on < current_date - (${retentionDays}::int - 1)
        RETURNING id`,
  ]);
  const purged = (txResult as unknown[][])[2] ?? [];
  return purged.length;
}

// ---------------------------------------------------------------------------
// Admin keys (ADMINCLIH-002)
// ---------------------------------------------------------------------------

const AdminKeySchema = z.object({
  id: IdSchema,
  hashed_key: z.string(),
  actor_email: z.string(),
  note: z.union([z.string(), z.null()]),
  created_at: DateStringSchema,
  revoked_at: z.union([DateStringSchema, z.null()]),
});

export type AdminKey = z.infer<typeof AdminKeySchema>;

/**
 * Look up an admin key by its hashed form. The caller hashes the presented
 * bearer (HMAC-SHA-256 keyed by the server pepper) and passes the result
 * here. Returns the row regardless of revocation status so the middleware
 * can distinguish "unknown" from "revoked" for the audit trail.
 */
export async function findAdminKeyByHash(
  sql: NeonClient,
  hashedKey: string
): Promise<AdminKey | null> {
  const r = rows(
    await sql`
    SELECT * FROM admin_keys WHERE hashed_key = ${hashedKey} LIMIT 1
  `
  );
  if (!r[0]) return null;
  return AdminKeySchema.parse(r[0]);
}
