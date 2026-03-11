import { describe, it, expect, beforeAll } from 'vitest';
import { generateKeyPair, exportPKCS8, exportSPKI, SignJWT, importPKCS8 } from 'jose';
import { verifyLicence, setPublicKeys } from '../licence-verifier.js';

let testPrivateKeyPem: string;
let testPublicKeyPem: string;

async function signTestJwt(
  claims: Record<string, unknown>,
  options: { kid?: string; exp?: number } = {}
): Promise<string> {
  const privateKey = await importPKCS8(testPrivateKeyPem, 'ES256');
  const now = Math.floor(Date.now() / 1000);

  const builder = new SignJWT(claims)
    .setProtectedHeader({ alg: 'ES256', kid: options.kid ?? '2026-03' })
    .setSubject((claims.sub as string) ?? 'user_test')
    .setIssuedAt(now)
    .setExpirationTime(options.exp ?? now + 86400);

  return builder.sign(privateKey);
}

beforeAll(async () => {
  const { privateKey, publicKey } = await generateKeyPair('ES256', { extractable: true });
  testPrivateKeyPem = await exportPKCS8(privateKey);
  testPublicKeyPem = await exportSPKI(publicKey);

  setPublicKeys({ '2026-03': testPublicKeyPem });
});

describe('verifyLicence', () => {
  it('returns valid result for a correctly signed JWT', async () => {
    const jwt = await signTestJwt({
      email: 'test@example.com',
      tier: 'pro',
      org: null,
      rcAfter: Math.floor(Date.now() / 1000) + 86400,
    });

    const result = await verifyLicence(jwt);
    expect(result.valid).toBe(true);
    expect(result.claims?.email).toBe('test@example.com');
    expect(result.claims?.tier).toBe('pro');
    expect(result.needsRefresh).toBe(false);
  });

  it('returns needsRefresh when rcAfter has passed', async () => {
    const jwt = await signTestJwt({
      email: 'test@example.com',
      tier: 'pro',
      rcAfter: Math.floor(Date.now() / 1000) - 100,
    });

    const result = await verifyLicence(jwt);
    expect(result.valid).toBe(true);
    expect(result.needsRefresh).toBe(true);
  });

  it('returns invalid for an expired JWT', async () => {
    const jwt = await signTestJwt(
      { email: 'test@example.com', tier: 'pro', rcAfter: 0 },
      { exp: Math.floor(Date.now() / 1000) - 100 }
    );

    const result = await verifyLicence(jwt);
    expect(result.valid).toBe(false);
    expect(result.reason).toBe('expired');
  });

  it('returns invalid for a tampered JWT', async () => {
    const jwt = await signTestJwt({ email: 'test@example.com', tier: 'pro', rcAfter: 0 });
    const tampered = jwt.slice(0, -5) + 'XXXXX';

    const result = await verifyLicence(tampered);
    expect(result.valid).toBe(false);
    expect(result.reason).toBe('invalid_signature');
  });

  it('returns invalid for a JWT signed with an unknown kid', async () => {
    const jwt = await signTestJwt(
      { email: 'test@example.com', tier: 'pro', rcAfter: 0 },
      { kid: 'unknown-key' }
    );

    const result = await verifyLicence(jwt);
    expect(result.valid).toBe(false);
    expect(result.reason).toBe('unknown_key');
  });

  it('returns invalid for garbage input', async () => {
    const result = await verifyLicence('not-a-jwt');
    expect(result.valid).toBe(false);
  });
});
