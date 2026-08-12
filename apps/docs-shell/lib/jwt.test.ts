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

/** Matches anvil-api signLicence issuer/audience (apps/anvil-api/src/lib/licence.ts). */
const LICENCE_ISSUER = 'https://api.eddacraft.ai';
const LICENCE_AUDIENCE = 'anvil-cli';

async function signToken(
  claims: Record<string, unknown>,
  expSeconds: number = 3600,
  options: { issuer?: string; audience?: string; subject?: string | null } = {}
): Promise<string> {
  const privateKey = await importPKCS8(TEST_PRIVATE_KEY_PEM, 'ES256');
  // Keep `sub` out of the constructor payload so `options.subject === null`
  // truly omits the claim even when callers also pass `sub` in `claims`.
  const { sub: claimsSub, ...payloadClaims } = claims;
  let builder = new SignJWT(payloadClaims)
    .setProtectedHeader({ alg: 'ES256' })
    .setIssuedAt()
    .setExpirationTime(Math.floor(Date.now() / 1000) + expSeconds);

  const issuer = options.issuer === undefined ? LICENCE_ISSUER : options.issuer;
  if (issuer) builder = builder.setIssuer(issuer);

  const audience = options.audience === undefined ? LICENCE_AUDIENCE : options.audience;
  if (audience) builder = builder.setAudience(audience);

  if (options.subject === null) {
    // deliberately omit sub (payloadClaims already stripped)
  } else if (options.subject !== undefined) {
    builder = builder.setSubject(options.subject);
  } else if (typeof claimsSub === 'string') {
    builder = builder.setSubject(claimsSub);
  }

  return builder.sign(privateKey);
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
    const token = await signToken({ sub: 'user@example.com', tier: 'beta' }, -60);
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

  it('rejects a signed token with unexpected issuer', async () => {
    const token = await signToken({ sub: 'user@example.com', tier: 'beta' }, 3600, {
      issuer: 'https://attacker.example.com',
    });
    const result = await verifyLicense(token);
    expect(result.valid).toBe(false);
  });

  it('rejects a signed token with unexpected audience', async () => {
    const token = await signToken({ sub: 'user@example.com', tier: 'beta' }, 3600, {
      audience: 'some-other-audience',
    });
    const result = await verifyLicense(token);
    expect(result.valid).toBe(false);
  });

  it('rejects a signed token missing subject', async () => {
    const token = await signToken({ tier: 'beta' }, 3600, { subject: null });
    const result = await verifyLicense(token);
    expect(result.valid).toBe(false);
  });

  it('rejects a signed token with empty subject', async () => {
    const token = await signToken({ tier: 'beta' }, 3600, { subject: '' });
    const result = await verifyLicense(token);
    expect(result.valid).toBe(false);
  });
});
