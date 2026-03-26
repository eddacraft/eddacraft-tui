/**
 * API Auth Flow — E2E Tests
 *
 * Tests the /auth/verify endpoint with various token states.
 * Uses the same mocking approach as the unit tests in anvil-api:
 * mock the DB modules via relative-path vi.mock, then mount
 * the route onto a test Hono app.
 *
 * Surface: API (auth route)
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { Hono } from 'hono';

// Mock the DB layer at the source-resolved paths (via vitest alias)
vi.mock('../../../anvil-api/src/db/client.js', () => ({
  getClient: vi.fn(),
}));

vi.mock('../../../anvil-api/src/db/queries.js', () => ({
  findTokenByHash: vi.fn(),
}));

vi.mock('../../../anvil-api/src/lib/token.js', async (importOriginal) => {
  const actual = await importOriginal<Record<string, unknown>>();
  return {
    ...actual,
    hashToken: vi.fn().mockReturnValue('mocked-hash'),
  };
});

// Import after mocks
const { auth } = await import('../../../anvil-api/src/routes/auth.js');
const { findTokenByHash } = await import('../../../anvil-api/src/db/queries.js');

const app = new Hono();
app.route('/auth', auth);

function post(path: string, body: unknown) {
  return app.request(path, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
  });
}

beforeEach(() => {
  vi.clearAllMocks();
});

afterEach(() => {
  vi.restoreAllMocks();
});

describe('API Auth Flow › POST /auth/verify', () => {
  it('rejects requests without a token field (400)', async () => {
    const res = await post('/auth/verify', {});
    expect(res.status).toBe(400);
  });

  it('returns valid: false for an invalid token format', async () => {
    const res = await post('/auth/verify', { token: 'not-a-real-token' });
    expect(res.status).toBe(200);
    expect(await res.json()).toEqual({ valid: false });
  });

  it('returns valid: false when token is not found in DB', async () => {
    vi.mocked(findTokenByHash).mockResolvedValue(null);
    const token = 'anvil_beta_' + 'A'.repeat(43);
    const res = await post('/auth/verify', { token });
    expect(res.status).toBe(200);
    expect(await res.json()).toEqual({ valid: false });
  });

  it('returns valid: false for a revoked token', async () => {
    vi.mocked(findTokenByHash).mockResolvedValue({
      id: '1',
      user_id: '2',
      token_hash: 'hash',
      scopes: ['beta'],
      expires_at: new Date(Date.now() + 86_400_000).toISOString(),
      revoked_at: new Date().toISOString(),
      created_at: new Date().toISOString(),
      email: 'user@test.local',
      user_status: 'active',
    });
    const token = 'anvil_beta_' + 'B'.repeat(43);
    const res = await post('/auth/verify', { token });
    expect(res.status).toBe(200);
    expect(await res.json()).toEqual({ valid: false });
  });

  it('returns valid: true for an active token', async () => {
    const expiresAt = new Date(Date.now() + 86_400_000).toISOString();
    vi.mocked(findTokenByHash).mockResolvedValue({
      id: '1',
      user_id: '2',
      token_hash: 'hash',
      scopes: ['beta'],
      expires_at: expiresAt,
      revoked_at: null,
      created_at: new Date().toISOString(),
      email: 'user@test.local',
      user_status: 'active',
    });
    const token = 'anvil_beta_' + 'C'.repeat(43);
    const res = await post('/auth/verify', { token });
    expect(res.status).toBe(200);
    const body = await res.json();
    expect(body).toEqual({
      valid: true,
      user: { email: 'user@test.local' },
      scopes: ['beta'],
      expiresAt,
    });
  });
});
