import { describe, it, expect, beforeAll, afterAll } from 'vitest';
import { generateKeyPair, importSPKI, jwtVerify, SignJWT } from 'jose';
import {
  signLicence,
  verifyLicence,
  verifySigningKey,
  verifyVerifyingKey,
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
    plan: 'beta',
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
    const claims = makeClaims({ email: 'josh@eddacraft.ai', plan: 'beta' });
    const jwt = await signLicence(claims);

    const pubKey = await importSPKI(testPublicKeyPem, 'ES256');
    const { payload } = await jwtVerify(jwt, pubKey);

    expect(payload.email).toBe('josh@eddacraft.ai');
    expect(payload.sub).toBe('user_test123');
    expect(payload.org).toBeNull();
    expect(payload.seats).toBe(1);
    expect(payload.iss).toBe('https://api.eddacraft.ai');
    expect(payload.aud).toBe('anvil-cli');
  });

  it('emits `plan` from the account plan (BACT-013 / ADR-121 decision 6)', async () => {
    const jwt = await signLicence(makeClaims({ plan: 'beta' }));
    const pubKey = await importSPKI(testPublicKeyPem, 'ES256');
    const { payload } = await jwtVerify(jwt, pubKey);
    expect(payload['plan']).toBe('beta');
  });

  it('mirrors `plan` onto a `tier` compat alias for apps/docs-shell and apps/docs-site, which still read `tier` directly (OQ-C) — never a second semantic axis', async () => {
    const jwt = await signLicence(makeClaims({ plan: 'beta' }));
    const pubKey = await importSPKI(testPublicKeyPem, 'ES256');
    const { payload } = await jwtVerify(jwt, pubKey);
    expect(payload['tier']).toBe('beta');
    expect(payload['tier']).toBe(payload['plan']);
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

describe('verifyVerifyingKey', () => {
  it('returns ok when LICENSE_PUBLIC_KEY is valid', async () => {
    const result = await verifyVerifyingKey();
    expect(result.ok).toBe(true);
  });

  it('returns error when LICENSE_PUBLIC_KEY is missing', async () => {
    const saved = process.env['LICENSE_PUBLIC_KEY'];
    delete process.env['LICENSE_PUBLIC_KEY'];
    _resetSigningKeyCacheForTests();
    try {
      const result = await verifyVerifyingKey();
      expect(result.ok).toBe(false);
      if (!result.ok) {
        expect(result.error).toMatch(/LICENSE_PUBLIC_KEY/);
      }
    } finally {
      process.env['LICENSE_PUBLIC_KEY'] = saved;
      _resetSigningKeyCacheForTests();
    }
  });
});

describe('verifyLicence', () => {
  it('returns the parsed claims for a freshly signed licence', async () => {
    const jwt = await signLicence(makeClaims({ sub: 'user-42', email: 'a@b.com' }), undefined, 7);
    const claims = await verifyLicence(jwt);
    expect(claims).not.toBeNull();
    expect(claims!.sub).toBe('user-42');
    expect(claims!.email).toBe('a@b.com');
    expect(claims!.scopes).toEqual(['beta']);
  });

  it('returns null for a tampered signature', async () => {
    const jwt = await signLicence(makeClaims());
    // Flip a character in the middle of the signature segment — the last
    // character can be padding-equivalent under base64url and a single-bit
    // flip there may still round-trip.
    const parts = jwt.split('.');
    const sig = parts[2]!;
    const mid = Math.floor(sig.length / 2);
    const swap = sig[mid] === 'A' ? 'B' : 'A';
    parts[2] = sig.slice(0, mid) + swap + sig.slice(mid + 1);
    const tampered = parts.join('.');
    const claims = await verifyLicence(tampered);
    expect(claims).toBeNull();
  });

  it('returns null for a JWT signed by a different key', async () => {
    const { privateKey } = await generateKeyPair('ES256', { extractable: true });
    const otherJwt = await new SignJWT({ email: 'a@b.com', scopes: ['beta'] })
      .setProtectedHeader({ alg: 'ES256' })
      .setSubject('user-other')
      .setIssuedAt()
      .setIssuer('https://api.eddacraft.ai')
      .setAudience('anvil-cli')
      .setExpirationTime(Math.floor(Date.now() / 1000) + 3600)
      .sign(privateKey);
    const claims = await verifyLicence(otherJwt);
    expect(claims).toBeNull();
  });

  it('returns null for a JWT with the wrong issuer', async () => {
    const privateKey = await (await import('jose')).importPKCS8(testPrivateKeyPem, 'ES256');
    const jwt = await new SignJWT({ email: 'a@b.com', scopes: ['beta'] })
      .setProtectedHeader({ alg: 'ES256' })
      .setSubject('user-1')
      .setIssuedAt()
      .setIssuer('https://attacker.example.com')
      .setAudience('anvil-cli')
      .setExpirationTime(Math.floor(Date.now() / 1000) + 3600)
      .sign(privateKey);
    const claims = await verifyLicence(jwt);
    expect(claims).toBeNull();
  });

  it('returns null for a JWT with the wrong audience', async () => {
    const privateKey = await (await import('jose')).importPKCS8(testPrivateKeyPem, 'ES256');
    const jwt = await new SignJWT({ email: 'a@b.com', scopes: ['beta'] })
      .setProtectedHeader({ alg: 'ES256' })
      .setSubject('user-1')
      .setIssuedAt()
      .setIssuer('https://api.eddacraft.ai')
      .setAudience('some-other-audience')
      .setExpirationTime(Math.floor(Date.now() / 1000) + 3600)
      .sign(privateKey);
    const claims = await verifyLicence(jwt);
    expect(claims).toBeNull();
  });

  it('returns null for an expired JWT', async () => {
    const privateKey = await (await import('jose')).importPKCS8(testPrivateKeyPem, 'ES256');
    const jwt = await new SignJWT({ email: 'a@b.com', scopes: ['beta'] })
      .setProtectedHeader({ alg: 'ES256' })
      .setSubject('user-1')
      .setIssuedAt(Math.floor(Date.now() / 1000) - 7200)
      .setIssuer('https://api.eddacraft.ai')
      .setAudience('anvil-cli')
      .setExpirationTime(Math.floor(Date.now() / 1000) - 3600)
      .sign(privateKey);
    const claims = await verifyLicence(jwt);
    expect(claims).toBeNull();
  });

  it('returns null for a JWT missing required claims (no scopes)', async () => {
    const privateKey = await (await import('jose')).importPKCS8(testPrivateKeyPem, 'ES256');
    // No `scopes` claim.
    const jwt = await new SignJWT({ email: 'a@b.com' })
      .setProtectedHeader({ alg: 'ES256' })
      .setSubject('user-1')
      .setIssuedAt()
      .setIssuer('https://api.eddacraft.ai')
      .setAudience('anvil-cli')
      .setExpirationTime(Math.floor(Date.now() / 1000) + 3600)
      .sign(privateKey);
    const claims = await verifyLicence(jwt);
    expect(claims).toBeNull();
  });

  it('throws when LICENSE_PUBLIC_KEY is missing — callers must distinguish config errors from verification failures', async () => {
    const jwt = await signLicence(makeClaims());
    const saved = process.env['LICENSE_PUBLIC_KEY'];
    delete process.env['LICENSE_PUBLIC_KEY'];
    _resetSigningKeyCacheForTests();
    try {
      await expect(verifyLicence(jwt)).rejects.toThrow('LICENSE_PUBLIC_KEY');
    } finally {
      process.env['LICENSE_PUBLIC_KEY'] = saved;
      _resetSigningKeyCacheForTests();
    }
  });

  it('returns null for a malformed JWT string', async () => {
    expect(await verifyLicence('not.a.jwt')).toBeNull();
    expect(await verifyLicence('')).toBeNull();
  });

  describe('plan claim (BACT-013 / OQ-C)', () => {
    it('returns `plan` from a freshly signed licence', async () => {
      const jwt = await signLicence(makeClaims({ plan: 'beta' }));
      const claims = await verifyLicence(jwt);
      expect(claims?.plan).toBe('beta');
    });

    it('SEC-012: downgrades a pre-BACT-013 licence to `beta` — a stale `tier` is never promoted into `plan`', async () => {
      const privateKey = await (await import('jose')).importPKCS8(testPrivateKeyPem, 'ES256');
      // Pre-BACT-013 shape: only `tier`, no `plan` claim at all.
      const legacyJwt = await new SignJWT({
        email: 'legacy@example.com',
        identity: { provider: 'github', id: null },
        org: null,
        tier: 'pro',
        scopes: ['beta'],
        seats: 1,
      })
        .setProtectedHeader({ alg: 'ES256' })
        .setSubject('user-legacy')
        .setIssuedAt()
        .setIssuer('https://api.eddacraft.ai')
        .setAudience('anvil-cli')
        .setExpirationTime(Math.floor(Date.now() / 1000) + 3600)
        .sign(privateKey);

      const claims = await verifyLicence(legacyJwt);
      // The licence still authenticates — this is a de-escalation, not a
      // rejection, so no pre-BACT-013 user is forced to re-authenticate.
      expect(claims).not.toBeNull();
      // ...but its `tier: 'pro'` is NOT inherited. No account holds `pro`
      // (beta_users.plan CHECK admits only 'beta'), and both edge verifiers
      // read this claim with no DB round trip, so promoting it would let a
      // stale token assert a plan nobody has.
      expect(claims?.plan).toBe('beta');
    });

    it('prefers `plan` over `tier` when a licence somehow carries both (dual-write window)', async () => {
      const privateKey = await (await import('jose')).importPKCS8(testPrivateKeyPem, 'ES256');
      const jwt = await new SignJWT({
        email: 'dual@example.com',
        identity: { provider: 'github', id: null },
        org: null,
        plan: 'beta',
        tier: 'pro',
        scopes: ['beta'],
        seats: 1,
      })
        .setProtectedHeader({ alg: 'ES256' })
        .setSubject('user-dual')
        .setIssuedAt()
        .setIssuer('https://api.eddacraft.ai')
        .setAudience('anvil-cli')
        .setExpirationTime(Math.floor(Date.now() / 1000) + 3600)
        .sign(privateKey);

      const claims = await verifyLicence(jwt);
      expect(claims?.plan).toBe('beta');
    });

    it('SEC-012: resolves `plan` to null when neither claim is present — fails closed, no permissive default', async () => {
      const privateKey = await (await import('jose')).importPKCS8(testPrivateKeyPem, 'ES256');
      const jwt = await new SignJWT({
        email: 'noplan@example.com',
        scopes: ['beta'],
      })
        .setProtectedHeader({ alg: 'ES256' })
        .setSubject('user-noplan')
        .setIssuedAt()
        .setIssuer('https://api.eddacraft.ai')
        .setAudience('anvil-cli')
        .setExpirationTime(Math.floor(Date.now() / 1000) + 3600)
        .sign(privateKey);

      const claims = await verifyLicence(jwt);
      // Identity still verifies — plan is an entitlement axis, not an
      // authentication one, so a claimless licence must not 401 the caller.
      expect(claims).not.toBeNull();
      // No live licence lacks both claims (`tier` has been minted since
      // 2026-03-12 and the TTL is 90 days), so failing closed here costs
      // nothing today and denies by default the moment plans differentiate.
      expect(claims?.plan).toBeNull();
    });

    it('SEC-012: uses the `plan` claim verbatim when present, whatever the alias says', async () => {
      const privateKey = await (await import('jose')).importPKCS8(testPrivateKeyPem, 'ES256');
      const jwt = await new SignJWT({
        email: 'planned@example.com',
        identity: { provider: 'github', id: null },
        org: null,
        plan: 'enterprise',
        tier: 'beta',
        scopes: ['beta'],
        seats: 1,
      })
        .setProtectedHeader({ alg: 'ES256' })
        .setSubject('user-planned')
        .setIssuedAt()
        .setIssuer('https://api.eddacraft.ai')
        .setAudience('anvil-cli')
        .setExpirationTime(Math.floor(Date.now() / 1000) + 3600)
        .sign(privateKey);

      const claims = await verifyLicence(jwt);
      expect(claims?.plan).toBe('enterprise');
    });

    it('SEC-012: denies a `tier` value we never minted rather than de-escalating it to beta', async () => {
      const privateKey = await (await import('jose')).importPKCS8(testPrivateKeyPem, 'ES256');
      // `tier: 'free'` was never issued by any signLicence. Treating "any
      // tier" as beta would *elevate* it; only the real legacy value ('pro')
      // de-escalates.
      const jwt = await new SignJWT({
        email: 'freetier@example.com',
        identity: { provider: 'github', id: null },
        org: null,
        tier: 'free',
        scopes: ['beta'],
        seats: 1,
      })
        .setProtectedHeader({ alg: 'ES256' })
        .setSubject('user-freetier')
        .setIssuedAt()
        .setIssuer('https://api.eddacraft.ai')
        .setAudience('anvil-cli')
        .setExpirationTime(Math.floor(Date.now() / 1000) + 3600)
        .sign(privateKey);

      const claims = await verifyLicence(jwt);
      expect(claims?.plan).toBeNull();
    });

    it('SEC-012: ignores a non-string `plan` claim rather than coercing it', async () => {
      const privateKey = await (await import('jose')).importPKCS8(testPrivateKeyPem, 'ES256');
      const jwt = await new SignJWT({
        email: 'weird@example.com',
        identity: { provider: 'github', id: null },
        org: null,
        plan: { name: 'beta' },
        scopes: ['beta'],
        seats: 1,
      })
        .setProtectedHeader({ alg: 'ES256' })
        .setSubject('user-weird')
        .setIssuedAt()
        .setIssuer('https://api.eddacraft.ai')
        .setAudience('anvil-cli')
        .setExpirationTime(Math.floor(Date.now() / 1000) + 3600)
        .sign(privateKey);

      const claims = await verifyLicence(jwt);
      expect(claims?.plan).toBeNull();
    });
  });
});
