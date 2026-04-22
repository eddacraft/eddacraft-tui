import { afterAll, afterEach, beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';
import { generateKeyPair, exportPKCS8 } from 'jose';
import { Hono } from 'hono';
import { authSession } from '../routes/auth-session.js';

vi.mock('../db/client.js', () => ({
  getClient: vi.fn(() => vi.fn()),
}));

vi.mock('../db/queries.js', () => ({
  findRefreshTokenByHash: vi.fn(),
  consumeRefreshToken: vi.fn(),
  revokeRefreshTokenFamily: vi.fn(),
  insertRefreshToken: vi.fn(),
  findUserById: vi.fn(),
}));

vi.mock('../lib/token.js', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../lib/token.js')>();
  return {
    ...actual,
    hashToken: vi.fn((input: string) => `hash:${input}`),
  };
});

import {
  consumeRefreshToken,
  findRefreshTokenByHash,
  findUserById,
  insertRefreshToken,
  revokeRefreshTokenFamily,
  type RefreshToken,
} from '../db/queries.js';

const app = new Hono();
app.route('/auth/session', authSession);

let originalSigningKey: string | undefined;

beforeAll(async () => {
  originalSigningKey = process.env['LICENSE_SIGNING_KEY'];
  const { privateKey } = await generateKeyPair('ES256', { extractable: true });
  process.env['LICENSE_SIGNING_KEY'] = await exportPKCS8(privateKey);
});

afterAll(() => {
  if (originalSigningKey === undefined) delete process.env['LICENSE_SIGNING_KEY'];
  else process.env['LICENSE_SIGNING_KEY'] = originalSigningKey;
});

beforeEach(() => {
  vi.resetAllMocks();
  vi.mocked(consumeRefreshToken).mockResolvedValue(true);
  vi.mocked(revokeRefreshTokenFamily).mockResolvedValue(0);
  vi.mocked(insertRefreshToken).mockResolvedValue(undefined as never);
});

afterEach(() => {
  vi.restoreAllMocks();
});

function makeToken(overrides: Partial<RefreshToken> = {}): RefreshToken {
  return {
    id: 'rt-1',
    user_id: 'user-1',
    token_hash: 'hash:raw-token',
    family_id: 'family-1',
    expires_at: new Date(Date.now() + 60_000).toISOString(),
    revoked_at: null,
    consumed_at: null,
    created_at: new Date().toISOString(),
    ...overrides,
  };
}

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

function post(body: unknown) {
  return app.request('/auth/session/refresh', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
  });
}

describe('POST /auth/session/refresh', () => {
  it('rotates the token and issues a new licence on the happy path', async () => {
    vi.mocked(findRefreshTokenByHash).mockResolvedValue(makeToken());
    vi.mocked(findUserById).mockResolvedValue(activeUser());

    const res = await post({ refreshToken: 'raw-token' });

    expect(res.status).toBe(200);
    const body = await res.json();
    expect(typeof body.license).toBe('string');
    expect(body.license.split('.').length).toBe(3);
    expect(typeof body.refreshToken).toBe('string');
    expect(body.refreshToken.length).toBeGreaterThanOrEqual(32);
    expect(body.refreshToken).not.toBe('raw-token');
    expect(body.expiresAt).toMatch(/^\d{4}-\d{2}-\d{2}T/);

    expect(vi.mocked(consumeRefreshToken)).toHaveBeenCalledWith(expect.anything(), 'rt-1');
    expect(vi.mocked(insertRefreshToken)).toHaveBeenCalledWith(
      expect.anything(),
      'user-1',
      expect.stringMatching(/^hash:[0-9a-f]{64}$/),
      'family-1', // new token carries the same family_id
      expect.any(Date)
    );
    expect(vi.mocked(revokeRefreshTokenFamily)).not.toHaveBeenCalled();
  });

  it('hashes the inbound refresh token before lookup', async () => {
    vi.mocked(findRefreshTokenByHash).mockResolvedValue(null);

    await post({ refreshToken: 'some-random-token' });

    expect(vi.mocked(findRefreshTokenByHash)).toHaveBeenCalledWith(
      expect.anything(),
      'hash:some-random-token'
    );
  });

  it('returns 401 when the refresh token is unknown', async () => {
    vi.mocked(findRefreshTokenByHash).mockResolvedValue(null);

    const res = await post({ refreshToken: 'ghost-token' });

    expect(res.status).toBe(401);
    expect(await res.json()).toEqual({ error: 'Invalid refresh token' });
    expect(vi.mocked(insertRefreshToken)).not.toHaveBeenCalled();
  });

  it('revokes the entire family when a consumed token is reused', async () => {
    vi.mocked(findRefreshTokenByHash).mockResolvedValue(
      makeToken({ consumed_at: new Date().toISOString(), family_id: 'family-compromised' })
    );

    const res = await post({ refreshToken: 'raw-token' });

    expect(res.status).toBe(401);
    expect(await res.json()).toEqual({ error: 'Token reuse detected' });
    expect(vi.mocked(revokeRefreshTokenFamily)).toHaveBeenCalledWith(
      expect.anything(),
      'family-compromised'
    );
    expect(vi.mocked(insertRefreshToken)).not.toHaveBeenCalled();
    // Must not call consumeRefreshToken — reuse path short-circuits before it.
    expect(vi.mocked(consumeRefreshToken)).not.toHaveBeenCalled();
  });

  it('returns 401 without revoking the family when the token is revoked', async () => {
    vi.mocked(findRefreshTokenByHash).mockResolvedValue(
      makeToken({ revoked_at: new Date().toISOString() })
    );

    const res = await post({ refreshToken: 'raw-token' });

    expect(res.status).toBe(401);
    expect(await res.json()).toEqual({ error: 'Invalid refresh token' });
    expect(vi.mocked(revokeRefreshTokenFamily)).not.toHaveBeenCalled();
    expect(vi.mocked(consumeRefreshToken)).not.toHaveBeenCalled();
  });

  it('returns 401 with the expired error when the token is past its TTL', async () => {
    vi.mocked(findRefreshTokenByHash).mockResolvedValue(
      makeToken({ expires_at: new Date(Date.now() - 1000).toISOString() })
    );

    const res = await post({ refreshToken: 'raw-token' });

    expect(res.status).toBe(401);
    expect(await res.json()).toEqual({ error: 'Refresh token expired' });
    expect(vi.mocked(consumeRefreshToken)).not.toHaveBeenCalled();
  });

  it('returns 401 when the associated user is not active', async () => {
    vi.mocked(findRefreshTokenByHash).mockResolvedValue(makeToken());
    vi.mocked(findUserById).mockResolvedValue(activeUser({ status: 'suspended' }));

    const res = await post({ refreshToken: 'raw-token' });

    expect(res.status).toBe(401);
    expect(await res.json()).toEqual({ error: 'User account is not active' });
    expect(vi.mocked(consumeRefreshToken)).not.toHaveBeenCalled();
  });

  it('returns 401 when the user record has been deleted', async () => {
    vi.mocked(findRefreshTokenByHash).mockResolvedValue(makeToken());
    vi.mocked(findUserById).mockResolvedValue(null);

    const res = await post({ refreshToken: 'raw-token' });

    expect(res.status).toBe(401);
    expect(await res.json()).toEqual({ error: 'User account is not active' });
    expect(vi.mocked(consumeRefreshToken)).not.toHaveBeenCalled();
  });

  it('revokes the family when the atomic consume loses a race', async () => {
    vi.mocked(findRefreshTokenByHash).mockResolvedValue(makeToken({ family_id: 'family-race' }));
    vi.mocked(findUserById).mockResolvedValue(activeUser());
    vi.mocked(consumeRefreshToken).mockResolvedValue(false);

    const res = await post({ refreshToken: 'raw-token' });

    expect(res.status).toBe(401);
    expect(await res.json()).toEqual({ error: 'Token reuse detected' });
    expect(vi.mocked(revokeRefreshTokenFamily)).toHaveBeenCalledWith(
      expect.anything(),
      'family-race'
    );
    expect(vi.mocked(insertRefreshToken)).not.toHaveBeenCalled();
  });

  it.each([
    { name: 'missing refreshToken', body: {} },
    { name: 'empty refreshToken', body: { refreshToken: '' } },
    { name: 'over-length refreshToken', body: { refreshToken: 'x'.repeat(201) } },
  ])('returns 400 for $name via Zod', async ({ body }) => {
    const res = await post(body);
    expect(res.status).toBe(400);
    expect(vi.mocked(findRefreshTokenByHash)).not.toHaveBeenCalled();
  });
});
