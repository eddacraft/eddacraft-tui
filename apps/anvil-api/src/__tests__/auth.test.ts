import { describe, it, expect, vi, beforeEach, afterEach, beforeAll, afterAll } from 'vitest';
import { generateKeyPair, exportPKCS8 } from 'jose';
import { Hono } from 'hono';
import { auth } from '../routes/auth.js';

// Mock the db client
vi.mock('../db/client.js', () => ({
  getClient: vi.fn(),
}));

// Mock queries
vi.mock('../db/queries.js', () => ({
  findTokenByHash: vi.fn(),
}));

// Mock token utilities (keep real implementations for format validation)
vi.mock('../lib/token.js', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../lib/token.js')>();
  return {
    ...actual,
    hashToken: vi.fn().mockReturnValue('mocked-hash'),
  };
});

import { findTokenByHash } from '../db/queries.js';

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

afterEach(() => {
  vi.restoreAllMocks();
});

const app = new Hono();
app.route('/auth', auth);

function post(path: string, body: unknown) {
  return app.request(path, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
  });
}

describe('POST /auth/verify', () => {
  const mockedFind = vi.mocked(findTokenByHash);

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('returns {valid: false} for invalid token format', async () => {
    const res = await post('/auth/verify', { token: 'not-a-valid-token' });
    expect(res.status).toBe(200);
    expect(await res.json()).toEqual({ valid: false });
  });

  it('returns {valid: false} when token not found in DB', async () => {
    mockedFind.mockResolvedValue(null);
    // Generate a valid-format token for testing
    const token = 'anvil_beta_' + 'A'.repeat(43);
    const res = await post('/auth/verify', { token });
    expect(res.status).toBe(200);
    expect(await res.json()).toEqual({ valid: false });
  });

  it('returns {valid: false} for revoked token', async () => {
    mockedFind.mockResolvedValue({
      id: '1',
      user_id: '2',
      token_hash: 'hash',
      scopes: ['beta'],
      expires_at: new Date(Date.now() + 86400000).toISOString(),
      revoked_at: new Date().toISOString(),
      created_at: new Date().toISOString(),
      email: 'test@example.com',
      user_status: 'active',
    });
    const token = 'anvil_beta_' + 'A'.repeat(43);
    const res = await post('/auth/verify', { token });
    expect(res.status).toBe(200);
    expect(await res.json()).toEqual({ valid: false });
  });

  it('returns {valid: false} for expired token', async () => {
    mockedFind.mockResolvedValue({
      id: '1',
      user_id: '2',
      token_hash: 'hash',
      scopes: ['beta'],
      expires_at: new Date(Date.now() - 86400000).toISOString(),
      revoked_at: null,
      created_at: new Date().toISOString(),
      email: 'test@example.com',
      user_status: 'active',
    });
    const token = 'anvil_beta_' + 'A'.repeat(43);
    const res = await post('/auth/verify', { token });
    expect(res.status).toBe(200);
    expect(await res.json()).toEqual({ valid: false });
  });

  it('returns {valid: false} for suspended user', async () => {
    mockedFind.mockResolvedValue({
      id: '1',
      user_id: '2',
      token_hash: 'hash',
      scopes: ['beta'],
      expires_at: new Date(Date.now() + 86400000).toISOString(),
      revoked_at: null,
      created_at: new Date().toISOString(),
      email: 'test@example.com',
      user_status: 'suspended',
    });
    const token = 'anvil_beta_' + 'A'.repeat(43);
    const res = await post('/auth/verify', { token });
    expect(res.status).toBe(200);
    expect(await res.json()).toEqual({ valid: false });
  });

  it('returns valid response for active token', async () => {
    const expiresAt = new Date(Date.now() + 86400000).toISOString();
    mockedFind.mockResolvedValue({
      id: '1',
      user_id: '2',
      token_hash: 'hash',
      scopes: ['beta'],
      expires_at: expiresAt,
      revoked_at: null,
      created_at: new Date().toISOString(),
      email: 'test@example.com',
      user_status: 'active',
    });
    const token = 'anvil_beta_' + 'A'.repeat(43);
    const res = await post('/auth/verify', { token });
    expect(res.status).toBe(200);
    const body = await res.json();
    expect(body).toEqual(
      expect.objectContaining({
        valid: true,
        user: { email: 'test@example.com' },
        scopes: ['beta'],
        expiresAt,
      })
    );
  });

  it('returns 400 for missing token field', async () => {
    const res = await post('/auth/verify', {});
    expect(res.status).toBe(400);
  });

  it('returns a licence JWT on successful verification', async () => {
    mockedFind.mockResolvedValue({
      id: '1',
      user_id: '2',
      token_hash: 'hash',
      scopes: ['beta'],
      expires_at: new Date(Date.now() + 86400000).toISOString(),
      revoked_at: null,
      created_at: new Date().toISOString(),
      email: 'test@example.com',
      user_status: 'active',
    });
    const res = await app.request('/auth/verify', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ token: 'anvil_beta_' + 'a'.repeat(43) }),
    });
    const json = await res.json();
    expect(json.valid).toBe(true);
    expect(json.license).toBeDefined();
    expect(typeof json.license).toBe('string');
    expect(json.license.split('.').length).toBe(3);
  });
});

describe('POST /auth/license/refresh', () => {
  const mockedFind = vi.mocked(findTokenByHash);

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('returns a fresh licence JWT for a valid token', async () => {
    mockedFind.mockResolvedValue({
      id: '1',
      user_id: '2',
      token_hash: 'hash',
      scopes: ['beta'],
      expires_at: new Date(Date.now() + 86400000).toISOString(),
      revoked_at: null,
      created_at: new Date().toISOString(),
      email: 'test@example.com',
      user_status: 'active',
    });
    const res = await app.request('/auth/license/refresh', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ token: 'anvil_beta_' + 'a'.repeat(43) }),
    });
    const json = await res.json();
    expect(json.license).toBeDefined();
    expect(typeof json.license).toBe('string');
  });

  it('returns valid:false for invalid token format', async () => {
    const res = await post('/auth/license/refresh', { token: 'bad_token' });
    const json = await res.json();
    expect(json.valid).toBe(false);
  });

  it('returns valid:false with reason for missing token', async () => {
    mockedFind.mockResolvedValue(null);
    const token = 'anvil_beta_' + 'A'.repeat(43);
    const res = await post('/auth/license/refresh', { token });
    const json = await res.json();
    expect(json.valid).toBe(false);
    expect(json.reason).toBe('invalid');
  });

  it('returns valid:false with reason:revoked for revoked token', async () => {
    mockedFind.mockResolvedValue({
      id: '1',
      user_id: '2',
      token_hash: 'hash',
      scopes: ['beta'],
      expires_at: new Date(Date.now() + 86400000).toISOString(),
      revoked_at: new Date().toISOString(),
      created_at: new Date().toISOString(),
      email: 'test@example.com',
      user_status: 'active',
    });
    const token = 'anvil_beta_' + 'A'.repeat(43);
    const res = await post('/auth/license/refresh', { token });
    const json = await res.json();
    expect(json.valid).toBe(false);
    expect(json.reason).toBe('revoked');
  });

  it('returns valid:false with reason:invalid for suspended user', async () => {
    mockedFind.mockResolvedValue({
      id: '1',
      user_id: '2',
      token_hash: 'hash',
      scopes: ['beta'],
      expires_at: new Date(Date.now() + 86400000).toISOString(),
      revoked_at: null,
      created_at: new Date().toISOString(),
      email: 'test@example.com',
      user_status: 'suspended',
    });
    const token = 'anvil_beta_' + 'A'.repeat(43);
    const res = await post('/auth/license/refresh', { token });
    const json = await res.json();
    expect(json.valid).toBe(false);
    expect(json.reason).toBe('invalid');
  });

  it('returns valid:false with reason:expired for expired token', async () => {
    mockedFind.mockResolvedValue({
      id: '1',
      user_id: '2',
      token_hash: 'hash',
      scopes: ['beta'],
      expires_at: new Date(Date.now() - 86400000).toISOString(),
      revoked_at: null,
      created_at: new Date().toISOString(),
      email: 'test@example.com',
      user_status: 'active',
    });
    const token = 'anvil_beta_' + 'A'.repeat(43);
    const res = await post('/auth/license/refresh', { token });
    const json = await res.json();
    expect(json.valid).toBe(false);
    expect(json.reason).toBe('expired');
  });
});
