import { describe, it, expect, beforeAll, afterAll } from 'vitest';
import { generateKeyPair, importSPKI, jwtVerify } from 'jose';
import {
  signLicence,
  verifySigningKey,
  _resetSigningKeyCacheForTests,
  type LicenceClaims,
} from '../licence.js';

let originalSigningKey: string | undefined;
let originalPublicKey: string | undefined;
let testPrivateKeyPem: string;
let testPublicKeyPem: string;

beforeAll(async () => {
  originalSigningKey = process.env['LICENSE_SIGNING_KEY'];
  originalPublicKey = process.env['LICENSE_PUBLIC_KEY'];

  const { privateKey, publicKey } = await generateKeyPair('ES256', { extractable: true });
  const { exportPKCS8 } = await import('jose');
  const { exportSPKI } = await import('jose');
  testPrivateKeyPem = await exportPKCS8(privateKey);
  testPublicKeyPem = await exportSPKI(publicKey);

  process.env['LICENSE_SIGNING_KEY'] = testPrivateKeyPem;
  process.env['LICENSE_PUBLIC_KEY'] = testPublicKeyPem;
});

afterAll(() => {
  if (originalSigningKey === undefined) delete process.env['LICENSE_SIGNING_KEY'];
  else process.env['LICENSE_SIGNING_KEY'] = originalSigningKey;
  if (originalPublicKey === undefined) delete process.env['LICENSE_PUBLIC_KEY'];
  else process.env['LICENSE_PUBLIC_KEY'] = originalPublicKey;
});

function makeClaims(overrides: Partial<LicenceClaims> = {}): LicenceClaims {
  return {
    sub: 'user_test123',
    email: 'test@example.com',
    identity: { provider: 'github', id: null },
    org: null,
    tier: 'pro',
    scopes: ['beta'],
    seats: 1,
    ...overrides,
  };
}

describe('signLicence', () => {
  it('returns a signed JWT string', async () => {
    const jwt = await signLicence(makeClaims());
    expect(typeof jwt).toBe('string');
    expect(jwt.split('.').length).toBe(3);
  });

  it('includes correct claims in the payload', async () => {
    const claims = makeClaims({ email: 'josh@eddacraft.ai', tier: 'pro' });
    const jwt = await signLicence(claims);

    const pubKey = await importSPKI(testPublicKeyPem, 'ES256');
    const { payload } = await jwtVerify(jwt, pubKey);

    expect(payload.email).toBe('josh@eddacraft.ai');
    expect(payload.tier).toBe('pro');
    expect(payload.sub).toBe('user_test123');
    expect(payload.org).toBeNull();
    expect(payload.seats).toBe(1);
    expect(payload.iss).toBe('https://api.eddacraft.ai');
    expect(payload.aud).toBe('anvil-cli');
  });

  it('sets exp to 90 days from now', async () => {
    const before = Math.floor(Date.now() / 1000);
    const jwt = await signLicence(makeClaims());
    const after = Math.floor(Date.now() / 1000);

    const pubKey = await importSPKI(testPublicKeyPem, 'ES256');
    const { payload } = await jwtVerify(jwt, pubKey);

    const ninetyDays = 90 * 24 * 60 * 60;
    expect(payload.exp).toBeGreaterThanOrEqual(before + ninetyDays);
    expect(payload.exp).toBeLessThanOrEqual(after + ninetyDays);
  });

  it('sets rcAfter to 7 days from now', async () => {
    const before = Math.floor(Date.now() / 1000);
    const jwt = await signLicence(makeClaims());
    const after = Math.floor(Date.now() / 1000);

    const pubKey = await importSPKI(testPublicKeyPem, 'ES256');
    const { payload } = await jwtVerify(jwt, pubKey);

    const sevenDays = 7 * 24 * 60 * 60;
    expect(payload['rcAfter']).toBeGreaterThanOrEqual(before + sevenDays);
    expect(payload['rcAfter']).toBeLessThanOrEqual(after + sevenDays);
  });

  it('sets kid header to identify the key version', async () => {
    const jwt = await signLicence(makeClaims());
    const headerB64 = jwt.split('.')[0];
    const header = JSON.parse(Buffer.from(headerB64, 'base64url').toString());
    expect(header.kid).toBeDefined();
    expect(header.alg).toBe('ES256');
  });

  it('throws when tokenExpiresAt is an invalid date string', async () => {
    await expect(signLicence(makeClaims(), 'not-a-date' as unknown as string)).rejects.toThrow(
      'Invalid tokenExpiresAt'
    );
  });

  it('uses custom ttlDays when provided', async () => {
    const before = Math.floor(Date.now() / 1000);
    const jwt = await signLicence(makeClaims(), undefined, 7);
    const after = Math.floor(Date.now() / 1000);

    const pubKey = await importSPKI(testPublicKeyPem, 'ES256');
    const { payload } = await jwtVerify(jwt, pubKey);

    const sevenDays = 7 * 24 * 60 * 60;
    expect(payload.exp).toBeGreaterThanOrEqual(before + sevenDays);
    expect(payload.exp).toBeLessThanOrEqual(after + sevenDays);
  });

  it('defaults to 90-day TTL when ttlDays not provided', async () => {
    const before = Math.floor(Date.now() / 1000);
    const jwt = await signLicence(makeClaims());
    const after = Math.floor(Date.now() / 1000);

    const pubKey = await importSPKI(testPublicKeyPem, 'ES256');
    const { payload } = await jwtVerify(jwt, pubKey);

    const ninetyDays = 90 * 24 * 60 * 60;
    expect(payload.exp).toBeGreaterThanOrEqual(before + ninetyDays);
    expect(payload.exp).toBeLessThanOrEqual(after + ninetyDays);
  });

  it('ttlDays caps tokenExpiresAt', async () => {
    const before = Math.floor(Date.now() / 1000);
    const farFuture = new Date(Date.now() + 365 * 24 * 60 * 60 * 1000);
    const jwt = await signLicence(makeClaims(), farFuture, 7);
    const after = Math.floor(Date.now() / 1000);

    const pubKey = await importSPKI(testPublicKeyPem, 'ES256');
    const { payload } = await jwtVerify(jwt, pubKey);

    const sevenDays = 7 * 24 * 60 * 60;
    expect(payload.exp).toBeGreaterThanOrEqual(before + sevenDays);
    expect(payload.exp).toBeLessThanOrEqual(after + sevenDays);
  });

  it('throws if LICENSE_SIGNING_KEY is not set', async () => {
    const saved = process.env['LICENSE_SIGNING_KEY'];
    delete process.env['LICENSE_SIGNING_KEY'];
    _resetSigningKeyCacheForTests();
    try {
      await expect(signLicence(makeClaims())).rejects.toThrow('LICENSE_SIGNING_KEY');
    } finally {
      process.env['LICENSE_SIGNING_KEY'] = saved;
      _resetSigningKeyCacheForTests();
    }
  });
});

describe('verifySigningKey', () => {
  it('returns ok when LICENSE_SIGNING_KEY is valid', async () => {
    const result = await verifySigningKey();
    expect(result.ok).toBe(true);
  });

  it('returns error when LICENSE_SIGNING_KEY is missing', async () => {
    const saved = process.env['LICENSE_SIGNING_KEY'];
    delete process.env['LICENSE_SIGNING_KEY'];
    _resetSigningKeyCacheForTests();
    try {
      const result = await verifySigningKey();
      expect(result.ok).toBe(false);
      if (!result.ok) {
        expect(result.error).toMatch(/LICENSE_SIGNING_KEY/);
      }
    } finally {
      process.env['LICENSE_SIGNING_KEY'] = saved;
      _resetSigningKeyCacheForTests();
    }
  });
});
