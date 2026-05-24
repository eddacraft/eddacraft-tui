import { z } from 'zod';
import type { NeonClient } from './client.js';
import { hashToken } from '../lib/token.js';

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

export async function confirmDeviceCode(sql: NeonClient, id: string): Promise<boolean> {
  const r = rows(
    await sql`
    UPDATE device_codes SET confirmed_at = now()
    WHERE id = ${id}
      AND confirmed_at IS NULL
    RETURNING id
  `
  );
  return r.length > 0;
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

export async function findActiveOtpCodes(sql: NeonClient, userId: string): Promise<OtpCode[]> {
  const r = rows(
    await sql`
    SELECT * FROM otp_codes
    WHERE user_id = ${userId}
      AND consumed_at IS NULL
      AND expires_at > now()
    ORDER BY created_at DESC
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
 * The null user_id ensures JOINs in /confirm never match.
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
 * Find a pending (unconfirmed, unexpired) device code and its bound user_id.
 * Used by the /confirm endpoint to verify the code belongs to the
 * authenticated caller. `attempts` is returned so the caller can enforce
 * the per-code brute-force lockout (see auth-device.ts MAX_ATTEMPTS).
 *
 * Returns `user_id` from the device_codes row directly rather than joining
 * to beta_users.email — the consumer asserts the row's user_id matches the
 * caller's authenticated identity. Anti-enumeration dummy rows inserted by
 * /start for inactive users carry `user_id IS NULL`: this query will
 * **still return them when the user_code matches**, but the consuming
 * route's `user_id === null || user_id !== authed.sub` check ensures they
 * cannot be confirmed under any authenticated identity. The check belongs
 * in the route layer because the same lookup feeds the attempts-counter
 * increment that bounds brute-force guessing — a dummy-row match should
 * still cost the attacker an attempt rather than silently disappearing.
 */
export async function findPendingDeviceCodeWithUserId(
  sql: NeonClient,
  userCode: string
): Promise<{ id: string; user_id: string | null; attempts: number } | null> {
  const r = rows(
    await sql`
    SELECT dc.id, dc.attempts, dc.user_id
    FROM device_codes dc
    WHERE dc.user_code = ${userCode}
      AND dc.expires_at > now()
      AND dc.confirmed_at IS NULL
    LIMIT 1
  `
  );
  if (!r[0]) return null;
  return {
    id: String(r[0].id),
    user_id: r[0].user_id == null ? null : String(r[0].user_id),
    attempts: z.coerce.number().parse(r[0].attempts),
  };
}

/**
 * Atomically increment the attempts counter on a device_code row, bounded
 * by `max`. Returns the new counter value, or `null` if the row was already
 * at or above the cap and no UPDATE fired. The `WHERE attempts < ${max}`
 * guard is what bounds the counter under concurrent /device/confirm bursts —
 * without it, parallel requests can all pass the route-level pre-check
 * (read attempts < max, decide to increment) and drive the counter
 * arbitrarily above the intended cap. With it, the DB enforces the ceiling
 * regardless of how many parallel callers race to increment.
 */
export async function incrementDeviceCodeAttempts(
  sql: NeonClient,
  id: string,
  max: number
): Promise<number | null> {
  const r = rows(
    await sql`
    UPDATE device_codes SET attempts = attempts + 1
    WHERE id = ${id} AND attempts < ${max}
    RETURNING attempts
  `
  );
  if (!r[0]) return null;
  return z.coerce.number().parse(r[0].attempts);
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
 * Increment attempt counters on multiple OTP codes at once.
 * Used when an incorrect code is submitted — all active codes get incremented.
 */
export async function incrementOtpAttemptsBatch(sql: NeonClient, ids: string[]): Promise<void> {
  if (ids.length === 0) return;
  await sql`
    UPDATE otp_codes SET attempts = attempts + 1
    WHERE id = ANY(${ids})
  `;
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
    LEFT JOIN beta_users bu ON bu.email = w.email
    WHERE bu.id IS NULL
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
      console.log('send_broadcast_snapshots reap deleted rows', { rowsDeleted });
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
 * Approval is derived by LEFT JOIN against beta_users — an entry is
 * "approved" when a matching beta_users row exists. The join uses
 * citext equality so email casing is handled by the column type.
 *
 * `approved_at` is a proxy for the approval event: it is
 * `beta_users.created_at`, not an audit-log timestamp. A row directly
 * inserted into beta_users (outside the waitlist flow) will still
 * surface an `approved_at` in this listing.
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
      ? sql`AND bu.id IS NULL`
      : status === 'approved'
        ? sql`AND bu.id IS NOT NULL`
        : sql``;
  const sourcePred = source === 'all' ? sql`` : sql`AND w.source = ${source}`;

  const result = rows(
    await sql`
      SELECT
        w.email,
        w.name,
        w.source,
        w.created_at,
        bu.created_at AS approved_at,
        COUNT(*) OVER () AS total
      FROM waitlist w
      LEFT JOIN beta_users bu ON bu.email = w.email
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
