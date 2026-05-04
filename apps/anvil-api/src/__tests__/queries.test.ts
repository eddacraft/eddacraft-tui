import { describe, expect, it, vi } from 'vitest';
import type { NeonClient } from '../db/client.js';
import {
  findActiveScopesForUser,
  revokeRefreshFamilyAndAccessTokensForUser,
} from '../db/queries.js';

function mockSql(result?: unknown): NeonClient {
  const sql = vi.fn().mockResolvedValue(result) as ReturnType<typeof vi.fn> & {
    transaction: ReturnType<typeof vi.fn>;
  };
  sql.transaction = vi.fn();
  return sql as unknown as NeonClient;
}

describe('db queries', () => {
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
});
