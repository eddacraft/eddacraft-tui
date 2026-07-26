import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { Hono } from 'hono';
import { admin } from '../routes/admin.js';
import { _resetAdminRateLimitForTests } from '../middleware/admin-rate-limit.js';

vi.mock('../lib/feature-flags.js', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../lib/feature-flags.js')>();
  return {
    ...actual,
    resolveApiScope: vi.fn(actual.resolveApiScope),
  };
});

afterEach(() => {
  vi.restoreAllMocks();
});

const ADMIN_KEY = 'test-admin-key-12345';

// Create a mock SQL tagged template function with transaction support
function createMockSql() {
  const sql = vi.fn() as ReturnType<typeof vi.fn> & { transaction: ReturnType<typeof vi.fn> };
  sql.transaction = vi.fn();
  return sql;
}

const mockSql = createMockSql();

// Mock db client
vi.mock('../db/client.js', () => ({
  getClient: vi.fn(() => mockSql),
}));

// Mock queries
vi.mock('../db/queries.js', () => ({
  findUserByEmail: vi.fn(),
  findUserWithTokens: vi.fn(),
  insertAuditLog: vi.fn().mockResolvedValue({
    id: 'audit-1',
    action: '',
    actor: '',
    metadata: {},
    created_at: new Date().toISOString(),
  }),
  upsertWaitlistWithName: vi.fn().mockResolvedValue(undefined),
  findWaitlistEntryByEmail: vi.fn().mockResolvedValue({ id: '1' }),
  findUnapprovedWaitlistEntries: vi.fn().mockResolvedValue([]),
  findWaitlistBySource: vi.fn().mockResolvedValue([]),
  findWaitlistPaginated: vi.fn().mockResolvedValue({ total: 0, items: [] }),
  findAuditEntries: vi.fn().mockResolvedValue({ total: 0, items: [] }),
  findRecentAuditForEmail: vi.fn().mockResolvedValue([]),
  insertBroadcastSnapshot: vi.fn(),
  findBroadcastSnapshot: vi.fn().mockResolvedValue(null),
  consumeBroadcastSnapshot: vi.fn().mockResolvedValue(null),
  findAdminKeyByHash: vi.fn().mockResolvedValue(null),
  findActiveScopesForUser: vi.fn().mockResolvedValue(['beta']),
}));

// Mock token utilities
vi.mock('../lib/token.js', () => ({
  generateToken: vi.fn().mockReturnValue('anvil_beta_' + 'X'.repeat(43)),
  hashToken: vi.fn().mockReturnValue('mocked-hash'),
  isValidTokenFormat: vi.fn().mockReturnValue(true),
}));

// Mock email (invite flow sends beta invite)
vi.mock('../lib/email.js', () => ({
  sendBetaInvite: vi.fn().mockResolvedValue({ sent: true }),
  sendWaitlistMigration: vi.fn().mockResolvedValue({ sent: true }),
}));

// Mock audience (invite flow moves to approved audience)
vi.mock('../lib/audience.js', () => ({
  moveToApprovedAudience: vi.fn().mockResolvedValue(undefined),
  removeFromBetaAudience: vi.fn().mockResolvedValue(undefined),
}));

// EMAIL-006: /admin/send-migration now resolves recipients via the
// broadcast-audiences resolver instead of findWaitlistBySource. Mock
// resolveAudience so the existing send-migration tests can drive it.
const resolveAudienceMock = vi.hoisted(() => vi.fn());
vi.mock('../lib/broadcast-audiences.js', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../lib/broadcast-audiences.js')>();
  return {
    ...actual,
    resolveAudience: resolveAudienceMock,
  };
});

import {
  findUserByEmail,
  findUserWithTokens,
  upsertWaitlistWithName,
  findWaitlistPaginated,
  findAuditEntries,
  findRecentAuditForEmail,
  findWaitlistEntryByEmail,
  findUnapprovedWaitlistEntries,
  insertAuditLog,
  insertBroadcastSnapshot,
  findBroadcastSnapshot,
  consumeBroadcastSnapshot,
  findAdminKeyByHash,
  findActiveScopesForUser,
} from '../db/queries.js';
import { sendBetaInvite, sendWaitlistMigration } from '../lib/email.js';
import { resolveApiScope } from '../lib/feature-flags.js';

const app = new Hono();
app.route('/admin', admin);

function request(method: string, path: string, body?: unknown, authKey?: string) {
  const headers: Record<string, string> = { 'Content-Type': 'application/json' };
  if (authKey) {
    headers['Authorization'] = `Bearer ${authKey}`;
  }
  return app.request(path, {
    method,
    headers,
    body: body ? JSON.stringify(body) : undefined,
  });
}

describe('admin endpoints', () => {
  const originalAdminKey = process.env['ADMIN_KEY'];

  beforeEach(() => {
    vi.clearAllMocks();
    _resetAdminRateLimitForTests();
    process.env['ADMIN_KEY'] = ADMIN_KEY;
    vi.mocked(resolveApiScope).mockImplementation((scope) => ({
      allowed: ['beta', 'preview', 'internal'].includes(scope),
      details: {
        value: true,
        variant: 'enabled',
        reason: 'default',
        flagKey: `api.scope.${scope}`,
      },
    }));
  });

  afterEach(() => {
    if (originalAdminKey !== undefined) {
      process.env['ADMIN_KEY'] = originalAdminKey;
    } else {
      delete process.env['ADMIN_KEY'];
    }
  });

  describe('auth middleware', () => {
    it('returns 401 without Authorization header', async () => {
      const res = await request('POST', '/admin/invite', { email: 'test@example.com' });
      expect(res.status).toBe(401);
    });

    it('returns 403 with wrong key', async () => {
      const res = await request(
        'POST',
        '/admin/invite',
        { email: 'test@example.com' },
        'wrong-key'
      );
      expect(res.status).toBe(403);
    });
  });

  describe('rate limiting (#951)', () => {
    // The coarse per-actor cap is 60 req/min. We don't want to issue
    // 61 real `/admin/audit` calls here; instead probe `GET
    // /admin/audit` (cheap) under the shared-key bucket until 429.
    it('caps the shared-key bucket at 60 admin requests/min', async () => {
      mockSql.mockResolvedValue([]);
      vi.mocked(findAuditEntries).mockResolvedValue({ total: 0, items: [] });
      const statuses: number[] = [];
      for (let i = 0; i < 62; i++) {
        const res = await request('GET', '/admin/audit', undefined, ADMIN_KEY);
        statuses.push(res.status);
      }
      // First 60 succeed; the 61st (and beyond) are throttled.
      expect(statuses.slice(0, 60).every((s) => s === 200)).toBe(true);
      expect(statuses[60]).toBe(429);
      expect(statuses[61]).toBe(429);
    });

    it('responds with the admin_rate_limited code body and Retry-After', async () => {
      mockSql.mockResolvedValue([]);
      vi.mocked(findAuditEntries).mockResolvedValue({ total: 0, items: [] });
      for (let i = 0; i < 60; i++) {
        await request('GET', '/admin/audit', undefined, ADMIN_KEY);
      }
      const res = await request('GET', '/admin/audit', undefined, ADMIN_KEY);
      expect(res.status).toBe(429);
      const body = (await res.json()) as Record<string, unknown>;
      expect(body['code']).toBe('admin_rate_limited');
      expect(body['scope']).toBe('all');
      expect(res.headers.get('Retry-After')).toBeTruthy();
    });
  });

  describe('POST /admin/invite', () => {
    it('default flow sends invite email and does not return token', async () => {
      // Mock waitlist insert (tagged template call)
      mockSql.mockResolvedValueOnce([]);
      // Mock transaction: [0] = stamp approved_at, [1] = upsert user, [2] = audit
      mockSql.transaction.mockResolvedValue([
        [{ email: 'alice@example.com' }],
        [{ id: 'user-1', email: 'alice@example.com' }],
        [{ id: 'audit-1' }],
      ]);

      const res = await request(
        'POST',
        '/admin/invite',
        { email: 'alice@example.com', notes: 'Design partner' },
        ADMIN_KEY
      );

      expect(res.status).toBe(201);
      const body = await res.json();
      expect(body.token).toBeUndefined();
      expect(body.user.email).toBe('alice@example.com');
      expect(body.scopes).toEqual(['beta']);
      expect(vi.mocked(upsertWaitlistWithName)).toHaveBeenCalledWith(
        expect.anything(),
        'alice@example.com',
        null,
        'manual'
      );
      expect(vi.mocked(sendBetaInvite)).toHaveBeenCalledWith('alice@example.com');
    });

    it('default flow writes no device_codes row (GHCLIAUTH-007)', async () => {
      mockSql.mockResolvedValueOnce([]);
      mockSql.transaction.mockResolvedValue([
        [{ email: 'alice@example.com' }],
        [{ id: 'user-1', email: 'alice@example.com' }],
        [{ id: 'audit-1' }],
      ]);

      const res = await request('POST', '/admin/invite', { email: 'alice@example.com' }, ADMIN_KEY);

      expect(res.status).toBe(201);
      const body = await res.json();
      expect(body.user.email).toBe('alice@example.com');
      const deviceCodeCall = mockSql.mock.calls.find((call) =>
        (call[0] as TemplateStringsArray).some((chunk) =>
          chunk.includes('INSERT INTO device_codes')
        )
      );
      expect(deviceCodeCall).toBeUndefined();
    });

    it('tokenOnly mode creates user and returns token', async () => {
      // Mock waitlist insert (tagged template call)
      mockSql.mockResolvedValueOnce([]);
      // Mock transaction: [0] = stamp approved_at, [1] = user, [2] = token, [3] = audit
      mockSql.transaction.mockResolvedValue([
        [{ email: 'alice@example.com' }],
        [{ id: 'user-1', email: 'alice@example.com' }],
        [{ id: 'token-1' }],
        [{ id: 'audit-1' }],
      ]);

      const res = await request(
        'POST',
        '/admin/invite',
        { email: 'alice@example.com', days: 90, notes: 'CI account', tokenOnly: true },
        ADMIN_KEY
      );

      expect(res.status).toBe(201);
      const body = await res.json();
      expect(body.token).toMatch(/^anvil_beta_/);
      expect(body.user.email).toBe('alice@example.com');
      expect(body.scopes).toEqual(['beta']);
      expect(mockSql.transaction).toHaveBeenCalledTimes(1);
    });

    it('edict mode records waitlist entry and marks access token as an edict', async () => {
      mockSql.transaction.mockResolvedValue([
        [{ email: 'alice@example.com' }],
        [{ id: 'user-1', email: 'alice@example.com' }],
        [{ id: 'token-1' }],
        [{ id: 'audit-1' }],
      ]);

      const res = await request(
        'POST',
        '/admin/invite',
        { email: 'alice@example.com', tokenOnly: true, edict: true },
        ADMIN_KEY
      );

      expect(res.status).toBe(201);
      expect(vi.mocked(upsertWaitlistWithName)).toHaveBeenCalledWith(
        expect.anything(),
        'alice@example.com',
        null,
        'manual'
      );
      expect(mockSql.mock.calls.some((call) => call.includes(true))).toBe(true);
    });

    it('rejects edict mode without tokenOnly', async () => {
      const res = await request(
        'POST',
        '/admin/invite',
        { email: 'alice@example.com', edict: true },
        ADMIN_KEY
      );

      expect(res.status).toBe(400);
    });

    it('returns 400 for invalid email', async () => {
      const res = await request('POST', '/admin/invite', { email: 'not-an-email' }, ADMIN_KEY);
      expect(res.status).toBe(400);
    });
  });

  describe('POST /admin/revoke', () => {
    // SEC-007: revocation is atomic — access tokens, refresh tokens, and
    // (for the email path) the user's `active` status all move in a single
    // batch transaction so that revoked accounts cannot pivot through
    // `/session/refresh` or re-mint via OAuth/OTP/device login. See
    // GH #1672 and plans/modules/security.aps.md SEC-007.
    it('atomically revokes access tokens, refresh tokens, and suspends the user when revoking by email', async () => {
      // Transaction result order matches admin.ts:
      // [0] = access_tokens revoked, [1] = refresh_tokens revoked,
      // [2] = beta_users suspended, [3] = audit log rows
      mockSql.transaction.mockResolvedValue([
        [{ id: 'token-1' }, { id: 'token-2' }],
        [{ id: 'refresh-1' }, { id: 'refresh-2' }, { id: 'refresh-3' }],
        [{ id: 'user-1' }],
        [{ id: 'audit-1' }],
      ]);

      const res = await request('POST', '/admin/revoke', { email: 'alice@example.com' }, ADMIN_KEY);

      expect(res.status).toBe(200);
      const body = await res.json();
      expect(body.revoked).toBe(2);
      expect(body.refreshSessionsRevoked).toBe(3);
      expect(body.accountSuspended).toBe(true);

      // Single batch transaction with all four statements
      expect(mockSql.transaction).toHaveBeenCalledTimes(1);
      const [statements] = mockSql.transaction.mock.calls[0] as [unknown[]];
      expect(statements).toHaveLength(4);
    });

    it('reports accountSuspended=false when the user was already inactive', async () => {
      mockSql.transaction.mockResolvedValue([
        [{ id: 'token-1' }],
        [{ id: 'refresh-1' }],
        [], // beta_users update matched no row (user already suspended/banned)
        [{ id: 'audit-1' }],
      ]);

      const res = await request('POST', '/admin/revoke', { email: 'alice@example.com' }, ADMIN_KEY);

      expect(res.status).toBe(200);
      const body = await res.json();
      expect(body.revoked).toBe(1);
      expect(body.refreshSessionsRevoked).toBe(1);
      expect(body.accountSuspended).toBe(false);
    });

    it('atomically revokes the access token and the user’s refresh sessions when revoking by token', async () => {
      // Transaction result order:
      // [0] = access_token revoked, [1] = refresh_tokens revoked, [2] = audit
      mockSql.transaction.mockResolvedValue([
        [{ id: 'token-1' }],
        [{ id: 'refresh-1' }, { id: 'refresh-2' }],
        [{ id: 'audit-1' }],
      ]);

      const res = await request(
        'POST',
        '/admin/revoke',
        { token: 'anvil_beta_' + 'X'.repeat(43) },
        ADMIN_KEY
      );

      expect(res.status).toBe(200);
      const body = await res.json();
      expect(body.revoked).toBe(1);
      expect(body.refreshSessionsRevoked).toBe(2);
      expect(body.accountSuspended).toBeUndefined();

      expect(mockSql.transaction).toHaveBeenCalledTimes(1);
      const [statements] = mockSql.transaction.mock.calls[0] as [unknown[]];
      expect(statements).toHaveLength(3);
    });

    it('reports revoked=0 when the access token hash is unknown', async () => {
      mockSql.transaction.mockResolvedValue([
        [], // no access_tokens row matched
        [], // refresh_tokens lookup yielded no user_id
        [{ id: 'audit-1' }],
      ]);

      const res = await request(
        'POST',
        '/admin/revoke',
        { token: 'anvil_beta_' + 'X'.repeat(43) },
        ADMIN_KEY
      );

      expect(res.status).toBe(200);
      const body = await res.json();
      expect(body.revoked).toBe(0);
      expect(body.refreshSessionsRevoked).toBe(0);
    });

    // SEC-007: idempotent double-revoke. The access_token row was already
    // revoked under the pre-fix code (so statement [0] matches zero rows),
    // but the user's refresh_tokens were never swept and remain usable.
    // Running revoke again must still scrub the leftover refresh sessions,
    // because statement [1] looks up `user_id` from the access_tokens row
    // without filtering on `revoked_at IS NULL`.
    it('sweeps refresh sessions on a double-revoke even when the access token is already revoked', async () => {
      mockSql.transaction.mockResolvedValue([
        [], // access_token already revoked — statement [0] matches nothing
        [{ id: 'refresh-1' }, { id: 'refresh-2' }], // refresh sessions still live
        [{ id: 'audit-1' }],
      ]);

      const res = await request(
        'POST',
        '/admin/revoke',
        { token: 'anvil_beta_' + 'X'.repeat(43) },
        ADMIN_KEY
      );

      expect(res.status).toBe(200);
      const body = await res.json();
      expect(body.revoked).toBe(0);
      expect(body.refreshSessionsRevoked).toBe(2);
    });

    it('returns 400 when neither email nor token provided', async () => {
      const res = await request('POST', '/admin/revoke', {}, ADMIN_KEY);
      expect(res.status).toBe(400);
    });

    // SEC-007 / Copilot review on #1806: email and token carry different
    // revocation semantics (account-level vs grant-level). Reject requests
    // that supply both so the server cannot silently pick a branch.
    it('returns 400 when both email and token are provided', async () => {
      const res = await request(
        'POST',
        '/admin/revoke',
        { email: 'alice@example.com', token: 'anvil_beta_' + 'X'.repeat(43) },
        ADMIN_KEY
      );
      expect(res.status).toBe(400);
    });
  });

  describe('GET /admin/user/:email', () => {
    it('returns user with tokens', async () => {
      vi.mocked(findUserWithTokens).mockResolvedValue({
        user: {
          id: 'user-1',
          email: 'alice@example.com',
          name: 'Alice',
          status: 'active',
          notes: null,
          created_at: new Date().toISOString(),
          updated_at: new Date().toISOString(),
        },
        tokens: [
          {
            id: 'token-1',
            user_id: 'user-1',
            token_hash: 'hash',
            scopes: ['beta'],
            is_edict: true,
            expires_at: new Date().toISOString(),
            revoked_at: null,
            created_at: new Date().toISOString(),
          },
        ],
      });

      const res = await request('GET', '/admin/user/alice@example.com', undefined, ADMIN_KEY);
      expect(res.status).toBe(200);
      const body = await res.json();
      expect(body.user.email).toBe('alice@example.com');
      expect(body.tokens).toHaveLength(1);
      expect(body.tokens[0].is_edict).toBe(true);
      // token_hash should not be exposed
      expect(body.tokens[0]).not.toHaveProperty('token_hash');
      expect(vi.mocked(findUserWithTokens)).toHaveBeenCalledWith(
        expect.anything(),
        'alice@example.com'
      );
    });

    it('returns 404 for unknown user', async () => {
      vi.mocked(findUserWithTokens).mockResolvedValue(null);
      const res = await request('GET', '/admin/user/nobody@example.com', undefined, ADMIN_KEY);
      expect(res.status).toBe(404);
    });

    it('returns 400 for invalid email format', async () => {
      const res = await request('GET', '/admin/user/not-an-email', undefined, ADMIN_KEY);
      expect(res.status).toBe(400);
    });

    it('includes recentAudit for the looked-up email', async () => {
      vi.mocked(findUserWithTokens).mockResolvedValue({
        user: {
          id: 'user-1',
          email: 'alice@example.com',
          name: 'Alice',
          status: 'active',
          notes: null,
          created_at: new Date().toISOString(),
          updated_at: new Date().toISOString(),
        },
        tokens: [],
      });
      vi.mocked(findRecentAuditForEmail).mockResolvedValue([
        {
          id: 'audit-1',
          action: 'user.invited',
          actor: 'josh@arkahna.io',
          metadata: { email: 'alice@example.com', scopes: ['beta'], days: 30 },
          created_at: '2026-04-17T09:00:00Z',
        },
        {
          id: 'audit-2',
          action: 'token.created',
          actor: 'josh@arkahna.io',
          metadata: { email: 'alice@example.com', scopes: ['beta'] },
          created_at: '2026-04-17T10:00:00Z',
        },
      ]);

      const res = await request('GET', '/admin/user/alice@example.com', undefined, ADMIN_KEY);
      expect(res.status).toBe(200);
      const body = await res.json();
      expect(body.recentAudit).toHaveLength(2);
      expect(body.recentAudit[0].action).toBe('user.invited');
      expect(vi.mocked(findRecentAuditForEmail)).toHaveBeenCalledWith(
        expect.anything(),
        'alice@example.com'
      );
    });

    it('returns empty recentAudit when no entries match', async () => {
      vi.mocked(findUserWithTokens).mockResolvedValue({
        user: {
          id: 'user-1',
          email: 'alice@example.com',
          name: null,
          status: 'active',
          notes: null,
          created_at: new Date().toISOString(),
          updated_at: new Date().toISOString(),
        },
        tokens: [],
      });
      vi.mocked(findRecentAuditForEmail).mockResolvedValue([]);

      const res = await request('GET', '/admin/user/alice@example.com', undefined, ADMIN_KEY);
      expect(res.status).toBe(200);
      const body = await res.json();
      expect(body.recentAudit).toEqual([]);
    });

    it('degrades to empty recentAudit when audit lookup throws', async () => {
      vi.mocked(findUserWithTokens).mockResolvedValue({
        user: {
          id: 'user-1',
          email: 'alice@example.com',
          name: null,
          status: 'active',
          notes: null,
          created_at: new Date().toISOString(),
          updated_at: new Date().toISOString(),
        },
        tokens: [],
      });
      vi.mocked(findRecentAuditForEmail).mockRejectedValue(new Error('db blew up'));
      const errSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

      const res = await request('GET', '/admin/user/alice@example.com', undefined, ADMIN_KEY);
      expect(res.status).toBe(200);
      const body = await res.json();
      expect(body.user.email).toBe('alice@example.com');
      expect(body.recentAudit).toEqual([]);
      expect(body.auditError).toBe(true);
      errSpy.mockRestore();
    });
  });

  describe('GET /admin/waitlist — waitlist list', () => {
    it('defaults to pending status, all sources, limit 50, offset 0', async () => {
      vi.mocked(findWaitlistPaginated).mockResolvedValue({ total: 0, items: [] });

      const res = await request('GET', '/admin/waitlist', undefined, ADMIN_KEY);
      expect(res.status).toBe(200);
      expect(vi.mocked(findWaitlistPaginated)).toHaveBeenCalledWith(expect.anything(), {
        status: 'pending',
        source: 'all',
        limit: 50,
        offset: 0,
      });
    });

    it('returns total and items in response body', async () => {
      vi.mocked(findWaitlistPaginated).mockResolvedValue({
        total: 2,
        items: [
          {
            email: 'alice@example.com',
            name: 'Alice',
            source: 'website',
            created_at: '2026-04-16T10:00:00Z',
            approved_at: null,
          },
          {
            email: 'bob@example.com',
            name: null,
            source: 'manual',
            created_at: '2026-04-17T09:00:00Z',
            approved_at: '2026-04-17T09:05:00Z',
          },
        ],
      });

      const res = await request('GET', '/admin/waitlist', undefined, ADMIN_KEY);
      const body = await res.json();
      expect(body.total).toBe(2);
      expect(body.items).toHaveLength(2);
      expect(body.items[0].email).toBe('alice@example.com');
      expect(body.items[1].approved_at).toBe('2026-04-17T09:05:00Z');
    });

    it('passes status, source, limit, and offset filters through', async () => {
      vi.mocked(findWaitlistPaginated).mockResolvedValue({ total: 0, items: [] });

      const res = await request(
        'GET',
        '/admin/waitlist?status=approved&source=manual&limit=10&offset=20',
        undefined,
        ADMIN_KEY
      );
      expect(res.status).toBe(200);
      expect(vi.mocked(findWaitlistPaginated)).toHaveBeenCalledWith(expect.anything(), {
        status: 'approved',
        source: 'manual',
        limit: 10,
        offset: 20,
      });
    });

    it('accepts status=all for unfiltered listing', async () => {
      vi.mocked(findWaitlistPaginated).mockResolvedValue({ total: 0, items: [] });

      const res = await request('GET', '/admin/waitlist?status=all', undefined, ADMIN_KEY);
      expect(res.status).toBe(200);
      expect(vi.mocked(findWaitlistPaginated)).toHaveBeenCalledWith(expect.anything(), {
        status: 'all',
        source: 'all',
        limit: 50,
        offset: 0,
      });
    });

    it('rejects invalid status with 400', async () => {
      const res = await request('GET', '/admin/waitlist?status=bogus', undefined, ADMIN_KEY);
      expect(res.status).toBe(400);
    });

    it('rejects invalid source with 400', async () => {
      const res = await request('GET', '/admin/waitlist?source=bogus', undefined, ADMIN_KEY);
      expect(res.status).toBe(400);
    });

    it('rejects limit above 200 with 400', async () => {
      const res = await request('GET', '/admin/waitlist?limit=500', undefined, ADMIN_KEY);
      expect(res.status).toBe(400);
    });

    it('rejects negative offset with 400', async () => {
      const res = await request('GET', '/admin/waitlist?offset=-1', undefined, ADMIN_KEY);
      expect(res.status).toBe(400);
    });

    it('requires admin auth', async () => {
      const res = await request('GET', '/admin/waitlist', undefined);
      expect(res.status).toBe(401);
    });
  });

  describe('GET /admin/audit — audit list', () => {
    it('defaults to limit 50, offset 0, no filters', async () => {
      vi.mocked(findAuditEntries).mockResolvedValue({ total: 0, items: [] });

      const res = await request('GET', '/admin/audit', undefined, ADMIN_KEY);
      expect(res.status).toBe(200);
      expect(vi.mocked(findAuditEntries)).toHaveBeenCalledWith(expect.anything(), {
        action: undefined,
        actor: undefined,
        limit: 50,
        offset: 0,
      });
    });

    it('returns items and total in response body', async () => {
      vi.mocked(findAuditEntries).mockResolvedValue({
        total: 1,
        items: [
          {
            id: 'audit-1',
            action: 'user.approved',
            actor: 'josh@arkahna.io',
            metadata: { email: 'alice@example.com' },
            created_at: '2026-04-17T09:00:00Z',
          },
        ],
      });

      const res = await request('GET', '/admin/audit', undefined, ADMIN_KEY);
      const body = await res.json();
      expect(body.total).toBe(1);
      expect(body.items).toHaveLength(1);
      expect(body.items[0].action).toBe('user.approved');
      expect(body.items[0].actor).toBe('josh@arkahna.io');
    });

    it('passes action and actor filters through', async () => {
      vi.mocked(findAuditEntries).mockResolvedValue({ total: 0, items: [] });

      const res = await request(
        'GET',
        '/admin/audit?action=user.approved&actor=josh@arkahna.io&limit=25&offset=10',
        undefined,
        ADMIN_KEY
      );
      expect(res.status).toBe(200);
      expect(vi.mocked(findAuditEntries)).toHaveBeenCalledWith(expect.anything(), {
        action: 'user.approved',
        actor: 'josh@arkahna.io',
        limit: 25,
        offset: 10,
      });
    });

    it('rejects limit above 200 with 400', async () => {
      const res = await request('GET', '/admin/audit?limit=500', undefined, ADMIN_KEY);
      expect(res.status).toBe(400);
    });

    it('rejects limit=0 with 400', async () => {
      const res = await request('GET', '/admin/audit?limit=0', undefined, ADMIN_KEY);
      expect(res.status).toBe(400);
    });

    it('rejects non-numeric limit with 400', async () => {
      const res = await request('GET', '/admin/audit?limit=abc', undefined, ADMIN_KEY);
      expect(res.status).toBe(400);
    });

    it('rejects negative offset with 400', async () => {
      const res = await request('GET', '/admin/audit?offset=-1', undefined, ADMIN_KEY);
      expect(res.status).toBe(400);
    });

    it('accepts non-email actor like "admin" (matches write-time sanitisation)', async () => {
      vi.mocked(findAuditEntries).mockResolvedValue({ total: 0, items: [] });
      const res = await request('GET', '/admin/audit?actor=admin', undefined, ADMIN_KEY);
      expect(res.status).toBe(200);
      expect(vi.mocked(findAuditEntries)).toHaveBeenCalledWith(
        expect.anything(),
        expect.objectContaining({ actor: 'admin' })
      );
    });

    it('rejects actor with control characters with 400', async () => {
      const res = await request('GET', '/admin/audit?actor=bad%09actor', undefined, ADMIN_KEY);
      expect(res.status).toBe(400);
    });

    it('rejects empty actor with 400', async () => {
      const res = await request('GET', '/admin/audit?actor=', undefined, ADMIN_KEY);
      expect(res.status).toBe(400);
    });

    it('requires admin auth', async () => {
      const res = await request('GET', '/admin/audit', undefined);
      expect(res.status).toBe(401);
    });
  });

  describe('POST /admin/approve', () => {
    it('single-email mode succeeds and returns approved entry', async () => {
      vi.mocked(findWaitlistEntryByEmail).mockResolvedValue({ id: 'wl-1' });
      mockSql.transaction.mockResolvedValue([
        [{ email: 'alice@example.com' }],
        [{ id: 'user-1', email: 'alice@example.com' }],
        [{ id: 'token-1' }],
        [{ id: 'audit-1' }],
      ]);

      const res = await request(
        'POST',
        '/admin/approve',
        { email: 'alice@example.com' },
        ADMIN_KEY
      );
      expect(res.status).toBe(200);
      const body = await res.json();
      expect(body.approved).toHaveLength(1);
      expect(body.approved[0].email).toBe('alice@example.com');
      expect(vi.mocked(sendBetaInvite)).toHaveBeenCalledWith('alice@example.com');
    });

    it('writes no device_codes row but keeps the scope-record token insert (GHCLIAUTH-007)', async () => {
      vi.mocked(findWaitlistEntryByEmail).mockResolvedValue({ id: 'wl-1' });
      mockSql.transaction.mockResolvedValue([
        [{ email: 'alice@example.com' }],
        [{ id: 'user-1', email: 'alice@example.com' }],
        [{ id: 'token-1' }],
        [{ id: 'audit-1' }],
      ]);

      const res = await request(
        'POST',
        '/admin/approve',
        { email: 'alice@example.com' },
        ADMIN_KEY
      );

      expect(res.status).toBe(200);
      const deviceCodeCall = mockSql.mock.calls.find((call) =>
        (call[0] as TemplateStringsArray).some((chunk) =>
          chunk.includes('INSERT INTO device_codes')
        )
      );
      expect(deviceCodeCall).toBeUndefined();
      const accessTokenCall = mockSql.mock.calls.find((call) =>
        (call[0] as TemplateStringsArray).some((chunk) =>
          chunk.includes('INSERT INTO access_tokens')
        )
      );
      expect(accessTokenCall).toBeDefined();
    });

    it('preserves existing graded scopes when approving a waitlisted user', async () => {
      vi.mocked(findWaitlistEntryByEmail).mockResolvedValue({ id: 'wl-1' });
      vi.mocked(findUserByEmail).mockResolvedValue({
        id: 'user-1',
        email: 'alice@example.com',
        name: null,
        status: 'active',
        notes: null,
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
      });
      vi.mocked(findActiveScopesForUser).mockResolvedValue(['preview', 'beta']);
      mockSql.transaction.mockResolvedValue([
        [{ email: 'alice@example.com' }],
        [{ id: 'user-1', email: 'alice@example.com' }],
        [{ id: 'token-1' }],
        [{ id: 'audit-1' }],
      ]);

      const res = await request(
        'POST',
        '/admin/approve',
        { email: 'alice@example.com' },
        ADMIN_KEY
      );

      expect(res.status).toBe(200);
      const accessTokenCall = mockSql.mock.calls.find((call) =>
        (call[0] as TemplateStringsArray).some((chunk) =>
          chunk.includes('INSERT INTO access_tokens')
        )
      );
      expect(accessTokenCall).toContainEqual(['preview', 'beta']);
    });

    it('drops disabled preserved scopes and audits the filtered grant atomically', async () => {
      vi.mocked(findWaitlistEntryByEmail).mockResolvedValue({ id: 'wl-1' });
      vi.mocked(findUserByEmail).mockResolvedValue({
        id: 'user-1',
        email: 'alice@example.com',
        name: null,
        status: 'active',
        notes: null,
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
      });
      vi.mocked(findActiveScopesForUser).mockResolvedValue(['preview', 'beta']);
      vi.mocked(resolveApiScope).mockImplementation((scope) => ({
        allowed: scope !== 'preview',
        details: {
          value: scope !== 'preview',
          variant: scope === 'preview' ? 'disabled' : 'enabled',
          reason: scope === 'preview' ? 'local_override' : 'default',
          flagKey: `api.scope.${scope}`,
        },
      }));
      mockSql.transaction.mockResolvedValue([
        [{ email: 'alice@example.com' }],
        [{ id: 'user-1', email: 'alice@example.com' }],
        [{ id: 'token-1' }],
        [{ id: 'audit-1' }],
        [{ id: 'scopes-audit-1' }],
      ]);

      const res = await request(
        'POST',
        '/admin/approve',
        { email: 'alice@example.com' },
        ADMIN_KEY
      );

      expect(res.status).toBe(200);
      const accessTokenCall = mockSql.mock.calls.find((call) =>
        (call[0] as TemplateStringsArray).some((chunk) =>
          chunk.includes('INSERT INTO access_tokens')
        )
      );
      expect(accessTokenCall).toContainEqual(['beta']);
      // The dropped-scopes audit is recorded inside the transaction (as the
      // 5th statement after waitlist stamp) — NOT via the standalone
      // insertAuditLog helper (zero-granted-scopes rejection path only).
      expect(vi.mocked(insertAuditLog)).not.toHaveBeenCalledWith(
        expect.anything(),
        'user.approve.scopes_dropped',
        expect.anything(),
        expect.anything(),
        expect.anything()
      );
      const transactionStatements = mockSql.transaction.mock.calls[0][0] as unknown[][];
      expect(transactionStatements).toHaveLength(5);
      const droppedAuditCall = mockSql.mock.calls.find(
        (call) => call[1] === 'user.approve.scopes_dropped'
      );
      expect(droppedAuditCall).toContainEqual(
        JSON.stringify({
          email: 'alice@example.com',
          droppedScopes: ['preview'],
          grantedScopes: ['beta'],
        })
      );
    });

    it('rejects approval when every requested scope is disabled', async () => {
      vi.mocked(findWaitlistEntryByEmail).mockResolvedValue({ id: 'wl-1' });
      vi.mocked(findUserByEmail).mockResolvedValue({
        id: 'user-1',
        email: 'alice@example.com',
        name: null,
        status: 'active',
        notes: null,
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
      });
      vi.mocked(findActiveScopesForUser).mockResolvedValue([]);
      vi.mocked(resolveApiScope).mockReturnValue({
        allowed: false,
        details: {
          value: false,
          variant: 'disabled',
          reason: 'local_override',
          flagKey: 'api.scope.beta',
        },
      });

      const res = await request(
        'POST',
        '/admin/approve',
        { email: 'alice@example.com' },
        ADMIN_KEY
      );

      expect(res.status).toBe(409);
      expect(await res.json()).toEqual({ error: 'No enabled API scopes available for approval' });
      expect(mockSql.transaction).not.toHaveBeenCalled();
      expect(vi.mocked(insertAuditLog)).toHaveBeenCalledWith(
        mockSql,
        'user.approve.scopes_dropped',
        'shared-key@anvil',
        {
          email: 'alice@example.com',
          droppedScopes: ['beta'],
          grantedScopes: [],
        },
        'shared'
      );
    });

    it('still returns no_scopes when dropped-scope audit logging fails', async () => {
      vi.mocked(findWaitlistEntryByEmail).mockResolvedValue({ id: 'wl-1' });
      vi.mocked(findUserByEmail).mockResolvedValue({
        id: 'user-1',
        email: 'alice@example.com',
        name: null,
        status: 'active',
        notes: null,
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
      });
      vi.mocked(findActiveScopesForUser).mockResolvedValue([]);
      vi.mocked(resolveApiScope).mockReturnValue({
        allowed: false,
        details: {
          value: false,
          variant: 'disabled',
          reason: 'local_override',
          flagKey: 'api.scope.beta',
        },
      });
      vi.mocked(insertAuditLog).mockRejectedValueOnce(new Error('audit down'));

      const res = await request(
        'POST',
        '/admin/approve',
        { email: 'alice@example.com' },
        ADMIN_KEY
      );

      expect(res.status).toBe(409);
      expect(await res.json()).toEqual({ error: 'No enabled API scopes available for approval' });
    });

    it('returns 404 when email not on waitlist', async () => {
      vi.mocked(findWaitlistEntryByEmail).mockResolvedValue(null);

      const res = await request('POST', '/admin/approve', { email: 'bob@example.com' }, ADMIN_KEY);
      expect(res.status).toBe(404);
    });

    it('batch mode returns skipped entries with reasons', async () => {
      vi.mocked(findUnapprovedWaitlistEntries).mockResolvedValue([
        { email: 'alice@example.com' },
        { email: 'bob@example.com' },
      ]);
      vi.mocked(findWaitlistEntryByEmail).mockResolvedValue({ id: 'wl-x' });
      // First email succeeds, second fails on a database error
      mockSql.transaction
        .mockResolvedValueOnce([
          [{ email: 'alice@example.com' }],
          [{ id: 'user-1', email: 'alice@example.com' }],
          [{ id: 'token-1' }],
          [{ id: 'audit-1' }],
        ])
        .mockRejectedValueOnce(new Error('connection reset'));

      const res = await request('POST', '/admin/approve', { batch: 2 }, ADMIN_KEY);
      expect(res.status).toBe(200);
      const body = await res.json();
      expect(body.approved).toHaveLength(1);
      expect(body.approved[0].email).toBe('alice@example.com');
      expect(body.skipped).toHaveLength(1);
      expect(body.skipped[0]).toMatchObject({
        email: 'bob@example.com',
        reason: 'error',
      });
    });

    it('batch mode reports fully dropped scopes as skipped and audits the drop', async () => {
      vi.mocked(findUnapprovedWaitlistEntries).mockResolvedValue([{ email: 'alice@example.com' }]);
      vi.mocked(findWaitlistEntryByEmail).mockResolvedValue({ id: 'wl-1' });
      vi.mocked(findUserByEmail).mockResolvedValue({
        id: 'user-1',
        email: 'alice@example.com',
        name: null,
        status: 'active',
        notes: null,
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
      });
      vi.mocked(findActiveScopesForUser).mockResolvedValue([]);
      vi.mocked(resolveApiScope).mockReturnValue({
        allowed: false,
        details: {
          value: false,
          variant: 'disabled',
          reason: 'local_override',
          flagKey: 'api.scope.beta',
        },
      });

      const res = await request('POST', '/admin/approve', { batch: 1 }, ADMIN_KEY);

      expect(res.status).toBe(200);
      const body = await res.json();
      expect(body.approved).toEqual([]);
      expect(body.skipped).toHaveLength(1);
      expect(body.skipped[0]).toMatchObject({
        email: 'alice@example.com',
        reason: 'no_scopes',
      });
      expect(vi.mocked(insertAuditLog)).toHaveBeenCalledWith(
        mockSql,
        'user.approve.scopes_dropped',
        'shared-key@anvil',
        {
          email: 'alice@example.com',
          droppedScopes: ['beta'],
          grantedScopes: [],
        },
        'shared'
      );
    });
  });

  describe('POST /admin/user/email-update', () => {
    // vi.clearAllMocks() doesn't drop mockResolvedValueOnce queues, so reset
    // findUserByEmail explicitly to avoid state leaking across tests here.
    beforeEach(() => {
      vi.mocked(findUserByEmail).mockReset();
    });

    const existingUser = {
      id: 'user-1',
      email: 'old@example.com',
      status: 'active',
      name: null,
      notes: null,
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString(),
    };

    it('updates the email and writes an audit entry', async () => {
      vi.mocked(findUserByEmail).mockResolvedValueOnce(existingUser).mockResolvedValueOnce(null);
      mockSql.transaction.mockResolvedValue([
        [{ id: 'user-1', email: 'new@example.com', status: 'active' }],
        [{ id: 'audit-1' }],
      ]);

      const res = await request(
        'POST',
        '/admin/user/email-update',
        { currentEmail: 'old@example.com', newEmail: 'new@example.com' },
        ADMIN_KEY
      );

      expect(res.status).toBe(200);
      const body = await res.json();
      expect(body.user).toEqual({ id: 'user-1', email: 'new@example.com', status: 'active' });
      expect(body.previousEmail).toBe('old@example.com');
      expect(mockSql.transaction).toHaveBeenCalledTimes(1);
    });

    it('normalises mixed-case input before lookup', async () => {
      vi.mocked(findUserByEmail).mockResolvedValueOnce(existingUser).mockResolvedValueOnce(null);
      mockSql.transaction.mockResolvedValue([
        [{ id: 'user-1', email: 'new@example.com', status: 'active' }],
        [{ id: 'audit-1' }],
      ]);

      const res = await request(
        'POST',
        '/admin/user/email-update',
        { currentEmail: 'OLD@Example.com', newEmail: 'NEW@Example.com' },
        ADMIN_KEY
      );

      expect(res.status).toBe(200);
      expect(vi.mocked(findUserByEmail)).toHaveBeenNthCalledWith(
        1,
        expect.anything(),
        'old@example.com'
      );
      expect(vi.mocked(findUserByEmail)).toHaveBeenNthCalledWith(
        2,
        expect.anything(),
        'new@example.com'
      );
    });

    it('returns 404 when the current user does not exist', async () => {
      vi.mocked(findUserByEmail).mockResolvedValueOnce(null);

      const res = await request(
        'POST',
        '/admin/user/email-update',
        { currentEmail: 'missing@example.com', newEmail: 'new@example.com' },
        ADMIN_KEY
      );

      expect(res.status).toBe(404);
      expect(mockSql.transaction).not.toHaveBeenCalled();
    });

    it('returns 409 when the new email is already taken', async () => {
      vi.mocked(findUserByEmail)
        .mockResolvedValueOnce(existingUser)
        .mockResolvedValueOnce({ ...existingUser, id: 'user-2', email: 'taken@example.com' });

      const res = await request(
        'POST',
        '/admin/user/email-update',
        { currentEmail: 'old@example.com', newEmail: 'taken@example.com' },
        ADMIN_KEY
      );

      expect(res.status).toBe(409);
      expect(mockSql.transaction).not.toHaveBeenCalled();
    });

    it('returns 400 when new email matches current', async () => {
      const res = await request(
        'POST',
        '/admin/user/email-update',
        { currentEmail: 'same@example.com', newEmail: 'same@example.com' },
        ADMIN_KEY
      );

      expect(res.status).toBe(400);
      expect(vi.mocked(findUserByEmail)).not.toHaveBeenCalled();
    });

    it('returns 400 for invalid email format', async () => {
      const res = await request(
        'POST',
        '/admin/user/email-update',
        { currentEmail: 'not-an-email', newEmail: 'new@example.com' },
        ADMIN_KEY
      );
      expect(res.status).toBe(400);
    });

    it('returns 401 without admin auth', async () => {
      const res = await request('POST', '/admin/user/email-update', {
        currentEmail: 'old@example.com',
        newEmail: 'new@example.com',
      });
      expect(res.status).toBe(401);
    });

    it('returns 409 on concurrent unique violation', async () => {
      vi.mocked(findUserByEmail).mockResolvedValueOnce(existingUser).mockResolvedValueOnce(null);
      const uniqueError = Object.assign(new Error('duplicate key'), { code: '23505' });
      mockSql.transaction.mockRejectedValueOnce(uniqueError);

      const res = await request(
        'POST',
        '/admin/user/email-update',
        { currentEmail: 'old@example.com', newEmail: 'race@example.com' },
        ADMIN_KEY
      );

      expect(res.status).toBe(409);
    });

    it('returns 404 when user deleted between lookup and update', async () => {
      vi.mocked(findUserByEmail).mockResolvedValueOnce(existingUser).mockResolvedValueOnce(null);
      mockSql.transaction.mockResolvedValue([[], [{ id: 'audit-1' }]]);

      const res = await request(
        'POST',
        '/admin/user/email-update',
        { currentEmail: 'old@example.com', newEmail: 'new@example.com' },
        ADMIN_KEY
      );

      expect(res.status).toBe(404);
      const body = await res.json();
      expect(body.error).toBe('User was deleted during update');
    });
  });

  describe('POST /admin/send-migration — request validation & auth', () => {
    it('returns 401 without admin auth', async () => {
      const res = await app.request('/admin/send-migration', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ source: 'import', dryRun: true }),
      });

      expect(res.status).toBe(401);
      expect(resolveAudienceMock).not.toHaveBeenCalled();
      expect(vi.mocked(insertBroadcastSnapshot)).not.toHaveBeenCalled();
    });

    it('rejects invalid source via Zod with 400', async () => {
      const res = await request(
        'POST',
        '/admin/send-migration',
        { source: 'bogus', dryRun: true },
        ADMIN_KEY
      );

      expect(res.status).toBe(400);
      expect(resolveAudienceMock).not.toHaveBeenCalled();
      expect(vi.mocked(insertBroadcastSnapshot)).not.toHaveBeenCalled();
    });

    it('rejects limit above 80 via Zod with 400 (aligned with broadcastSchema)', async () => {
      const res = await request(
        'POST',
        '/admin/send-migration',
        { source: 'import', dryRun: true, limit: 500 },
        ADMIN_KEY
      );

      expect(res.status).toBe(400);
      expect(resolveAudienceMock).not.toHaveBeenCalled();
    });

    it('rejects limit below 1 via Zod with 400', async () => {
      const res = await request(
        'POST',
        '/admin/send-migration',
        { source: 'import', dryRun: true, limit: 0 },
        ADMIN_KEY
      );

      expect(res.status).toBe(400);
      expect(resolveAudienceMock).not.toHaveBeenCalled();
    });

    it('passes the caller-supplied limit through to resolveAudience on dry-run', async () => {
      resolveAudienceMock.mockResolvedValue([]);
      vi.mocked(insertBroadcastSnapshot).mockResolvedValue({
        token: 'tk',
        template: 'waitlist-migration',
        template_props: {},
        audience_key: 'waitlist:source',
        audience_params: { source: 'import' },
        recipients: [],
        created_by_actor: 'shared-key@anvil',
        created_at: '2026-04-17T09:00:00Z',
        expires_at: '2026-04-17T09:10:00Z',
        consumed_at: null,
      });

      const res = await request(
        'POST',
        '/admin/send-migration',
        { source: 'import', dryRun: true, limit: 7 },
        ADMIN_KEY
      );

      expect(res.status).toBe(200);
      expect(resolveAudienceMock).toHaveBeenCalledWith(expect.anything(), 'waitlist:source', {
        limit: 7,
        params: { source: 'import' },
      });
    });
  });

  describe('POST /admin/send-migration — snapshot token flow', () => {
    const importedRecipients = [
      { email: 'alice@example.com', name: 'Alice' },
      { email: 'bob@example.com', name: null },
    ];

    function makeSnapshot(overrides: Partial<Record<string, unknown>> = {}) {
      return {
        token: 'snap-token-abc',
        template: 'waitlist-migration',
        template_props: {},
        audience_key: 'waitlist:source',
        audience_params: { source: 'import' },
        recipients: importedRecipients,
        created_by_actor: 'josh@arkahna.io',
        created_at: '2026-04-17T09:00:00Z',
        expires_at: '2026-04-17T09:10:00Z',
        consumed_at: null,
        ...overrides,
      };
    }

    describe('dry-run', () => {
      it('returns previewToken plus the recipient snapshot', async () => {
        resolveAudienceMock.mockResolvedValue(importedRecipients);
        vi.mocked(insertBroadcastSnapshot).mockResolvedValue(makeSnapshot());

        const res = await request(
          'POST',
          '/admin/send-migration',
          { source: 'import', dryRun: true, limit: 20 },
          ADMIN_KEY
        );

        expect(res.status).toBe(200);
        const body = await res.json();
        expect(body.dryRun).toBe(true);
        expect(body.source).toBe('import');
        expect(body.count).toBe(2);
        expect(body.recipients).toEqual(importedRecipients);
        expect(body.previewToken).toBe('snap-token-abc');
        expect(body.expiresAt).toBe('2026-04-17T09:10:00Z');

        const insertCall = vi.mocked(insertBroadcastSnapshot).mock.calls[0]?.[1];
        expect(insertCall).toMatchObject({
          template: 'waitlist-migration',
          templateProps: {},
          audienceKey: 'waitlist:source',
          audienceParams: { source: 'import' },
          recipients: importedRecipients,
          createdByActor: 'shared-key@anvil',
          ttlSeconds: 600,
        });
        expect(typeof insertCall?.token).toBe('string');
        expect(insertCall?.token.length).toBeGreaterThanOrEqual(16);
      });

      it('binds the snapshot to the sentinel and ignores X-Admin-Actor on shared-key auth', async () => {
        resolveAudienceMock.mockResolvedValue([]);
        vi.mocked(insertBroadcastSnapshot).mockResolvedValue(
          makeSnapshot({ created_by_actor: 'shared-key@anvil', recipients: [] })
        );

        // Under ADMINCLIH-002, X-Admin-Actor is no longer trusted as the
        // creator — shared-key requests always record the sentinel.
        const res = await app.request('/admin/send-migration', {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
            Authorization: `Bearer ${ADMIN_KEY}`,
            'X-Admin-Actor': 'mallory@evil.example',
          },
          body: JSON.stringify({ source: 'import', dryRun: true }),
        });

        expect(res.status).toBe(200);
        const insertCall = vi.mocked(insertBroadcastSnapshot).mock.calls[0]?.[1];
        expect(insertCall?.createdByActor).toBe('shared-key@anvil');
      });
    });

    describe('real-send', () => {
      it('returns 400 preview_token_required when the token is missing', async () => {
        const res = await request(
          'POST',
          '/admin/send-migration',
          { source: 'import', dryRun: false },
          ADMIN_KEY
        );

        expect(res.status).toBe(400);
        const body = await res.json();
        expect(body.code).toBe('preview_token_required');
        expect(vi.mocked(consumeBroadcastSnapshot)).not.toHaveBeenCalled();
      });

      it('returns 410 preview_token_missing when the token is unknown', async () => {
        vi.mocked(consumeBroadcastSnapshot).mockResolvedValue(null);
        vi.mocked(findBroadcastSnapshot).mockResolvedValue(null);

        const res = await request(
          'POST',
          '/admin/send-migration',
          { source: 'import', dryRun: false, previewToken: 'ghost-token' },
          ADMIN_KEY
        );

        expect(res.status).toBe(410);
        const body = await res.json();
        expect(body.code).toBe('preview_token_missing');
      });

      it('returns 410 preview_token_missing when a different operator owns the token', async () => {
        // The find is scoped to (token, actor) in the DB layer, so a
        // caller who is not the creator receives `missing` — the server
        // must never confirm to a non-owner that the token exists.
        vi.mocked(consumeBroadcastSnapshot).mockResolvedValue(null);
        vi.mocked(findBroadcastSnapshot).mockResolvedValue(null);

        const res = await request(
          'POST',
          '/admin/send-migration',
          { source: 'import', dryRun: false, previewToken: 'snap-token-abc' },
          ADMIN_KEY
        );

        expect(res.status).toBe(410);
        const body = await res.json();
        expect(body.code).toBe('preview_token_missing');
        // Find was called with the caller's actor, not with any
        // foreign actor — the server does not widen the search.
        expect(vi.mocked(findBroadcastSnapshot)).toHaveBeenCalledWith(
          expect.anything(),
          expect.objectContaining({
            token: 'snap-token-abc',
            actor: expect.any(String),
          })
        );
      });

      it('returns 410 preview_token_consumed when the token was already used', async () => {
        vi.mocked(consumeBroadcastSnapshot).mockResolvedValue(null);
        vi.mocked(findBroadcastSnapshot).mockResolvedValue(
          makeSnapshot({
            created_by_actor: 'admin',
            consumed_at: '2026-04-17T09:05:00Z',
          })
        );

        const res = await request(
          'POST',
          '/admin/send-migration',
          { source: 'import', dryRun: false, previewToken: 'snap-token-abc' },
          ADMIN_KEY
        );

        expect(res.status).toBe(410);
        const body = await res.json();
        expect(body.code).toBe('preview_token_consumed');
      });

      it('returns 410 preview_token_expired when the token is past TTL', async () => {
        vi.mocked(consumeBroadcastSnapshot).mockResolvedValue(null);
        vi.mocked(findBroadcastSnapshot).mockResolvedValue(
          makeSnapshot({
            created_by_actor: 'admin',
            consumed_at: null,
            expires_at: '2026-04-17T08:00:00Z',
          })
        );

        const res = await request(
          'POST',
          '/admin/send-migration',
          { source: 'import', dryRun: false, previewToken: 'snap-token-abc' },
          ADMIN_KEY
        );

        expect(res.status).toBe(410);
        const body = await res.json();
        expect(body.code).toBe('preview_token_expired');
      });

      it('returns 409 cohort_drift with added/removed when the cohort changed', async () => {
        vi.mocked(consumeBroadcastSnapshot).mockResolvedValue(
          makeSnapshot({
            created_by_actor: 'admin',
            recipients: [
              { email: 'alice@example.com', name: 'Alice' },
              { email: 'bob@example.com', name: null },
            ],
          })
        );
        resolveAudienceMock.mockResolvedValue([
          { email: 'alice@example.com', name: 'Alice' },
          { email: 'carol@example.com', name: 'Carol' },
        ]);

        const res = await request(
          'POST',
          '/admin/send-migration',
          { source: 'import', dryRun: false, previewToken: 'snap-token-abc' },
          ADMIN_KEY
        );

        expect(res.status).toBe(409);
        const body = await res.json();
        expect(body.code).toBe('cohort_drift');
        expect(body.added).toEqual(['carol@example.com']);
        expect(body.removed).toEqual(['bob@example.com']);
        // Must NOT have sent any emails on a drift rejection.
        expect(vi.mocked(sendWaitlistMigration)).not.toHaveBeenCalled();
        // Must NOT write a `migration.email.sent` audit entry — but the
        // snapshot has been consumed (state change), so we DO write a
        // `migration.email.dispatch_started` (before the loop ran) and
        // a `migration.email.blocked` (with reason=cohort_drift).
        const auditCalls = vi.mocked(insertAuditLog).mock.calls;
        const actions = auditCalls.map((call) => call[1]);
        expect(actions).not.toContain('migration.email.sent');
        expect(actions).toContain('migration.email.dispatch_started');
        expect(actions).toContain('migration.email.blocked');
        const blockedCall = auditCalls.find((call) => call[1] === 'migration.email.blocked');
        expect(blockedCall?.[3]).toMatchObject({ reason: 'cohort_drift' });
      });

      it('rejects a second real-send with the same token as preview_token_consumed', async () => {
        // Simulate two back-to-back real-sends racing on the same token.
        // The first consume wins; the second finds a row whose
        // consumed_at is set and returns `preview_token_consumed`.
        const snap = makeSnapshot({
          created_by_actor: 'admin',
          recipients: importedRecipients,
        });
        vi.mocked(consumeBroadcastSnapshot).mockResolvedValueOnce(snap).mockResolvedValueOnce(null);
        vi.mocked(findBroadcastSnapshot).mockResolvedValueOnce(
          makeSnapshot({
            created_by_actor: 'admin',
            recipients: importedRecipients,
            consumed_at: '2026-04-17T09:05:00Z',
          })
        );
        resolveAudienceMock.mockResolvedValue(importedRecipients);
        vi.mocked(sendWaitlistMigration).mockResolvedValue({ sent: true });

        const first = await request(
          'POST',
          '/admin/send-migration',
          { source: 'import', dryRun: false, previewToken: 'snap-token-abc' },
          ADMIN_KEY
        );
        const second = await request(
          'POST',
          '/admin/send-migration',
          { source: 'import', dryRun: false, previewToken: 'snap-token-abc' },
          ADMIN_KEY
        );

        expect(first.status).toBe(200);
        expect(second.status).toBe(410);
        expect((await second.json()).code).toBe('preview_token_consumed');
        // Recipients were emailed exactly once across both calls.
        expect(vi.mocked(sendWaitlistMigration)).toHaveBeenCalledTimes(importedRecipients.length);
      });

      it('sends to the snapshot recipients on the golden path', async () => {
        vi.mocked(consumeBroadcastSnapshot).mockResolvedValue(
          makeSnapshot({
            created_by_actor: 'admin',
            recipients: importedRecipients,
          })
        );
        resolveAudienceMock.mockResolvedValue(importedRecipients);
        vi.mocked(sendWaitlistMigration).mockResolvedValue({ sent: true });

        const res = await request(
          'POST',
          '/admin/send-migration',
          { source: 'import', dryRun: false, previewToken: 'snap-token-abc' },
          ADMIN_KEY
        );

        expect(res.status).toBe(200);
        const body = await res.json();
        expect(body.source).toBe('import');
        expect(body.total).toBe(2);
        expect(body.sent).toBe(2);
        expect(body.failed).toBe(0);
        expect(vi.mocked(sendWaitlistMigration)).toHaveBeenCalledTimes(2);
        expect(vi.mocked(sendWaitlistMigration)).toHaveBeenCalledWith('alice@example.com', 'Alice');
        expect(vi.mocked(sendWaitlistMigration)).toHaveBeenCalledWith('bob@example.com', undefined);
        expect(vi.mocked(insertAuditLog)).toHaveBeenCalledWith(
          expect.anything(),
          'migration.email.sent',
          expect.any(String),
          expect.objectContaining({
            source: 'import',
            sent: 2,
            failed: 0,
            // Token is hashed in audit metadata so a DB-read leak of
            // audit_log doesn't recover usable preview tokens.
            previewTokenHash: expect.any(String),
          }),
          'shared'
        );
        // Confirm the hash is NOT the raw token. The hashToken mock
        // returns 'mocked-hash' in the test env; the assertion that
        // matters is that the raw token is not present.
        const auditCall = vi
          .mocked(insertAuditLog)
          .mock.calls.find((call) => call[1] === 'migration.email.sent');
        const metadata = auditCall?.[3] as { previewTokenHash: string };
        expect(metadata.previewTokenHash).toBe('mocked-hash');
        expect(metadata.previewTokenHash).not.toBe('snap-token-abc');
        expect(metadata).not.toHaveProperty('previewToken');
      });

      it('records partial failures without aborting the send', async () => {
        vi.mocked(consumeBroadcastSnapshot).mockResolvedValue(
          makeSnapshot({
            created_by_actor: 'admin',
            recipients: importedRecipients,
          })
        );
        resolveAudienceMock.mockResolvedValue(importedRecipients);
        vi.mocked(sendWaitlistMigration)
          .mockResolvedValueOnce({ sent: true })
          .mockResolvedValueOnce({ sent: false, message: 'smtp blew up' });

        const res = await request(
          'POST',
          '/admin/send-migration',
          { source: 'import', dryRun: false, previewToken: 'snap-token-abc' },
          ADMIN_KEY
        );

        expect(res.status).toBe(200);
        const body = await res.json();
        expect(body.sent).toBe(1);
        expect(body.failed).toBe(1);
        expect(body.results[1].error).toBe('smtp blew up');
      });
    });
  });

  // -------------------------------------------------------------------------
  // ADMINCLIH-002: per-operator admin keys
  // -------------------------------------------------------------------------
  describe('per-operator admin keys (ADMINCLIH-002)', () => {
    const originalFlag = process.env['ADMIN_PER_OPERATOR_KEYS'];
    const originalPepper = process.env['ADMIN_KEY_PEPPER'];

    beforeEach(() => {
      process.env['ADMIN_PER_OPERATOR_KEYS'] = '1';
      process.env['ADMIN_KEY_PEPPER'] = 'test-pepper';
    });

    afterEach(() => {
      if (originalFlag !== undefined) {
        process.env['ADMIN_PER_OPERATOR_KEYS'] = originalFlag;
      } else {
        delete process.env['ADMIN_PER_OPERATOR_KEYS'];
      }
      if (originalPepper !== undefined) {
        process.env['ADMIN_KEY_PEPPER'] = originalPepper;
      } else {
        delete process.env['ADMIN_KEY_PEPPER'];
      }
    });

    function activeKey(
      overrides: Partial<
        Parameters<typeof vi.mocked<typeof findAdminKeyByHash>>[0] extends never
          ? never
          : NonNullable<Awaited<ReturnType<typeof findAdminKeyByHash>>>
      > = {}
    ): NonNullable<Awaited<ReturnType<typeof findAdminKeyByHash>>> {
      return {
        id: 'key-1',
        hashed_key: 'hashed',
        actor_email: 'alice@eddacraft.ai',
        note: null,
        created_at: new Date().toISOString(),
        revoked_at: null,
        ...overrides,
      };
    }

    // Find the tagged-template call that issued the audit_log INSERT and
    // return its bind values.
    function findAuditInsertBinds(): unknown[] | undefined {
      for (const call of mockSql.mock.calls as unknown[][]) {
        const strings = call[0] as TemplateStringsArray | undefined;
        if (!strings) continue;
        if (strings.some((chunk) => chunk.includes('INSERT INTO audit_log'))) {
          return call.slice(1);
        }
      }
      return undefined;
    }

    it('authenticates via per-operator key and ignores X-Admin-Actor', async () => {
      vi.mocked(findAdminKeyByHash).mockResolvedValue(activeKey());

      mockSql.mockResolvedValueOnce([]); // upsertWaitlistWithName
      mockSql.transaction.mockResolvedValue([
        [{ email: 'bob@example.com' }],
        [{ id: 'user-1', email: 'bob@example.com' }],
        [{ id: 'token-1' }],
        [{ id: 'audit-1' }],
      ]);

      const res = await app.request('/admin/invite', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          Authorization: 'Bearer operator-raw-bearer',
          'X-Admin-Actor': 'mallory@evil.example',
        },
        body: JSON.stringify({ email: 'bob@example.com', tokenOnly: true }),
      });

      expect(res.status).toBe(201);
      const binds = findAuditInsertBinds();
      expect(binds).toBeDefined();
      // Bind order (tokenOnly path): action, actor, metadata, auth_method
      expect(binds).toContain('alice@eddacraft.ai');
      expect(binds).toContain('per_operator');
      expect(binds).not.toContain('mallory@evil.example');
    });

    it('returns 401 admin_key_revoked when the key is revoked', async () => {
      vi.mocked(findAdminKeyByHash).mockResolvedValue(
        activeKey({ revoked_at: new Date().toISOString() })
      );

      const res = await request(
        'POST',
        '/admin/invite',
        { email: 'bob@example.com', tokenOnly: true },
        'revoked-bearer'
      );

      expect(res.status).toBe(401);
      const body = await res.json();
      expect(body.code).toBe('admin_key_revoked');
      expect(vi.mocked(insertAuditLog)).toHaveBeenCalledWith(
        expect.anything(),
        'admin.auth.failed',
        'admin-auth-failure@anvil',
        expect.objectContaining({ outcome: 'rejected_revoked' }),
        'per_operator'
      );
    });

    it('returns 403 and audits unknown bearer when no key matches', async () => {
      vi.mocked(findAdminKeyByHash).mockResolvedValue(null);

      const res = await request(
        'POST',
        '/admin/invite',
        { email: 'bob@example.com', tokenOnly: true },
        'unknown-bearer'
      );

      expect(res.status).toBe(403);
      expect(vi.mocked(insertAuditLog)).toHaveBeenCalledWith(
        expect.anything(),
        'admin.auth.failed',
        'admin-auth-failure@anvil',
        expect.objectContaining({
          outcome: 'rejected_unknown',
          hashed_bearer: expect.any(String),
        }),
        'per_operator'
      );
    });

    it('falls back to shared-key path when admin_keys lookup throws', async () => {
      vi.mocked(findAdminKeyByHash).mockRejectedValue(new Error('db down'));
      // Shared-key comparison will succeed because the bearer is the ADMIN_KEY.
      mockSql.mockResolvedValueOnce([]); // upsertWaitlistWithName
      mockSql.transaction.mockResolvedValue([
        [{ email: 'bob@example.com' }],
        [{ id: 'user-1', email: 'bob@example.com' }],
        [{ id: 'token-1' }],
        [{ id: 'audit-1' }],
      ]);

      const res = await request(
        'POST',
        '/admin/invite',
        { email: 'bob@example.com', tokenOnly: true },
        ADMIN_KEY
      );

      expect(res.status).toBe(201);
      const binds = findAuditInsertBinds();
      expect(binds).toContain('shared-key@anvil');
      expect(binds).toContain('shared');
    });

    it('audits unknown bearer as shared when admin_keys lookup throws', async () => {
      vi.mocked(findAdminKeyByHash).mockRejectedValue(new Error('db down'));

      const res = await request(
        'POST',
        '/admin/invite',
        { email: 'bob@example.com', tokenOnly: true },
        'unknown-bearer'
      );

      expect(res.status).toBe(403);
      expect(vi.mocked(insertAuditLog)).toHaveBeenCalledWith(
        expect.anything(),
        'admin.auth.failed',
        'admin-auth-failure@anvil',
        expect.objectContaining({
          outcome: 'rejected_unknown',
          hashed_bearer: null,
        }),
        'shared'
      );
    });

    it('shared-key path ignores X-Admin-Actor', async () => {
      vi.mocked(findAdminKeyByHash).mockResolvedValue(null);
      mockSql.mockResolvedValueOnce([]);
      mockSql.transaction.mockResolvedValue([
        [{ email: 'bob@example.com' }],
        [{ id: 'user-1', email: 'bob@example.com' }],
        [{ id: 'token-1' }],
        [{ id: 'audit-1' }],
      ]);

      const res = await app.request('/admin/invite', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${ADMIN_KEY}`,
          'X-Admin-Actor': 'mallory@evil.example',
        },
        body: JSON.stringify({ email: 'bob@example.com', tokenOnly: true }),
      });

      expect(res.status).toBe(201);
      const binds = findAuditInsertBinds();
      expect(binds).toContain('shared-key@anvil');
      expect(binds).not.toContain('mallory@evil.example');
    });

    it('with flag off, bypasses admin_keys lookup entirely', async () => {
      process.env['ADMIN_PER_OPERATOR_KEYS'] = 'false';
      vi.mocked(findAdminKeyByHash).mockResolvedValue(activeKey());
      mockSql.mockResolvedValueOnce([]);
      mockSql.transaction.mockResolvedValue([
        [{ email: 'bob@example.com' }],
        [{ id: 'user-1', email: 'bob@example.com' }],
        [{ id: 'token-1' }],
        [{ id: 'audit-1' }],
      ]);

      const res = await request(
        'POST',
        '/admin/invite',
        { email: 'bob@example.com', tokenOnly: true },
        ADMIN_KEY
      );

      expect(res.status).toBe(201);
      expect(vi.mocked(findAdminKeyByHash)).not.toHaveBeenCalled();
      const binds = findAuditInsertBinds();
      expect(binds).toContain('shared-key@anvil');
    });

    it('with flag on but pepper unset, skips per-operator lookup and logs loudly', async () => {
      delete process.env['ADMIN_KEY_PEPPER'];
      vi.mocked(findAdminKeyByHash).mockResolvedValue(activeKey());
      mockSql.mockResolvedValueOnce([]);
      mockSql.transaction.mockResolvedValue([
        [{ email: 'bob@example.com' }],
        [{ id: 'user-1', email: 'bob@example.com' }],
        [{ id: 'token-1' }],
        [{ id: 'audit-1' }],
      ]);
      const errorSpy = vi.spyOn(console, 'error').mockImplementation(() => undefined);

      try {
        const res = await request(
          'POST',
          '/admin/invite',
          { email: 'bob@example.com', tokenOnly: true },
          ADMIN_KEY
        );

        expect(res.status).toBe(201);
        // Lookup MUST be skipped — hashing with an empty pepper would be
        // both predictable and guaranteed not to match any provisioned row.
        expect(vi.mocked(findAdminKeyByHash)).not.toHaveBeenCalled();
        expect(errorSpy).toHaveBeenCalledWith(expect.stringContaining('ADMIN_KEY_PEPPER is unset'));
        const binds = findAuditInsertBinds();
        expect(binds).toContain('shared-key@anvil');
      } finally {
        errorSpy.mockRestore();
      }
    });
  });
});
