import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../db/queries.js', () => ({
  findActiveScopesForUser: vi.fn(),
  insertRefreshToken: vi.fn(),
}));

vi.mock('../lib/licence.js', () => ({
  signLicence: vi.fn(),
}));

vi.mock('../lib/token.js', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../lib/token.js')>();
  return { ...actual, hashToken: vi.fn() };
});

import { findActiveScopesForUser, insertRefreshToken } from '../db/queries.js';
import { signLicence, type LicenceClaims } from '../lib/licence.js';
import { hashToken } from '../lib/token.js';
import { mintSession } from '../lib/session.js';

const sql = {} as never;
const user = { id: 'user-1', email: 'alice@example.com' };

beforeEach(() => {
  vi.mocked(findActiveScopesForUser).mockResolvedValue(['beta', 'preview']);
  vi.mocked(signLicence).mockResolvedValue('signed.jwt.token');
  vi.mocked(hashToken).mockImplementation((t: string) => `hash:${t}`);
  vi.mocked(insertRefreshToken).mockResolvedValue(undefined as never);
});

afterEach(() => vi.clearAllMocks());

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
