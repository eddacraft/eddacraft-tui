import { describe, it, expect, beforeAll, beforeEach } from 'vitest';
import { SignJWT, importPKCS8 } from 'jose';
import middleware from './middleware';
import { resetKeyCache } from './lib/jwt';

const TEST_PUBLIC_KEY_PEM = `-----BEGIN PUBLIC KEY-----
MFkwEwYHKoZIzj0CAQYIKoZIzj0DAQcDQgAEJBBvPQBkWNKD9mb6JqYMmoaUg8+e
/SCR2JLkHkbyDsplOGtZ9weVkaOZqBY+/BpvI/CUUroMrrLtCbZLAAH1DQ==
-----END PUBLIC KEY-----`;

const TEST_PRIVATE_KEY_PEM = `-----BEGIN PRIVATE KEY-----
MIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQgdo4R+xyoyB8SDaqT
VMjMeNt4PRYgGRjF0/1UL4WT8Z+hRANCAAQkEG89AGRY0oP2ZvompgyahpSDz579
IJHYkuQeRvIOymU4a1n3B5WRo5moFj78Gm8j8JRSugyusu0JtksAAfUN
-----END PRIVATE KEY-----`;

async function signToken(expSecondsFromNow: number = 3600): Promise<string> {
  const privateKey = await importPKCS8(TEST_PRIVATE_KEY_PEM, 'ES256');
  return new SignJWT({ sub: 'test@example.com' })
    .setProtectedHeader({ alg: 'ES256' })
    .setIssuedAt()
    .setExpirationTime(Math.floor(Date.now() / 1000) + expSecondsFromNow)
    .sign(privateKey);
}

function makeRequest(url: string, cookies: Record<string, string> = {}): Request {
  const cookieHeader = Object.entries(cookies)
    .map(([k, v]) => `${k}=${v}`)
    .join('; ');
  return new Request(url, {
    headers: cookieHeader ? { cookie: cookieHeader } : {},
  });
}

describe('middleware', () => {
  beforeAll(() => {
    process.env.LICENSE_PUBLIC_KEY = TEST_PUBLIC_KEY_PEM;
  });

  beforeEach(() => {
    resetKeyCache();
  });

  it('redirects to login when no cookie', async () => {
    const req = makeRequest('https://docs.eddacraft.ai/anvil/overview');
    const res = await middleware(req as never);
    expect(res.status).toBe(302);
    const location = res.headers.get('location')!;
    expect(location).toContain('/auth/login');
    expect(location).toContain('next=%2Fanvil%2Foverview');
  });

  it('passes through with a valid cookie', async () => {
    const token = await signToken();
    const req = makeRequest('https://docs.eddacraft.ai/anvil/overview', {
      'anvil-docs-session': token,
    });
    const res = await middleware(req as never);
    expect(res.status).not.toBe(302);
  });

  it('redirects and clears cookie when token is expired', async () => {
    const token = await signToken(-60);
    const req = makeRequest('https://docs.eddacraft.ai/anvil/overview', {
      'anvil-docs-session': token,
    });
    const res = await middleware(req as never);
    expect(res.status).toBe(302);
    const setCookie = res.headers.get('set-cookie') ?? '';
    expect(setCookie).toContain('anvil-docs-session=');
    expect(setCookie).toContain('Max-Age=0');
  });

  it('redirects when token is garbage', async () => {
    const req = makeRequest('https://docs.eddacraft.ai/anvil/overview', {
      'anvil-docs-session': 'not.a.jwt',
    });
    const res = await middleware(req as never);
    expect(res.status).toBe(302);
  });

  it('preserves deep path in next param', async () => {
    const req = makeRequest('https://docs.eddacraft.ai/anvil/quickstart/setup');
    const res = await middleware(req as never);
    expect(res.headers.get('location')).toContain('next=%2Fanvil%2Fquickstart%2Fsetup');
  });
});
