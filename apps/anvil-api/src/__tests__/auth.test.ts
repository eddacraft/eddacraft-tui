import { describe, it, expect, vi, beforeEach, afterEach, beforeAll, afterAll } from 'vitest';
import { generateKeyPair, exportPKCS8, exportSPKI } from 'jose';
import { Hono } from 'hono';
import { auth } from '../routes/auth.js';
import { signLicence, _resetSigningKeyCacheForTests } from '../lib/licence.js';

// Mock the db client
vi.mock('../db/client.js', () => ({
  getClient: vi.fn(),
}));

// Mock queries
vi.mock('../db/queries.js', () => ({
  findTokenByHash: vi.fn(),
  findActiveScopesForUser: vi.fn().mockResolvedValue(['beta']),
  findUserById: vi.fn(),
}));

// Mock token utilities (keep real implementations for format validation)
vi.mock('../lib/token.js', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../lib/token.js')>();
  return {
    ...actual,
    hashToken: vi.fn().mockReturnValue('mocked-hash'),
  };
});

import { findTokenByHash, findUserById } from '../db/queries.js';

let originalSigningKey: string | undefined;
let originalPublicKey: string | undefined;

beforeAll(async () => {
  originalSigningKey = process.env['LICENSE_SIGNING_KEY'];
  originalPublicKey = process.env['LICENSE_PUBLIC_KEY'];
  const { privateKey, publicKey } = await generateKeyPair('ES256', { extractable: true });
  process.env['LICENSE_SIGNING_KEY'] = await exportPKCS8(privateKey);
  process.env['LICENSE_PUBLIC_KEY'] = await exportSPKI(publicKey);
  _resetSigningKeyCacheForTests();
});

afterAll(() => {
  if (originalSigningKey === undefined) delete process.env['LICENSE_SIGNING_KEY'];
  else process.env['LICENSE_SIGNING_KEY'] = originalSigningKey;
  if (originalPublicKey === undefined) delete process.env['LICENSE_PUBLIC_KEY'];
  else process.env['LICENSE_PUBLIC_KEY'] = originalPublicKey;
  _resetSigningKeyCacheForTests();
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

/** Decodes a JWT payload without verifying the signature — test-only. */
function decodeJwtPayload(jwt: string): Record<string, unknown> {
  const payloadB64 = jwt.split('.')[1] ?? '';
  return JSON.parse(Buffer.from(payloadB64, 'base64url').toString());
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
        user: { email: 'test@example.com', plan: 'beta' },
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

  it('signs the licence with the joined account plan claim (BACT-013)', async () => {
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
      plan: 'beta',
    });
    const res = await post('/auth/verify', { token: 'anvil_beta_' + 'a'.repeat(43) });
    const json = await res.json();
    expect(json.user).toEqual({ email: 'test@example.com', plan: 'beta' });
    const payload = decodeJwtPayload(json.license);
    expect(payload['plan']).toBe('beta');
    expect(payload['tier']).toBe('beta');
  });

  it('defaults the licence plan claim to `beta` when the joined row has no plan (fixture tolerance)', async () => {
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
    const res = await post('/auth/verify', { token: 'anvil_beta_' + 'a'.repeat(43) });
    const json = await res.json();
    const payload = decodeJwtPayload(json.license);
    expect(payload['plan']).toBe('beta');
  });
});

describe('POST /auth/verify — licence JWT credential (CIB-066)', () => {
  const mockedFindUser = vi.mocked(findUserById);

  beforeEach(() => {
    vi.clearAllMocks();
  });

  function activeUser(overrides: Record<string, unknown> = {}) {
    return {
      id: 'user-1',
      email: 'dev@example.com',
      name: null,
      notes: null,
      status: 'active',
      plan: 'beta',
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString(),
      ...overrides,
    };
  }

  function mintLicence(expiresAt?: string, plan = 'pro'): Promise<string> {
    return signLicence(
      {
        sub: 'user-1',
        email: 'dev@example.com',
        identity: { provider: 'github', id: '12345' },
        org: null,
        plan,
        scopes: ['beta'],
        seats: 1,
      },
      expiresAt ?? new Date(Date.now() + 7 * 24 * 60 * 60 * 1000).toISOString()
    );
  }

  it('accepts a valid licence for an active user and reports the identity', async () => {
    mockedFindUser.mockResolvedValue(activeUser() as never);
    const licence = await mintLicence();
    // A real licence is far beyond the old max(200) schema cap — this also
    // guards the schema widening that lets licences reach verification.
    expect(licence.length).toBeGreaterThan(200);

    const res = await post('/auth/verify', { token: licence });

    expect(res.status).toBe(200);
    const body = await res.json();
    expect(body.valid).toBe(true);
    // BACT-013: the DB row's plan ('beta', from `activeUser()`) wins over
    // the licence claim's plan ('pro', from `mintLicence()`'s default) — see
    // the freshness test immediately below for the explicit regression pin.
    expect(body.user).toEqual({ email: 'dev@example.com', plan: 'beta' });
    expect(body.scopes).toEqual(['beta']);
    expect(body.isEdict).toBe(false);
    expect(mockedFindUser).toHaveBeenCalledWith(undefined, 'user-1');
  });

  it('reports the fresh account plan from the DB row, not the (possibly stale) licence claim (BACT-013)', async () => {
    // The claim says 'pro' (a claim minted before the account's plan
    // changed, or before BACT-013); the DB row is the authority.
    mockedFindUser.mockResolvedValue(activeUser({ plan: 'beta' }) as never);
    const licence = await mintLicence(undefined, 'pro');

    const res = await post('/auth/verify', { token: licence });
    const body = await res.json();
    expect(body.user).toEqual({ email: 'dev@example.com', plan: 'beta' });
  });

  it('falls back to the licence plan claim when the DB row has no plan (fixture tolerance)', async () => {
    const { plan: _plan, ...userWithoutPlan } = activeUser({ plan: 'beta' });
    mockedFindUser.mockResolvedValue(userWithoutPlan as never);
    const licence = await mintLicence(undefined, 'legacy-claim');

    const res = await post('/auth/verify', { token: licence });
    const body = await res.json();
    expect(body.user).toEqual({ email: 'dev@example.com', plan: 'legacy-claim' });
  });

  it('rejects a licence whose subject is no longer active', async () => {
    mockedFindUser.mockResolvedValue({ ...activeUser(), status: 'suspended' } as never);
    const res = await post('/auth/verify', { token: await mintLicence() });
    expect(res.status).toBe(200);
    expect(await res.json()).toEqual({ valid: false });
  });

  it('rejects a licence whose subject no longer exists', async () => {
    mockedFindUser.mockResolvedValue(null as never);
    const res = await post('/auth/verify', { token: await mintLicence() });
    expect(await res.json()).toEqual({ valid: false });
  });

  it('rejects an expired licence', async () => {
    mockedFindUser.mockResolvedValue(activeUser() as never);
    const expired = await mintLicence(new Date(Date.now() - 60_000).toISOString());
    const res = await post('/auth/verify', { token: expired });
    expect(await res.json()).toEqual({ valid: false });
    expect(mockedFindUser).not.toHaveBeenCalled();
  });

  it('rejects a tampered licence', async () => {
    mockedFindUser.mockResolvedValue(activeUser() as never);
    const licence = await mintLicence();
    // Flip a char WELL INSIDE the signature: the final base64url char of an
    // ES256 signature carries 2 data bits + 4 discarded padding bits, so a
    // last-char flip can decode to the identical signature (~25% flake).
    const i = licence.length - 10;
    const flipped = licence[i] === 'A' ? 'B' : 'A';
    const sigFlip = licence.slice(0, i) + flipped + licence.slice(i + 1);
    const res = await post('/auth/verify', { token: sigFlip });
    expect(await res.json()).toEqual({ valid: false });
    expect(mockedFindUser).not.toHaveBeenCalled();
  });

  it('rejects a string that is neither an access token nor a JWT', async () => {
    const res = await post('/auth/verify', { token: 'not-a-credential' });
    expect(res.status).toBe(200);
    expect(await res.json()).toEqual({ valid: false });
  });

  it('rejects non-JWT garbage without touching the verifying key (no spurious 503)', async () => {
    const saved = process.env['LICENSE_PUBLIC_KEY'];
    delete process.env['LICENSE_PUBLIC_KEY'];
    _resetSigningKeyCacheForTests();
    try {
      const res = await post('/auth/verify', { token: 'not-a-real-token' });
      expect(res.status).toBe(200);
      expect(await res.json()).toEqual({ valid: false });
    } finally {
      process.env['LICENSE_PUBLIC_KEY'] = saved;
      _resetSigningKeyCacheForTests();
    }
  });

  it('returns 503 when the verifying key is unavailable (misconfiguration, not invalid creds)', async () => {
    const saved = process.env['LICENSE_PUBLIC_KEY'];
    delete process.env['LICENSE_PUBLIC_KEY'];
    _resetSigningKeyCacheForTests();
    try {
      const res = await post('/auth/verify', { token: await mintLicence() });
      expect(res.status).toBe(503);
      expect(await res.json()).toEqual({ error: 'verification_unavailable' });
    } finally {
      process.env['LICENSE_PUBLIC_KEY'] = saved;
      _resetSigningKeyCacheForTests();
    }
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

  it('signs the refreshed licence with the joined account plan claim (BACT-013)', async () => {
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
      plan: 'beta',
    });
    const res = await post('/auth/license/refresh', { token: 'anvil_beta_' + 'a'.repeat(43) });
    const json = await res.json();
    const payload = decodeJwtPayload(json.license);
    expect(payload['plan']).toBe('beta');
    expect(payload['tier']).toBe('beta');
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
