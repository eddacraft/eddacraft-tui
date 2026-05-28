import { describe, it, expect, beforeAll } from 'vitest';
import { SignJWT, importPKCS8 } from 'jose';
import { verifyLicense, resetKeyCache } from './jwt';

const TEST_PUBLIC_KEY_PEM = `-----BEGIN PUBLIC KEY-----
MFkwEwYHKoZIzj0CAQYIKoZIzj0DAQcDQgAEJBBvPQBkWNKD9mb6JqYMmoaUg8+e
/SCR2JLkHkbyDsplOGtZ9weVkaOZqBY+/BpvI/CUUroMrrLtCbZLAAH1DQ==
-----END PUBLIC KEY-----`;

const TEST_PRIVATE_KEY_PEM = `-----BEGIN PRIVATE KEY-----
MIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQgdo4R+xyoyB8SDaqT
VMjMeNt4PRYgGRjF0/1UL4WT8Z+hRANCAAQkEG89AGRY0oP2ZvompgyahpSDz579
IJHYkuQeRvIOymU4a1n3B5WRo5moFj78Gm8j8JRSugyusu0JtksAAfUN
-----END PRIVATE KEY-----`;

async function signToken(
  claims: Record<string, unknown>,
  expSeconds: number = 3600
): Promise<string> {
  const privateKey = await importPKCS8(TEST_PRIVATE_KEY_PEM, 'ES256');
  return new SignJWT(claims)
    .setProtectedHeader({ alg: 'ES256' })
    .setIssuedAt()
    .setExpirationTime(Math.floor(Date.now() / 1000) + expSeconds)
    .sign(privateKey);
}

describe('verifyLicense', () => {
  beforeAll(() => {
    process.env.LICENSE_PUBLIC_KEY = TEST_PUBLIC_KEY_PEM;
    resetKeyCache();
  });

  it('verifies a valid token', async () => {
    const token = await signToken({ sub: 'user@example.com', tier: 'beta' });
    const result = await verifyLicense(token);
    expect(result.valid).toBe(true);
  });

  it('rejects a valid token without docs entitlement', async () => {
    const token = await signToken({ sub: 'user@example.com' });
    const result = await verifyLicense(token);
    expect(result.valid).toBe(false);
  });

  it('rejects a valid token with a non-entitled tier', async () => {
    const token = await signToken({ sub: 'user@example.com', tier: 'free' });
    const result = await verifyLicense(token);
    expect(result.valid).toBe(false);
  });

  it('rejects an expired token', async () => {
    const token = await signToken({ sub: 'user@example.com' }, -60);
    const result = await verifyLicense(token);
    expect(result.valid).toBe(false);
  });

  it('rejects a garbled token', async () => {
    const result = await verifyLicense('not.a.jwt');
    expect(result.valid).toBe(false);
  });

  it('rejects a token signed with a different key', async () => {
    const token = 'eyJhbGciOiJFUzI1NiJ9.eyJzdWIiOiJmb28ifQ.signature';
    const result = await verifyLicense(token);
    expect(result.valid).toBe(false);
  });
});
