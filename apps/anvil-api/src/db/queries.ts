import { z } from 'zod';
import type { NeonClient } from './client.js';

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
  expires_at: DateStringSchema,
  revoked_at: z.union([DateStringSchema, z.null()]),
  created_at: DateStringSchema,
});

const AuditEntrySchema = z.object({
  id: IdSchema,
  action: z.string(),
  actor: z.string(),
  metadata: z
    .union([z.record(z.string(), z.unknown()), z.null(), z.undefined()])
    .transform((v) => v ?? {}),
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

export async function revokeTokensByEmail(sql: NeonClient, email: string): Promise<number> {
  const r = rows(
    await sql`
    UPDATE access_tokens SET revoked_at = now()
    WHERE user_id = (SELECT id FROM beta_users WHERE email = ${email})
      AND revoked_at IS NULL
    RETURNING id
  `
  );
  return r.length;
}

export async function revokeTokenByHash(sql: NeonClient, tokenHash: string): Promise<boolean> {
  const r = rows(
    await sql`
    UPDATE access_tokens SET revoked_at = now()
    WHERE token_hash = ${tokenHash}
      AND revoked_at IS NULL
    RETURNING id
  `
  );
  return r.length > 0;
}

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
  metadata: Record<string, unknown> = {}
): Promise<AuditEntry> {
  const r = rows(
    await sql`
    INSERT INTO audit_log (action, actor, metadata)
    VALUES (${action}, ${actor}, ${JSON.stringify(metadata)})
    RETURNING *
  `
  );
  return AuditEntrySchema.parse(r[0]);
}
