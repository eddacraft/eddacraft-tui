import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { Hono } from 'hono';
import { accountActivity } from '../routes/account-activity.js';
import { ACCOUNT_FEATURE_KEYS } from '../lib/account-activity.js';

const mocks = vi.hoisted(() => ({
  getClient: vi.fn(),
  sql: vi.fn(),
  verifyLicence: vi.fn(),
  upsertAccountFeatureTouch: vi.fn(),
  stampUserActivity: vi.fn(),
}));

vi.mock('../db/client.js', () => ({
  getClient: mocks.getClient,
}));

vi.mock('../lib/licence.js', () => ({
  verifyLicence: mocks.verifyLicence,
}));

vi.mock('../db/queries.js', () => ({
  upsertAccountFeatureTouch: mocks.upsertAccountFeatureTouch,
  stampUserActivity: mocks.stampUserActivity,
}));

const app = new Hono();
app.route('/account/activity', accountActivity);

function post(body: unknown, auth?: string) {
  return app.request('/account/activity', {
    method: 'POST',
    headers: {
      'content-type': 'application/json',
      ...(auth ? { authorization: auth } : {}),
    },
    body: JSON.stringify(body),
  });
}

describe('POST /account/activity (BACT-005)', () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  beforeEach(() => {
    vi.clearAllMocks();
    mocks.getClient.mockReturnValue(mocks.sql);
    mocks.verifyLicence.mockResolvedValue({
      sub: 'user-1',
      email: 'alice@example.com',
      identity: { provider: 'github', id: '1' },
      org: null,
      tier: 'pro',
      scopes: ['beta'],
      seats: 1,
    });
    mocks.upsertAccountFeatureTouch.mockResolvedValue({
      user_id: 'user-1',
      feature_key: 'watch',
      first_seen_at: '2026-08-12T00:00:00.000Z',
      last_seen_at: '2026-08-12T00:00:00.000Z',
      touch_count: 1,
    });
    mocks.stampUserActivity.mockResolvedValue(undefined);
  });

  it('rejects missing auth', async () => {
    const res = await post({ features: ['watch'] });
    expect(res.status).toBe(401);
    expect(mocks.upsertAccountFeatureTouch).not.toHaveBeenCalled();
  });

  it('rejects invalid licence', async () => {
    mocks.verifyLicence.mockResolvedValue(null);
    const res = await post({ features: ['watch'] }, 'Bearer bad');
    expect(res.status).toBe(401);
  });

  it('accepts allowlisted feature keys', async () => {
    const res = await post({ features: ['watch', 'check'] }, 'Bearer good');
    expect(res.status).toBe(202);
    const body = await res.json();
    expect(body).toEqual({ accepted: true, features: ['watch', 'check'] });
    expect(mocks.upsertAccountFeatureTouch).toHaveBeenCalledTimes(2);
    expect(mocks.upsertAccountFeatureTouch).toHaveBeenCalledWith(mocks.sql, 'user-1', 'watch');
  });

  it('rejects unknown feature keys closed-set', async () => {
    const res = await post({ features: ['watch', 'rm-rf'] }, 'Bearer good');
    expect(res.status).toBe(400);
    const body = await res.json();
    expect(body.error).toBe('Unknown feature keys');
    expect(body.rejected).toEqual(['rm-rf']);
    expect(body.allowed).toEqual([...ACCOUNT_FEATURE_KEYS]);
    expect(mocks.upsertAccountFeatureTouch).not.toHaveBeenCalled();
  });

  it('rejects empty features and non-json', async () => {
    const empty = await post({ features: [] }, 'Bearer good');
    expect(empty.status).toBe(400);

    const noJson = await app.request('/account/activity', {
      method: 'POST',
      headers: {
        'content-type': 'text/plain',
        authorization: 'Bearer good',
      },
      body: 'x',
    });
    expect(noJson.status).toBe(400);
  });

  it('dedupes repeated keys in one request', async () => {
    const res = await post({ features: ['watch', 'watch'] }, 'Bearer good');
    expect(res.status).toBe(202);
    expect(mocks.upsertAccountFeatureTouch).toHaveBeenCalledTimes(1);
  });

  it('advances last_activity_at with kind feature exactly once per accepted request (BACT-008)', async () => {
    const res = await post({ features: ['watch', 'check'] }, 'Bearer good');
    expect(res.status).toBe(202);
    expect(mocks.stampUserActivity).toHaveBeenCalledTimes(1);
    expect(mocks.stampUserActivity).toHaveBeenCalledWith(mocks.sql, 'user-1', 'feature');
  });

  it('does not stamp activity when auth is missing or invalid', async () => {
    await post({ features: ['watch'] });
    expect(mocks.stampUserActivity).not.toHaveBeenCalled();

    mocks.verifyLicence.mockResolvedValue(null);
    await post({ features: ['watch'] }, 'Bearer bad');
    expect(mocks.stampUserActivity).not.toHaveBeenCalled();
  });

  it('does not stamp activity when the feature keys are rejected', async () => {
    const res = await post({ features: ['watch', 'rm-rf'] }, 'Bearer good');
    expect(res.status).toBe(400);
    expect(mocks.stampUserActivity).not.toHaveBeenCalled();
  });

  it('still returns the accepted-touches success response when stampUserActivity rejects (best-effort)', async () => {
    // Regression: the activity stamp running after the touches already
    // upserted must never turn into a 500 the client retries — a retry
    // would re-upsert the same touches and inflate touch_count.
    mocks.stampUserActivity.mockRejectedValue(new Error('activity stamp unavailable'));
    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => undefined);

    const res = await post({ features: ['watch', 'check'] }, 'Bearer good');

    expect(res.status).toBe(202);
    const body = await res.json();
    expect(body).toEqual({ accepted: true, features: ['watch', 'check'] });
    expect(mocks.upsertAccountFeatureTouch).toHaveBeenCalledTimes(2);
    expect(mocks.stampUserActivity).toHaveBeenCalledWith(mocks.sql, 'user-1', 'feature');
    expect(consoleErrorSpy).toHaveBeenCalled();

    consoleErrorSpy.mockRestore();
  });
});
