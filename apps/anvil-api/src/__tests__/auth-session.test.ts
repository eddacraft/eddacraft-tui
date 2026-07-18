import { afterAll, afterEach, beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';
import { generateKeyPair, exportPKCS8 } from 'jose';
import { Hono } from 'hono';
import { authSession } from '../routes/auth-session.js';

vi.mock('../db/client.js', () => ({
  getClient: vi.fn(() => vi.fn()),
}));

vi.mock('../db/queries.js', () => ({
  findRefreshTokenByHash: vi.fn(),
  consumeAndRotateRefreshToken: vi.fn(),
  revokeRefreshFamilyAndAccessTokensForUser: vi.fn(),
  findUserById: vi.fn(),
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
  consumeAndRotateRefreshToken,
  findRefreshTokenByHash,
  findUserById,
  revokeRefreshFamilyAndAccessTokensForUser,
  findActiveScopesForUser,
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
  vi.mocked(consumeAndRotateRefreshToken).mockResolvedValue({
    status: 'rotated',
    token: makeToken({ id: 'rt-2', token_hash: 'hash:new' }),
  });
  vi.mocked(revokeRefreshFamilyAndAccessTokensForUser).mockResolvedValue({
    refreshTokensRevoked: 0,
    accessTokensRevoked: 0,
  });
  // Default to the conservative `['beta']` fallback so existing tests
  // don't have to know about the new scope-lookup call. Tests that care
  // about graded scopes set this explicitly.
  vi.mocked(findActiveScopesForUser).mockResolvedValue(['beta']);
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

    expect(vi.mocked(consumeAndRotateRefreshToken)).toHaveBeenCalledWith(expect.anything(), {
      oldTokenId: 'rt-1',
      userId: 'user-1',
      newTokenHash: expect.stringMatching(/^hash:[0-9a-f]{64}$/),
      familyId: 'family-1',
      expiresAt: expect.any(Date),
    });
    expect(vi.mocked(revokeRefreshFamilyAndAccessTokensForUser)).not.toHaveBeenCalled();
  });

  it('hashes the inbound refresh token before lookup', async () => {
    vi.mocked(findRefreshTokenByHash).mockResolvedValue(null);

    await post({ refreshToken: 'some-random-token' });

    expect(vi.mocked(findRefreshTokenByHash)).toHaveBeenCalledWith(
      expect.anything(),
      'hash:some-random-token'
    );
  });

  it("carries the user's current scopes into the refreshed licence", async () => {
    // Regression: previously `scopes` was hardcoded to `['beta']` in the
    // refresh path, which silently downgraded any user invited with a
    // graded scope (e.g. `preview` via FLAGM-005 `admin invite`) on their
    // first token rotation.
    vi.mocked(findRefreshTokenByHash).mockResolvedValue(makeToken());
    vi.mocked(findUserById).mockResolvedValue(activeUser());
    vi.mocked(findActiveScopesForUser).mockResolvedValue(['preview', 'beta']);

    const res = await post({ refreshToken: 'raw-token' });
    expect(res.status).toBe(200);
    const body = await res.json();
    // The signed licence is opaque without verification, but `decodeJwt`
    // exposes the claims directly — and the scopes claim is the contract
    // the kernel client relies on for entitlement gating.
    const { decodeJwt } = await import('jose');
    const claims = decodeJwt(body.license) as { scopes?: string[] };
    expect(claims.scopes).toEqual(['preview', 'beta']);
    expect(vi.mocked(findActiveScopesForUser)).toHaveBeenCalledWith(expect.anything(), 'user-1');
  });

  it('returns 401 when the refresh token is unknown', async () => {
    vi.mocked(findRefreshTokenByHash).mockResolvedValue(null);

    const res = await post({ refreshToken: 'ghost-token' });

    expect(res.status).toBe(401);
    expect(await res.json()).toEqual({ error: 'Invalid refresh token' });
    expect(vi.mocked(consumeAndRotateRefreshToken)).not.toHaveBeenCalled();
  });

  it('revokes the entire family when a consumed token is reused', async () => {
    vi.mocked(findRefreshTokenByHash).mockResolvedValue(
      makeToken({ consumed_at: new Date().toISOString(), family_id: 'family-compromised' })
    );

    const res = await post({ refreshToken: 'raw-token' });

    expect(res.status).toBe(401);
    expect(await res.json()).toEqual({ error: 'Token reuse detected' });
    expect(vi.mocked(revokeRefreshFamilyAndAccessTokensForUser)).toHaveBeenCalledWith(
      expect.anything(),
      'family-compromised',
      'user-1'
    );
    // Must not call rotate — reuse path short-circuits before it.
    expect(vi.mocked(consumeAndRotateRefreshToken)).not.toHaveBeenCalled();
  });

  it('returns 401 without revoking the family when the token is revoked', async () => {
    vi.mocked(findRefreshTokenByHash).mockResolvedValue(
      makeToken({ revoked_at: new Date().toISOString() })
    );

    const res = await post({ refreshToken: 'raw-token' });

    expect(res.status).toBe(401);
    expect(await res.json()).toEqual({ error: 'Invalid refresh token' });
    expect(vi.mocked(revokeRefreshFamilyAndAccessTokensForUser)).not.toHaveBeenCalled();
    expect(vi.mocked(consumeAndRotateRefreshToken)).not.toHaveBeenCalled();
  });

  it('returns 401 with the expired error when the token is past its TTL', async () => {
    vi.mocked(findRefreshTokenByHash).mockResolvedValue(
      makeToken({ expires_at: new Date(Date.now() - 1000).toISOString() })
    );

    const res = await post({ refreshToken: 'raw-token' });

    expect(res.status).toBe(401);
    expect(await res.json()).toEqual({ error: 'Refresh token expired' });
    expect(vi.mocked(consumeAndRotateRefreshToken)).not.toHaveBeenCalled();
  });

  it('returns 401 when the associated user is not active', async () => {
    vi.mocked(findRefreshTokenByHash).mockResolvedValue(makeToken());
    vi.mocked(findUserById).mockResolvedValue(activeUser({ status: 'suspended' }));

    const res = await post({ refreshToken: 'raw-token' });

    expect(res.status).toBe(401);
    expect(await res.json()).toEqual({ error: 'User account is not active' });
    expect(vi.mocked(consumeAndRotateRefreshToken)).not.toHaveBeenCalled();
  });

  it('returns 401 when the user record has been deleted', async () => {
    vi.mocked(findRefreshTokenByHash).mockResolvedValue(makeToken());
    vi.mocked(findUserById).mockResolvedValue(null);

    const res = await post({ refreshToken: 'raw-token' });

    expect(res.status).toBe(401);
    expect(await res.json()).toEqual({ error: 'User account is not active' });
    expect(vi.mocked(consumeAndRotateRefreshToken)).not.toHaveBeenCalled();
  });

  it('revokes the family when the atomic rotate loses a race', async () => {
    vi.mocked(findRefreshTokenByHash).mockResolvedValue(makeToken({ family_id: 'family-race' }));
    vi.mocked(findUserById).mockResolvedValue(activeUser());
    vi.mocked(consumeAndRotateRefreshToken).mockResolvedValue({ status: 'failed' });

    const res = await post({ refreshToken: 'raw-token' });

    expect(res.status).toBe(401);
    expect(await res.json()).toEqual({ error: 'Token reuse detected' });
    expect(vi.mocked(revokeRefreshFamilyAndAccessTokensForUser)).toHaveBeenCalledWith(
      expect.anything(),
      'family-race',
      'user-1'
    );
  });

  it('does not return a session when rotate fails after concurrent family revocation', async () => {
    // Regression for clawpatch high: a winner that consumed then lost the
    // mint race against a concurrent family revoke must not hand back a
    // live replacement refresh token.
    vi.mocked(findRefreshTokenByHash).mockResolvedValue(
      makeToken({ family_id: 'family-post-revoke' })
    );
    vi.mocked(findUserById).mockResolvedValue(activeUser());
    vi.mocked(consumeAndRotateRefreshToken).mockResolvedValue({ status: 'failed' });

    const res = await post({ refreshToken: 'raw-token' });

    expect(res.status).toBe(401);
    const body = await res.json();
    expect(body).toEqual({ error: 'Token reuse detected' });
    expect(body).not.toHaveProperty('license');
    expect(body).not.toHaveProperty('refreshToken');
    expect(vi.mocked(findActiveScopesForUser)).not.toHaveBeenCalled();
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
