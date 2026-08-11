import { describe, expect, it, vi } from 'vitest';
import type { NeonClient } from '../db/client.js';
import {
  consumeAndRotateRefreshToken,
  findActiveScopesForUser,
  insertOtpCodeIfUnderLimit,
  revokeRefreshFamilyAndAccessTokensForUser,
  stampUserLogin,
} from '../db/queries.js';

function mockSql(result?: unknown): NeonClient {
  const sql = vi.fn().mockResolvedValue(result) as ReturnType<typeof vi.fn> & {
    transaction: ReturnType<typeof vi.fn>;
  };
  sql.transaction = vi.fn();
  return sql as unknown as NeonClient;
}

describe('db queries', () => {
  describe('stampUserLogin', () => {
    it('issues a single UPDATE for the user id and method (BACT-002)', async () => {
      const sql = mockSql([]);
      await expect(stampUserLogin(sql, 'user-1', 'github')).resolves.toBeUndefined();
      expect(sql).toHaveBeenCalledTimes(1);
      // neon tagged template: first arg is strings, subsequent are values
      const call = vi.mocked(sql).mock.calls[0];
      expect(call).toBeDefined();
      // values include user id and method somewhere in the template args
      const flat = call?.flatMap((part) => (Array.isArray(part) ? part : [part]));
      const asText = JSON.stringify(flat);
      expect(asText).toContain('user-1');
      expect(asText).toContain('github');
    });
  });
  describe('findActiveScopesForUser', () => {
    it('defaults to beta when the user has no active token rows', async () => {
      const sql = mockSql([{ active_token_count: 0, scopes: [] }]);

      await expect(findActiveScopesForUser(sql, 'user-1')).resolves.toEqual(['beta']);
    });

    it('returns zero scopes when active token rows have empty scope arrays', async () => {
      const sql = mockSql([{ active_token_count: 1, scopes: [] }]);

      await expect(findActiveScopesForUser(sql, 'user-1')).resolves.toEqual([]);
    });
  });

  describe('revokeRefreshFamilyAndAccessTokensForUser', () => {
    it('revokes the refresh family and access tokens in one transaction', async () => {
      const sql = mockSql();
      vi.mocked(sql.transaction).mockResolvedValue([
        [{ id: 'rt-1' }],
        [{ id: 'at-1' }, { id: 'at-2' }],
      ]);

      await expect(
        revokeRefreshFamilyAndAccessTokensForUser(sql, 'family-1', 'user-1')
      ).resolves.toEqual({ refreshTokensRevoked: 1, accessTokensRevoked: 2 });

      expect(sql.transaction).toHaveBeenCalledTimes(1);
      expect(sql.transaction).toHaveBeenCalledWith([expect.anything(), expect.anything()]);
    });

    it('does not report partial success when the transaction fails', async () => {
      const sql = mockSql();
      vi.mocked(sql.transaction).mockRejectedValue(new Error('database unavailable'));

      await expect(
        revokeRefreshFamilyAndAccessTokensForUser(sql, 'family-1', 'user-1')
      ).rejects.toThrow('database unavailable');
    });
  });

  describe('consumeAndRotateRefreshToken', () => {
    const args = {
      oldTokenId: 'rt-1',
      userId: 'user-1',
      newTokenHash: 'hash:new-token',
      familyId: 'family-1',
      expiresAt: new Date('2026-09-01T00:00:00.000Z'),
    };

    it('returns the inserted token when the atomic rotate succeeds', async () => {
      const inserted = {
        id: 'rt-2',
        user_id: 'user-1',
        token_hash: 'hash:new-token',
        family_id: 'family-1',
        expires_at: '2026-09-01T00:00:00.000Z',
        revoked_at: null,
        consumed_at: null,
        created_at: '2026-07-18T00:00:00.000Z',
      };
      const sql = mockSql([inserted]);

      await expect(consumeAndRotateRefreshToken(sql, args)).resolves.toEqual({
        status: 'rotated',
        token: {
          id: 'rt-2',
          user_id: 'user-1',
          token_hash: 'hash:new-token',
          family_id: 'family-1',
          expires_at: '2026-09-01T00:00:00.000Z',
          revoked_at: null,
          consumed_at: null,
          created_at: '2026-07-18T00:00:00.000Z',
        },
      });
      expect(sql).toHaveBeenCalledTimes(1);
    });

    it('returns failed when no replacement row is produced', async () => {
      // Covers both consume-lost and family-already-revoked: the CTE returns
      // zero rows and must not leave a partial consume (single-statement).
      const sql = mockSql([]);

      await expect(consumeAndRotateRefreshToken(sql, args)).resolves.toEqual({
        status: 'failed',
      });
    });
  });

  describe('insertOtpCodeIfUnderLimit', () => {
    it('returns the inserted row when the cap still has room', async () => {
      const inserted = {
        id: 'otp-1',
        user_id: 'user-1',
        code_hash: 'hash:123456',
        attempts: 0,
        expires_at: '2026-07-18T01:00:00.000Z',
        consumed_at: null,
        created_at: '2026-07-18T00:00:00.000Z',
      };
      const sql = mockSql();
      vi.mocked(sql.transaction).mockResolvedValue([[{ pg_advisory_xact_lock: '' }], [inserted]]);

      await expect(
        insertOtpCodeIfUnderLimit(
          sql,
          'user-1',
          'hash:123456',
          new Date('2026-07-18T01:00:00.000Z'),
          3
        )
      ).resolves.toEqual({
        id: 'otp-1',
        user_id: 'user-1',
        code_hash: 'hash:123456',
        attempts: 0,
        expires_at: '2026-07-18T01:00:00.000Z',
        consumed_at: null,
        created_at: '2026-07-18T00:00:00.000Z',
      });
      expect(sql.transaction).toHaveBeenCalledWith([expect.anything(), expect.anything()]);
    });

    it('returns null when the cap blocks the insert', async () => {
      const sql = mockSql();
      vi.mocked(sql.transaction).mockResolvedValue([[{ pg_advisory_xact_lock: '' }], []]);

      await expect(
        insertOtpCodeIfUnderLimit(
          sql,
          'user-1',
          'hash:123456',
          new Date('2026-07-18T01:00:00.000Z'),
          3
        )
      ).resolves.toBeNull();
    });
  });
});
