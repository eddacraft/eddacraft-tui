import { afterAll, afterEach, beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';
import { generateKeyPair, exportPKCS8, exportSPKI } from 'jose';
import { Hono } from 'hono';
import { authDevice } from '../routes/auth-device.js';
import { USER_CODE_CONSTRAINT } from '../lib/device-code.js';
import { _resetSigningKeyCacheForTests } from '../lib/licence.js';

vi.mock('../db/client.js', () => ({
  getClient: vi.fn(() => vi.fn()),
}));

vi.mock('../db/queries.js', () => ({
  findUserByEmail: vi.fn(),
  findUserById: vi.fn(),
  insertDeviceCode: vi.fn(),
  insertDummyDeviceCode: vi.fn(),
  pollDeviceCode: vi.fn(),
  deviceCodeExistsByPollToken: vi.fn(),
  consumeDeviceCode: vi.fn(),
  insertRefreshToken: vi.fn(),
  findActiveScopesForUser: vi.fn(),
}));

vi.mock('../lib/token.js', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../lib/token.js')>();
  return {
    ...actual,
    hashToken: vi.fn((input: string) => `hash:${input}`),
  };
});

import {
  consumeDeviceCode,
  deviceCodeExistsByPollToken,
  findUserByEmail,
  findUserById,
  insertDeviceCode,
  insertDummyDeviceCode,
  insertRefreshToken,
  pollDeviceCode,
  findActiveScopesForUser,
} from '../db/queries.js';

const app = new Hono();
app.route('/auth/device', authDevice);

let originalSigningKey: string | undefined;
let originalPublicKey: string | undefined;
const ORIGINAL_ACTIVATE_URL = process.env['ACTIVATE_URL'];

beforeAll(async () => {
  originalSigningKey = process.env['LICENSE_SIGNING_KEY'];
  originalPublicKey = process.env['LICENSE_PUBLIC_KEY'];
  const { privateKey, publicKey } = await generateKeyPair('ES256', { extractable: true });
  process.env['LICENSE_SIGNING_KEY'] = await exportPKCS8(privateKey);
  process.env['LICENSE_PUBLIC_KEY'] = await exportSPKI(publicKey);
  _resetSigningKeyCacheForTests();
});

afterAll(() => {
  if (originalSigningKey === undefined) delete process.env['LICENSE_SIGNING_KEY'];
  else process.env['LICENSE_SIGNING_KEY'] = originalSigningKey;
  if (originalPublicKey === undefined) delete process.env['LICENSE_PUBLIC_KEY'];
  else process.env['LICENSE_PUBLIC_KEY'] = originalPublicKey;
  _resetSigningKeyCacheForTests();
});

beforeEach(() => {
  vi.resetAllMocks();
  // Restore default mock implementations after reset.
  vi.mocked(insertDeviceCode).mockResolvedValue(undefined as never);
  vi.mocked(insertDummyDeviceCode).mockResolvedValue(undefined);
  vi.mocked(pollDeviceCode).mockResolvedValue(null);
  vi.mocked(deviceCodeExistsByPollToken).mockResolvedValue(false);
  vi.mocked(consumeDeviceCode).mockResolvedValue(null);
  vi.mocked(insertRefreshToken).mockResolvedValue(undefined as never);
  // Default to the conservative `['beta']` fallback so existing tests
  // don't have to know about the new scope-lookup call.
  vi.mocked(findActiveScopesForUser).mockResolvedValue(['beta']);
  delete process.env['ACTIVATE_URL'];
});

afterEach(() => {
  vi.restoreAllMocks();
  if (ORIGINAL_ACTIVATE_URL === undefined) delete process.env['ACTIVATE_URL'];
  else process.env['ACTIVATE_URL'] = ORIGINAL_ACTIVATE_URL;
});

type UserRow = {
  id: string;
  email: string;
  name: string | null;
  status: string;
  notes: string | null;
  created_at: string;
  updated_at: string;
};

function activeUser(overrides: Partial<UserRow> = {}): UserRow {
  return {
    id: 'user-1',
    email: 'active@example.com',
    name: 'Octocat',
    status: 'active',
    notes: null,
    created_at: new Date().toISOString(),
    updated_at: new Date().toISOString(),
    ...overrides,
  };
}

function makeDeviceCodeRow(overrides: Record<string, unknown> = {}) {
  return {
    id: 'dc-1',
    user_id: 'user-1',
    user_code: 'ANVIL-DEADBEEF',
    poll_token: 'hash:poll-token',
    confirmed_at: null,
    expires_at: new Date(Date.now() + 900_000).toISOString(),
    last_polled_at: null,
    created_at: new Date().toISOString(),
    ...overrides,
  };
}

function post(path: string, body: unknown, extraHeaders: Record<string, string> = {}) {
  return app.request(path, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', ...extraHeaders },
    body: JSON.stringify(body),
  });
}

describe('POST /auth/device/start', () => {
  it('creates a real device code for an active user and returns the full response shape', async () => {
    vi.mocked(findUserByEmail).mockResolvedValue(activeUser());

    const res = await post('/auth/device/start', { email: 'active@example.com' });

    expect(res.status).toBe(200);
    const body = await res.json();
    expect(body.userCode).toMatch(/^ANVIL-[0-9A-F]{8}$/);
    expect(typeof body.pollToken).toBe('string');
    expect(body.pollToken.length).toBeGreaterThanOrEqual(32);
    expect(body.expiresIn).toBe(900);
    expect(body.interval).toBe(5);
    expect(body.verificationUrl).toBe('https://eddacraft.ai/auth/activate');

    expect(vi.mocked(insertDeviceCode)).toHaveBeenCalledTimes(1);
    expect(vi.mocked(insertDummyDeviceCode)).not.toHaveBeenCalled();
    const [, userId, userCode, pollTokenHash] = vi.mocked(insertDeviceCode).mock.calls[0]!;
    expect(userId).toBe('user-1');
    expect(userCode).toBe(body.userCode);
    expect(pollTokenHash).toBe(`hash:${body.pollToken}`);
  });

  it('returns a response shape indistinguishable from the active-user path for unknown emails', async () => {
    // First call: known active user — record the canonical shape.
    vi.mocked(findUserByEmail).mockResolvedValueOnce(activeUser());
    const knownRes = await post('/auth/device/start', { email: 'active@example.com' });
    const knownBody = await knownRes.json();
    const knownKeys = Object.keys(knownBody).sort();

    // Second call: unknown email — falls through to dummy row.
    vi.mocked(findUserByEmail).mockResolvedValueOnce(null);
    const unknownRes = await post('/auth/device/start', { email: 'nobody@example.com' });
    const unknownBody = await unknownRes.json();

    expect(unknownRes.status).toBe(knownRes.status);
    // Identical key sets — a regression that adds or drops a field on one
    // path would reintroduce the user-enumeration side channel.
    expect(Object.keys(unknownBody).sort()).toEqual(knownKeys);
    // Fixed-shape fields must match byte-for-byte.
    expect(unknownBody.expiresIn).toBe(knownBody.expiresIn);
    expect(unknownBody.interval).toBe(knownBody.interval);
    expect(unknownBody.verificationUrl).toBe(knownBody.verificationUrl);
    // Variable-shape fields must still have matching shapes.
    expect(unknownBody.userCode).toMatch(/^ANVIL-[0-9A-F]{8}$/);
    expect(typeof unknownBody.pollToken).toBe('string');
    expect(unknownBody.pollToken.length).toBe(knownBody.pollToken.length);

    expect(vi.mocked(insertDummyDeviceCode)).toHaveBeenCalledTimes(1);
    expect(vi.mocked(insertDeviceCode)).toHaveBeenCalledTimes(1); // only the known-user call
  });

  it('falls through to a dummy row for a suspended user', async () => {
    vi.mocked(findUserByEmail).mockResolvedValue(activeUser({ status: 'suspended' }));

    const res = await post('/auth/device/start', { email: 'susp@example.com' });

    expect(res.status).toBe(200);
    expect(vi.mocked(insertDummyDeviceCode)).toHaveBeenCalledTimes(1);
    expect(vi.mocked(insertDeviceCode)).not.toHaveBeenCalled();
  });

  it('retries user_code generation on a unique-constraint collision', async () => {
    vi.mocked(findUserByEmail).mockResolvedValue(activeUser());

    const collision = Object.assign(new Error('unique_violation'), {
      code: '23505',
      constraint: USER_CODE_CONSTRAINT,
    });
    vi.mocked(insertDeviceCode)
      .mockRejectedValueOnce(collision)
      .mockResolvedValueOnce(undefined as never);

    const res = await post('/auth/device/start', { email: 'active@example.com' });

    expect(res.status).toBe(200);
    expect(vi.mocked(insertDeviceCode)).toHaveBeenCalledTimes(2);
    // Each attempt generates a fresh user_code, so the two calls should not collide.
    const firstCode = vi.mocked(insertDeviceCode).mock.calls[0]![2];
    const secondCode = vi.mocked(insertDeviceCode).mock.calls[1]![2];
    expect(firstCode).not.toBe(secondCode);
  });

  it('propagates non-collision DB errors instead of retrying', async () => {
    vi.mocked(findUserByEmail).mockResolvedValue(activeUser());
    vi.mocked(insertDeviceCode).mockRejectedValue(new Error('db down'));

    const res = await post('/auth/device/start', { email: 'active@example.com' });

    expect(res.status).toBe(500);
    expect(vi.mocked(insertDeviceCode)).toHaveBeenCalledTimes(1);
  });

  it('honours ACTIVATE_URL when set', async () => {
    vi.mocked(findUserByEmail).mockResolvedValue(activeUser());
    process.env['ACTIVATE_URL'] = 'https://custom.example/activate';

    const res = await post('/auth/device/start', { email: 'active@example.com' });

    expect(res.status).toBe(200);
    expect((await res.json()).verificationUrl).toBe('https://custom.example/activate');
  });

  it('rejects missing or malformed email via Zod without touching the DB', async () => {
    const bad = await post('/auth/device/start', { email: 'not-an-email' });
    expect(bad.status).toBe(400);
    const missing = await post('/auth/device/start', {});
    expect(missing.status).toBe(400);
    expect(vi.mocked(findUserByEmail)).not.toHaveBeenCalled();
  });
});

describe('POST /auth/device/poll', () => {
  it('returns status:expired when the poll token has no row at all', async () => {
    vi.mocked(pollDeviceCode).mockResolvedValue(null);
    vi.mocked(deviceCodeExistsByPollToken).mockResolvedValue(false);

    const res = await post('/auth/device/poll', { pollToken: 'missing-token' });

    expect(res.status).toBe(200);
    expect(await res.json()).toEqual({ status: 'expired' });
  });

  it('returns 429 slow_down when a row exists but the cooldown is active', async () => {
    vi.mocked(pollDeviceCode).mockResolvedValue(null);
    vi.mocked(deviceCodeExistsByPollToken).mockResolvedValue(true);

    const res = await post('/auth/device/poll', { pollToken: 'hot-token' });

    expect(res.status).toBe(429);
    expect(await res.json()).toEqual({ error: 'slow_down', retryAfter: 5 });
  });

  it('returns status:pending when the code is unconfirmed', async () => {
    vi.mocked(pollDeviceCode).mockResolvedValue(makeDeviceCodeRow({ confirmed_at: null }));

    const res = await post('/auth/device/poll', { pollToken: 'active-token' });

    expect(res.status).toBe(200);
    expect(await res.json()).toEqual({ status: 'pending' });
    expect(vi.mocked(consumeDeviceCode)).not.toHaveBeenCalled();
  });

  it('returns status:expired when expires_at is in the past', async () => {
    vi.mocked(pollDeviceCode).mockResolvedValue(
      makeDeviceCodeRow({ expires_at: new Date(Date.now() - 1000).toISOString() })
    );

    const res = await post('/auth/device/poll', { pollToken: 'stale-token' });

    expect(res.status).toBe(200);
    expect(await res.json()).toEqual({ status: 'expired' });
    expect(vi.mocked(consumeDeviceCode)).not.toHaveBeenCalled();
  });

  it('returns status:expired when the confirmed code was already consumed by a sibling', async () => {
    vi.mocked(pollDeviceCode).mockResolvedValue(
      makeDeviceCodeRow({ confirmed_at: new Date().toISOString() })
    );
    vi.mocked(consumeDeviceCode).mockResolvedValue(null);

    const res = await post('/auth/device/poll', { pollToken: 'racing-token' });

    expect(res.status).toBe(200);
    expect(await res.json()).toEqual({ status: 'expired' });
    expect(vi.mocked(insertRefreshToken)).not.toHaveBeenCalled();
  });

  it('returns status:expired when the confirmed user has been suspended', async () => {
    vi.mocked(pollDeviceCode).mockResolvedValue(
      makeDeviceCodeRow({ confirmed_at: new Date().toISOString() })
    );
    vi.mocked(consumeDeviceCode).mockResolvedValue({ user_id: 'user-1' });
    vi.mocked(findUserById).mockResolvedValue(activeUser({ status: 'suspended' }));

    const res = await post('/auth/device/poll', { pollToken: 'confirmed-token' });

    expect(res.status).toBe(200);
    expect(await res.json()).toEqual({ status: 'expired' });
    expect(vi.mocked(insertRefreshToken)).not.toHaveBeenCalled();
  });

  it('mints a licence and refresh token on the happy confirmed path', async () => {
    vi.mocked(pollDeviceCode).mockResolvedValue(
      makeDeviceCodeRow({ confirmed_at: new Date().toISOString() })
    );
    vi.mocked(consumeDeviceCode).mockResolvedValue({ user_id: 'user-1' });
    vi.mocked(findUserById).mockResolvedValue(activeUser());

    const res = await post('/auth/device/poll', { pollToken: 'good-token' });

    expect(res.status).toBe(200);
    const body = await res.json();
    expect(body.status).toBe('confirmed');
    expect(typeof body.license).toBe('string');
    expect(body.license.split('.').length).toBe(3);
    expect(typeof body.refreshToken).toBe('string');
    expect(body.refreshToken.length).toBeGreaterThanOrEqual(32);
    expect(body.expiresAt).toMatch(/^\d{4}-\d{2}-\d{2}T/);

    expect(vi.mocked(insertRefreshToken)).toHaveBeenCalledWith(
      expect.anything(),
      'user-1',
      expect.stringMatching(/^hash:[0-9a-f]{64}$/),
      expect.stringMatching(/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i),
      expect.any(Date)
    );
  });

  it('hashes the inbound poll token before every DB call', async () => {
    vi.mocked(pollDeviceCode).mockResolvedValue(null);
    vi.mocked(deviceCodeExistsByPollToken).mockResolvedValue(false);

    await post('/auth/device/poll', { pollToken: 'raw-token' });

    expect(vi.mocked(pollDeviceCode)).toHaveBeenCalledWith(
      expect.anything(),
      'hash:raw-token',
      expect.any(Number)
    );
    expect(vi.mocked(deviceCodeExistsByPollToken)).toHaveBeenCalledWith(
      expect.anything(),
      'hash:raw-token'
    );
  });

  it('rejects missing or over-length poll tokens via Zod', async () => {
    expect((await post('/auth/device/poll', {})).status).toBe(400);
    expect((await post('/auth/device/poll', { pollToken: '' })).status).toBe(400);
    expect((await post('/auth/device/poll', { pollToken: 'x'.repeat(201) })).status).toBe(400);
    expect(vi.mocked(pollDeviceCode)).not.toHaveBeenCalled();
  });
});
