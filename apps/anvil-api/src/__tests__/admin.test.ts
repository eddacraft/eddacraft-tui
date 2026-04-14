import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { Hono } from 'hono';
import { admin } from '../routes/admin.js';

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

// Mock queries (findUserWithTokens is still used directly)
vi.mock('../db/queries.js', () => ({
  findUserWithTokens: vi.fn(),
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

import { findUserWithTokens } from '../db/queries.js';

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
    process.env['ADMIN_KEY'] = ADMIN_KEY;
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

  describe('POST /admin/invite', () => {
    it('default flow sends invite email and does not return token', async () => {
      // Mock waitlist insert (tagged template call)
      mockSql.mockResolvedValueOnce([]);
      // Mock transaction: [0] = upsert user, [1] = insert token, [2] = insert device code, [3] = audit
      mockSql.transaction.mockResolvedValue([
        [{ id: 'user-1', email: 'alice@example.com' }],
        [{ id: 'token-1' }],
        [{ id: 'device-1' }],
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
    });

    it('tokenOnly mode creates user and returns token', async () => {
      // Mock waitlist insert (tagged template call)
      mockSql.mockResolvedValueOnce([]);
      // Mock transaction: [0] = upsert user, [1] = insert token, [2] = audit
      mockSql.transaction.mockResolvedValue([
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

    it('returns 400 for invalid email', async () => {
      const res = await request('POST', '/admin/invite', { email: 'not-an-email' }, ADMIN_KEY);
      expect(res.status).toBe(400);
    });
  });

  describe('POST /admin/revoke', () => {
    it('revokes all tokens by email', async () => {
      // Mock the transaction to return results:
      // [0] = revoked token rows, [1] = audit log rows
      mockSql.transaction.mockResolvedValue([
        [{ id: 'token-1' }, { id: 'token-2' }],
        [{ id: 'audit-1' }],
      ]);

      const res = await request('POST', '/admin/revoke', { email: 'alice@example.com' }, ADMIN_KEY);

      expect(res.status).toBe(200);
      const body = await res.json();
      expect(body.revoked).toBe(2);
    });

    it('returns 400 when neither email nor token provided', async () => {
      const res = await request('POST', '/admin/revoke', {}, ADMIN_KEY);
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
      // token_hash should not be exposed
      expect(body.tokens[0]).not.toHaveProperty('token_hash');
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
  });
});
