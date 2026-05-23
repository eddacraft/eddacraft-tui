import { z } from 'zod';
import type { NeonClient } from '../db/client.js';

export const AUDIENCE_KEYS = [
  'beta:active',
  'beta:active-recent',
  'beta:active-idle',
  'waitlist:pending',
  'waitlist:source',
  'waitlist:approved-no-token',
] as const;

export type AudienceKey = (typeof AUDIENCE_KEYS)[number];

export interface AudienceRow {
  email: string;
  name: string | null;
  user_id: string | null;
}

export interface AudienceParams {
  limit: number;
  params?: Record<string, string>;
}

export const RECENT_ACTIVITY_DAYS = 30;

const AudienceRowSchema = z.object({
  email: z.string(),
  name: z.string().nullable(),
  user_id: z
    .union([z.string(), z.number()])
    .nullable()
    .transform((v) => (v == null ? null : String(v))),
});

function rows(result: unknown): AudienceRow[] {
  return z.array(AudienceRowSchema).parse(result);
}

export async function resolveAudience(
  sql: NeonClient,
  key: AudienceKey,
  opts: AudienceParams
): Promise<AudienceRow[]> {
  switch (key) {
    case 'beta:active':
      return resolveBetaActive(sql, opts);
    case 'beta:active-recent':
      return resolveBetaActiveRecent(sql, opts);
    case 'beta:active-idle':
      return resolveBetaActiveIdle(sql, opts);
    case 'waitlist:pending':
      return resolveWaitlistPending(sql, opts);
    case 'waitlist:source':
      return resolveWaitlistSource(sql, opts);
    case 'waitlist:approved-no-token':
      return resolveWaitlistApprovedNoToken(sql, opts);
  }
}

// status = 'active' across every beta_users resolver enforces the hard
// exclusion of suspended/banned (and pending) users. The Phase 4
// suppressions table will plug in here as a LEFT JOIN once it lands.

async function resolveBetaActive(
  sql: NeonClient,
  { limit }: AudienceParams
): Promise<AudienceRow[]> {
  const r = await sql`
    SELECT bu.email, bu.name, bu.id AS user_id
    FROM beta_users bu
    WHERE bu.status = 'active'
    ORDER BY bu.created_at ASC
    LIMIT ${limit}
  `;
  return rows(r);
}

async function resolveBetaActiveRecent(
  sql: NeonClient,
  { limit }: AudienceParams
): Promise<AudienceRow[]> {
  const r = await sql`
    SELECT DISTINCT bu.email, bu.name, bu.id AS user_id
    FROM beta_users bu
    JOIN refresh_tokens rt ON rt.user_id = bu.id
    WHERE bu.status = 'active'
      AND rt.revoked_at IS NULL
      AND rt.created_at > now() - (${RECENT_ACTIVITY_DAYS}::int * INTERVAL '1 day')
    ORDER BY bu.created_at ASC
    LIMIT ${limit}
  `;
  return rows(r);
}

async function resolveBetaActiveIdle(
  sql: NeonClient,
  { limit }: AudienceParams
): Promise<AudienceRow[]> {
  const r = await sql`
    SELECT bu.email, bu.name, bu.id AS user_id
    FROM beta_users bu
    WHERE bu.status = 'active'
      AND NOT EXISTS (
        SELECT 1 FROM refresh_tokens rt
        WHERE rt.user_id = bu.id
          AND rt.revoked_at IS NULL
          AND rt.created_at > now() - (${RECENT_ACTIVITY_DAYS}::int * INTERVAL '1 day')
      )
    ORDER BY bu.created_at ASC
    LIMIT ${limit}
  `;
  return rows(r);
}

async function resolveWaitlistPending(
  sql: NeonClient,
  { limit }: AudienceParams
): Promise<AudienceRow[]> {
  const r = await sql`
    SELECT w.email, w.name, NULL AS user_id
    FROM waitlist w
    LEFT JOIN beta_users bu ON bu.email = w.email
    WHERE bu.id IS NULL
    ORDER BY w.created_at ASC
    LIMIT ${limit}
  `;
  return rows(r);
}

async function resolveWaitlistSource(
  sql: NeonClient,
  { limit, params }: AudienceParams
): Promise<AudienceRow[]> {
  const source = params?.source;
  if (!source) {
    throw new Error('audience waitlist:source requires params.source');
  }
  const r = await sql`
    SELECT w.email, w.name, NULL AS user_id
    FROM waitlist w
    LEFT JOIN beta_users bu ON bu.email = w.email
    WHERE w.source = ${source}
      AND bu.id IS NULL
    ORDER BY w.created_at ASC
    LIMIT ${limit}
  `;
  return rows(r);
}

async function resolveWaitlistApprovedNoToken(
  sql: NeonClient,
  { limit }: AudienceParams
): Promise<AudienceRow[]> {
  const r = await sql`
    SELECT bu.email, bu.name, bu.id AS user_id
    FROM beta_users bu
    WHERE bu.status = 'active'
      AND NOT EXISTS (
        SELECT 1 FROM access_tokens at
        WHERE at.user_id = bu.id
          AND at.revoked_at IS NULL
          AND at.expires_at > now()
      )
    ORDER BY bu.created_at ASC
    LIMIT ${limit}
  `;
  return rows(r);
}
