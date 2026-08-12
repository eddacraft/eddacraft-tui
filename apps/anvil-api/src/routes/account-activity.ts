import { Hono } from 'hono';
import { z } from 'zod';
import { getClient } from '../db/client.js';
import { stampUserActivity, upsertAccountFeatureTouch } from '../db/queries.js';
import { verifyLicence } from '../lib/licence.js';
import {
  ACCOUNT_FEATURE_KEYS,
  isAccountFeatureKey,
  type AccountFeatureKey,
} from '../lib/account-activity.js';

/**
 * BACT-005: authenticated account feature-touch ingest.
 *
 * Requires a valid licence JWT (same identity as product session). Payload is
 * a closed allowlist of feature keys only — never free-form argv/paths.
 * Failures on the client are expected to be fire-and-forget; this route still
 * returns clear 4xx/5xx for operators and tests.
 */
export const accountActivity = new Hono();

const activityBodySchema = z.strictObject({
  features: z.array(z.string().min(1).max(64)).min(1).max(ACCOUNT_FEATURE_KEYS.length),
});

accountActivity.post('/', async (c) => {
  const authHeader = c.req.header('authorization') ?? '';
  const match = authHeader.match(/^Bearer\s+(.+)$/i);
  if (!match?.[1]) {
    return c.json({ error: 'Authentication required' }, 401);
  }

  let claims;
  try {
    claims = await verifyLicence(match[1]);
  } catch (err) {
    console.error('account activity licence verify misconfigured:', err);
    return c.json({ error: 'Service unavailable' }, 503);
  }
  if (!claims) {
    return c.json({ error: 'Invalid or expired licence' }, 401);
  }

  const contentType = c.req.header('content-type') ?? '';
  if (!contentType.toLowerCase().includes('application/json')) {
    return c.json({ error: 'Content-Type must be application/json' }, 400);
  }

  let body: unknown;
  try {
    body = await c.req.json();
  } catch {
    return c.json({ error: 'Invalid JSON payload' }, 400);
  }

  const parsed = activityBodySchema.safeParse(body);
  if (!parsed.success) {
    return c.json({ error: 'Invalid activity payload' }, 400);
  }

  const accepted: AccountFeatureKey[] = [];
  const rejected: string[] = [];
  for (const key of parsed.data.features) {
    if (isAccountFeatureKey(key)) {
      if (!accepted.includes(key)) accepted.push(key);
    } else {
      rejected.push(key);
    }
  }

  // Unknown keys fail closed (reject request) so clients cannot invent analytics.
  if (rejected.length > 0) {
    return c.json(
      {
        error: 'Unknown feature keys',
        rejected,
        allowed: [...ACCOUNT_FEATURE_KEYS],
      },
      400
    );
  }

  if (accepted.length === 0) {
    return c.json({ error: 'No feature keys provided' }, 400);
  }

  const sql = getClient();
  try {
    for (const key of accepted) {
      await upsertAccountFeatureTouch(sql, claims.sub, key);
    }
  } catch (err) {
    console.error('account activity upsert failed:', err);
    return c.json({ error: 'Failed to record activity' }, 500);
  }

  // BACT-008 / ADR-121 decision 4: an accepted feature-touch is account
  // activity — advance once per request regardless of how many allowlisted
  // keys were accepted, never touching login stamps. Best-effort: this runs
  // after the touches above already committed, so a stamp failure must not
  // 500 — a client retry on a 500 would re-upsert the same touches and
  // inflate touch_count.
  try {
    await stampUserActivity(sql, claims.sub, 'feature');
  } catch (err) {
    console.error('account activity stamp failed (non-fatal):', err);
  }

  return c.json({ accepted: true, features: accepted }, 202);
});
