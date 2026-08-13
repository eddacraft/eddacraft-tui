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

import { describe, it, expect, vi, beforeEach, afterEach, beforeAll, afterAll } from 'vitest';
import { Hono } from 'hono';
import { generateKeyPair, exportPKCS8 } from 'jose';

// Mock the DB layer at the source-resolved paths (via vitest alias)
vi.mock('../../../anvil-api/src/db/client.js', () => ({
  getClient: vi.fn(),
}));

vi.mock('../../../anvil-api/src/db/queries.js', () => ({
  findTokenByHash: vi.fn(),
  // /auth/verify and /auth/license/refresh now read live scopes via
  // findActiveScopesForUser (commit 907af5f2). Default to ['beta'] so
  // existing happy-path tests don't have to know about the lookup.
  findActiveScopesForUser: vi.fn().mockResolvedValue(['beta']),
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

let originalSigningKey: string | undefined;

function post(path: string, body: unknown) {
  return app.request(path, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
  });
}

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
    expect(body).toEqual(
      expect.objectContaining({
        valid: true,
        user: { email: 'user@test.local', plan: 'beta' },
        scopes: ['beta'],
        expiresAt,
      })
    );
    expect(body.license).toEqual(expect.any(String));
  });
});
