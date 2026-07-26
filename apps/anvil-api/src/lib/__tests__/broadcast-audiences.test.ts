import { describe, expect, it, vi } from 'vitest';
import type { NeonClient } from '../../db/client.js';
import {
  type AudienceKey,
  AUDIENCE_KEYS,
  RECENT_ACTIVITY_DAYS,
  resolveAudience,
} from '../broadcast-audiences.js';

function mockSql(result: unknown = []): NeonClient {
  return vi.fn().mockResolvedValue(result) as unknown as NeonClient;
}

function lastQuery(sql: NeonClient): string {
  const fn = sql as unknown as ReturnType<typeof vi.fn>;
  const call = fn.mock.calls.at(-1);
  if (!call) throw new Error('sql was not called');
  const fragments = call[0] as unknown;
  if (!Array.isArray(fragments)) throw new Error('first arg is not a tagged template');
  return (fragments as string[]).join(' ').replace(/\s+/g, ' ').trim();
}

describe('AUDIENCE_KEYS', () => {
  it('lists exactly the six v1 audiences', () => {
    expect(AUDIENCE_KEYS).toEqual([
      'beta:active',
      'beta:active-recent',
      'beta:active-idle',
      'waitlist:pending',
      'waitlist:source',
      'waitlist:approved-no-token',
    ]);
  });
});

describe('resolveAudience', () => {
  describe('beta:active', () => {
    it('queries beta_users filtered to active status', async () => {
      const sql = mockSql([]);
      await resolveAudience(sql, 'beta:active', { limit: 100 });
      const q = lastQuery(sql);
      expect(q).toContain('FROM beta_users');
      expect(q).toContain("status = 'active'");
    });

    it('maps rows to AudienceRow with user_id from beta_users.id', async () => {
      const sql = mockSql([{ email: 'a@x.com', name: 'Alice', user_id: 'u-1' }]);
      const rows = await resolveAudience(sql, 'beta:active', { limit: 100 });
      expect(rows).toEqual([{ email: 'a@x.com', name: 'Alice', user_id: 'u-1' }]);
    });

    it('preserves null name', async () => {
      const sql = mockSql([{ email: 'a@x.com', name: null, user_id: 'u-1' }]);
      const rows = await resolveAudience(sql, 'beta:active', { limit: 100 });
      expect(rows[0]?.name).toBeNull();
    });

    it('returns an empty array when no rows match', async () => {
      const sql = mockSql([]);
      const rows = await resolveAudience(sql, 'beta:active', { limit: 100 });
      expect(rows).toEqual([]);
    });
  });

  describe('beta:active-recent', () => {
    it('joins refresh_tokens and constrains to the 30-day window', async () => {
      const sql = mockSql([]);
      await resolveAudience(sql, 'beta:active-recent', { limit: 100 });
      const q = lastQuery(sql);
      expect(q).toContain('refresh_tokens');
      expect(q).toContain("status = 'active'");
      expect(q).toContain('revoked_at IS NULL');
      expect(q).toMatch(/INTERVAL/i);
    });

    it('uses RECENT_ACTIVITY_DAYS = 30', () => {
      expect(RECENT_ACTIVITY_DAYS).toBe(30);
    });

    it('uses EXISTS rather than JOIN+DISTINCT (Postgres rejects DISTINCT + ORDER BY on a non-selected column)', async () => {
      const sql = mockSql([{ email: 'a@x.com', name: 'Alice', user_id: 'u-1' }]);
      const rows = await resolveAudience(sql, 'beta:active-recent', { limit: 100 });
      expect(rows).toHaveLength(1);
      const q = lastQuery(sql);
      // Same pattern as beta:active-idle but without `NOT` — keeps the
      // pair symmetric and avoids the DISTINCT+ORDER-BY runtime error.
      expect(q).toMatch(/\bEXISTS\b/i);
      expect(q).not.toMatch(/\bNOT\s+EXISTS\b/i);
      expect(q).not.toMatch(/SELECT\s+DISTINCT/i);
    });
  });

  describe('beta:active-idle', () => {
    it('uses NOT EXISTS against refresh_tokens to invert active-recent', async () => {
      const sql = mockSql([]);
      await resolveAudience(sql, 'beta:active-idle', { limit: 100 });
      const q = lastQuery(sql);
      expect(q).toContain("status = 'active'");
      expect(q).toMatch(/NOT\s+EXISTS/i);
      expect(q).toContain('refresh_tokens');
      expect(q).toContain('revoked_at IS NULL');
      expect(q).toMatch(/INTERVAL/i);
    });
  });

  describe('waitlist:pending', () => {
    it('returns waitlist rows with approved_at still null', async () => {
      const sql = mockSql([]);
      await resolveAudience(sql, 'waitlist:pending', { limit: 100 });
      const q = lastQuery(sql);
      expect(q).toContain('FROM waitlist');
      expect(q).toContain('approved_at IS NULL');
      expect(q).not.toContain('LEFT JOIN beta_users');
    });

    it('returns null user_id for waitlist-only rows', async () => {
      const sql = mockSql([{ email: 'a@x.com', name: 'Alice', user_id: null }]);
      const rows = await resolveAudience(sql, 'waitlist:pending', { limit: 100 });
      expect(rows[0]?.user_id).toBeNull();
    });
  });

  describe('waitlist:source', () => {
    it('requires params.source and excludes already-approved rows', async () => {
      const sql = mockSql([]);
      await resolveAudience(sql, 'waitlist:source', {
        limit: 100,
        params: { source: 'import' },
      });
      const q = lastQuery(sql);
      expect(q).toContain('w.source =');
      expect(q).toContain('approved_at IS NULL');
    });

    it('passes the source value as a bound parameter', async () => {
      const sql = mockSql([]);
      await resolveAudience(sql, 'waitlist:source', {
        limit: 100,
        params: { source: 'import' },
      });
      const fn = sql as unknown as ReturnType<typeof vi.fn>;
      const args = fn.mock.calls.at(-1)!.slice(1);
      expect(args).toContain('import');
    });

    it('throws when params.source is missing', async () => {
      const sql = mockSql([]);
      await expect(resolveAudience(sql, 'waitlist:source', { limit: 100 })).rejects.toThrow(
        /source/
      );
    });
  });

  describe('waitlist:approved-no-token', () => {
    it('selects active beta_users with no live access_tokens', async () => {
      const sql = mockSql([]);
      await resolveAudience(sql, 'waitlist:approved-no-token', { limit: 100 });
      const q = lastQuery(sql);
      expect(q).toContain("status = 'active'");
      expect(q).toMatch(/NOT\s+EXISTS/i);
      expect(q).toContain('access_tokens');
      expect(q).toContain('revoked_at IS NULL');
      expect(q).toContain('expires_at');
    });

    it('excludes service accounts (any access_token with is_edict=true)', async () => {
      const sql = mockSql([]);
      await resolveAudience(sql, 'waitlist:approved-no-token', { limit: 100 });
      const q = lastQuery(sql);
      expect(q).toContain('is_edict = true');
    });
  });

  describe('hard exclusions', () => {
    const betaUsersAudiences: AudienceKey[] = [
      'beta:active',
      'beta:active-recent',
      'beta:active-idle',
      'waitlist:approved-no-token',
    ];

    it.each(betaUsersAudiences)(
      "%s narrows to status='active' so suspended/banned are excluded",
      async (key) => {
        const sql = mockSql([]);
        await resolveAudience(sql, key, { limit: 100 });
        const q = lastQuery(sql);
        expect(q).toContain("status = 'active'");
      }
    );

    it('waitlist:pending excludes admitted rows via approved_at IS NULL', async () => {
      const sql = mockSql([]);
      await resolveAudience(sql, 'waitlist:pending', { limit: 100 });
      const q = lastQuery(sql);
      expect(q).toContain('approved_at IS NULL');
    });

    it('waitlist:source excludes admitted rows via approved_at IS NULL', async () => {
      const sql = mockSql([]);
      await resolveAudience(sql, 'waitlist:source', {
        limit: 100,
        params: { source: 'import' },
      });
      const q = lastQuery(sql);
      expect(q).toContain('approved_at IS NULL');
    });
  });

  describe('limit handling', () => {
    it.each(AUDIENCE_KEYS)('%s honours the limit', async (key) => {
      const sql = mockSql([]);
      const params = key === 'waitlist:source' ? { source: 'import' } : undefined;
      await resolveAudience(sql, key, { limit: 250, params });
      const fn = sql as unknown as ReturnType<typeof vi.fn>;
      const args = fn.mock.calls.at(-1)!.slice(1);
      expect(args).toContain(250);
    });
  });

  describe('ordering', () => {
    it.each(AUDIENCE_KEYS)(
      '%s orders by created_at ASC for deterministic snapshots',
      async (key) => {
        const sql = mockSql([]);
        const params = key === 'waitlist:source' ? { source: 'import' } : undefined;
        await resolveAudience(sql, key, { limit: 100, params });
        const q = lastQuery(sql);
        expect(q).toMatch(/ORDER\s+BY[^L]+created_at\s+ASC/i);
      }
    );
  });
});
