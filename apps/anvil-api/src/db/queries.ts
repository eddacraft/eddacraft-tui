import type { NeonClient } from './client.js';

export interface BetaUser {
  id: string;
  email: string;
  name: string | null;
  status: string;
  notes: string | null;
  created_at: string;
  updated_at: string;
}

export interface AccessToken {
  id: string;
  user_id: string;
  token_hash: string;
  scopes: string[];
  expires_at: string;
  revoked_at: string | null;
  created_at: string;
}

export interface AuditEntry {
  id: string;
  action: string;
  actor: string;
  metadata: Record<string, unknown>;
  created_at: string;
}

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
  return (r[0] as BetaUser) ?? null;
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
  return r[0] as BetaUser;
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
  return r[0] as AccessToken;
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
  return (r[0] as AccessToken & { email: string; user_status: string }) ?? null;
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
  return { user, tokens: r as AccessToken[] };
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
  return r[0] as AuditEntry;
}
