import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../db/queries.js', () => ({
  findActiveScopesForUser: vi.fn(),
  insertRefreshToken: vi.fn(),
  consumeAndRotateRefreshToken: vi.fn(),
  stampUserLogin: vi.fn(),
}));

vi.mock('../lib/licence.js', () => ({
  signLicence: vi.fn(),
}));

vi.mock('../lib/token.js', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../lib/token.js')>();
  return { ...actual, hashToken: vi.fn() };
});

import {
  consumeAndRotateRefreshToken,
  findActiveScopesForUser,
  insertRefreshToken,
  stampUserLogin,
} from '../db/queries.js';
import { signLicence, type LicenceClaims } from '../lib/licence.js';
import { hashToken } from '../lib/token.js';
import { mintRotatedSession, mintSession } from '../lib/session.js';

const sql = {} as never;
const user = { id: 'user-1', email: 'alice@example.com' };

beforeEach(() => {
  // resetAllMocks wipes implementations as well as call history; re-state every
  // default below so each test starts from the same baseline.
  vi.resetAllMocks();
  vi.mocked(findActiveScopesForUser).mockResolvedValue(['beta', 'preview']);
  vi.mocked(signLicence).mockResolvedValue('signed.jwt.token');
  vi.mocked(hashToken).mockImplementation((t: string) => `hash:${t}`);
  vi.mocked(insertRefreshToken).mockResolvedValue(undefined as never);
  vi.mocked(stampUserLogin).mockResolvedValue(undefined);
  vi.mocked(consumeAndRotateRefreshToken).mockResolvedValue({
    status: 'rotated',
    token: {
      id: 'rt-2',
      user_id: 'user-1',
      token_hash: 'hash:new',
      family_id: 'family-1',
      expires_at: '2026-09-01T00:00:00.000Z',
      revoked_at: null,
      consumed_at: null,
      created_at: '2026-07-18T00:00:00.000Z',
    },
  });
});

afterEach(() => vi.restoreAllMocks());

describe('mintSession', () => {
  it('signs a licence with the resolved scopes and the given identity', async () => {
    const identity: LicenceClaims['identity'] = { provider: 'github', id: '42' };
    const result = await mintSession(sql, { user, identity });

    expect(vi.mocked(findActiveScopesForUser)).toHaveBeenCalledWith(sql, 'user-1');
    expect(vi.mocked(signLicence)).toHaveBeenCalledWith(
      {
        sub: 'user-1',
        email: 'alice@example.com',
        identity: { provider: 'github', id: '42' },
        org: null,
        tier: 'pro',
        scopes: ['beta', 'preview'],
        seats: 1,
      },
      undefined,
      7
    );
    expect(result.license).toBe('signed.jwt.token');
  });

  it('returns a fresh hex refresh token and an ISO expiry ~7 days ahead', async () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date('2026-06-04T00:00:00.000Z'));
    try {
      const result = await mintSession(sql, {
        user,
        identity: { provider: 'email', id: null },
      });
      expect(result.refreshToken).toMatch(/^[0-9a-f]{64}$/);
      expect(result.expiresAt).toBe('2026-06-11T00:00:00.000Z');
    } finally {
      vi.useRealTimers();
    }
  });

  it('inserts a refresh token under a fresh random family by default', async () => {
    await mintSession(sql, { user, identity: { provider: 'email', id: null } });
    await mintSession(sql, { user, identity: { provider: 'email', id: null } });

    const calls = vi.mocked(insertRefreshToken).mock.calls;
    expect(calls).toHaveLength(2);
    // (sql, userId, hash, familyId, expiresAt)
    for (const call of calls) {
      expect(call[1]).toBe('user-1');
      expect(call[2]).toMatch(/^hash:[0-9a-f]{64}$/);
      expect(call[4]).toBeInstanceOf(Date);
    }
    expect(calls[0][3]).not.toBe(calls[1][3]); // distinct families
  });

  it('reuses a provided familyId (refresh rotation in the same family)', async () => {
    await mintSession(sql, {
      user,
      identity: { provider: 'email', id: null },
      familyId: 'family-1',
    });
    expect(vi.mocked(insertRefreshToken)).toHaveBeenCalledWith(
      sql,
      'user-1',
      expect.stringMatching(/^hash:[0-9a-f]{64}$/),
      'family-1',
      expect.any(Date)
    );
  });

  it('does not insert a refresh token if signing fails', async () => {
    vi.mocked(signLicence).mockRejectedValueOnce(new Error('signing key unavailable'));
    await expect(
      mintSession(sql, { user, identity: { provider: 'email', id: null } })
    ).rejects.toThrow('signing key unavailable');
    expect(vi.mocked(insertRefreshToken)).not.toHaveBeenCalled();
  });

  it('honours custom ttlDays and refreshTtlDays', async () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date('2026-06-04T00:00:00.000Z'));
    try {
      const result = await mintSession(sql, {
        user,
        identity: { provider: 'email', id: null },
        ttlDays: 30,
        refreshTtlDays: 1,
      });
      expect(vi.mocked(signLicence)).toHaveBeenCalledWith(expect.anything(), undefined, 30);
      expect(result.expiresAt).toBe('2026-07-04T00:00:00.000Z');
      const refreshExpiry = vi.mocked(insertRefreshToken).mock.calls[0][4] as Date;
      expect(refreshExpiry.toISOString()).toBe('2026-06-05T00:00:00.000Z');
    } finally {
      vi.useRealTimers();
    }
  });
});

it('stamps interactive login when loginMethod is provided (BACT-002)', async () => {
  await mintSession(sql, {
    user,
    identity: { provider: 'github', id: '42' },
    loginMethod: 'github',
  });
  expect(vi.mocked(stampUserLogin)).toHaveBeenCalledWith(sql, 'user-1', 'github');
});

it('does not stamp login when loginMethod is omitted', async () => {
  await mintSession(sql, {
    user,
    identity: { provider: 'email', id: null },
  });
  expect(vi.mocked(stampUserLogin)).not.toHaveBeenCalled();
});

it('stamps otp and device methods for email-identity paths', async () => {
  await mintSession(sql, {
    user,
    identity: { provider: 'email', id: null },
    loginMethod: 'otp',
  });
  await mintSession(sql, {
    user,
    identity: { provider: 'email', id: null },
    loginMethod: 'device',
  });
  expect(vi.mocked(stampUserLogin)).toHaveBeenNthCalledWith(1, sql, 'user-1', 'otp');
  expect(vi.mocked(stampUserLogin)).toHaveBeenNthCalledWith(2, sql, 'user-1', 'device');
});

describe('mintRotatedSession', () => {
  it('rotates via the atomic query then signs a licence', async () => {
    const result = await mintRotatedSession(sql, {
      user,
      identity: { provider: 'email', id: null },
      familyId: 'family-1',
      oldTokenId: 'rt-1',
    });

    expect(result).toEqual({
      ok: true,
      session: {
        license: 'signed.jwt.token',
        refreshToken: expect.stringMatching(/^[0-9a-f]{64}$/),
        expiresAt: expect.stringMatching(/^\d{4}-\d{2}-\d{2}T/),
      },
    });
    expect(vi.mocked(consumeAndRotateRefreshToken)).toHaveBeenCalledWith(sql, {
      oldTokenId: 'rt-1',
      userId: 'user-1',
      newTokenHash: expect.stringMatching(/^hash:[0-9a-f]{64}$/),
      familyId: 'family-1',
      expiresAt: expect.any(Date),
    });
    expect(vi.mocked(insertRefreshToken)).not.toHaveBeenCalled();
    expect(vi.mocked(findActiveScopesForUser)).toHaveBeenCalledWith(sql, 'user-1');
  });

  it('returns ok:false without signing when the atomic rotate fails', async () => {
    vi.mocked(consumeAndRotateRefreshToken).mockResolvedValue({ status: 'failed' });

    const result = await mintRotatedSession(sql, {
      user,
      identity: { provider: 'email', id: null },
      familyId: 'family-1',
      oldTokenId: 'rt-1',
    });

    expect(result).toEqual({ ok: false });
    expect(vi.mocked(signLicence)).not.toHaveBeenCalled();
    expect(vi.mocked(findActiveScopesForUser)).not.toHaveBeenCalled();
  });
});

describe('mintRotatedSession login stamps (BACT-002)', () => {
  it('does not stamp login on refresh rotation', async () => {
    await mintRotatedSession(sql, {
      user,
      identity: { provider: 'github', id: '42' },
      familyId: 'family-1',
      oldTokenId: 'rt-1',
    });
    expect(vi.mocked(stampUserLogin)).not.toHaveBeenCalled();
  });
});
