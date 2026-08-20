import { describe, it, expect, beforeAll, beforeEach, afterEach, vi } from 'vitest';
import { SignJWT, importPKCS8 } from 'jose';

vi.stubEnv('ANVIL_DOCS_URL', 'https://eddacraft-anvil-docs-private.vercel.app');
vi.stubEnv('PUBLIC_DOCS_URL', 'https://eddacraft-docs-public.vercel.app');
vi.stubEnv('DOCS_UPSTREAM_SECRET', 'test-secret');

const { default: proxy } = await import('./proxy');
const { resetKeyCache } = await import('./lib/jwt');

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
  expSecondsFromNow: number = 3600,
  // Real post-BACT-013 mint shape: `plan` is the entitlement claim and `tier`
  // mirrors it byte-for-byte (anvil-api `signLicence`). A `tier`-only token
  // with no `plan` was never minted for any value except the legacy 'pro'.
  claims: Record<string, unknown> = { sub: 'test@example.com', plan: 'beta', tier: 'beta' }
): Promise<string> {
  const privateKey = await importPKCS8(TEST_PRIVATE_KEY_PEM, 'ES256');
  let builder = new SignJWT(claims)
    .setProtectedHeader({ alg: 'ES256' })
    .setIssuedAt()
    .setExpirationTime(Math.floor(Date.now() / 1000) + expSecondsFromNow)
    .setIssuer('https://api.eddacraft.ai')
    .setAudience('anvil-cli');
  if (typeof claims['sub'] === 'string') {
    builder = builder.setSubject(claims['sub']);
  }
  return builder.sign(privateKey);
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

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('redirects to login when no cookie', async () => {
    const req = makeRequest('https://docs.eddacraft.ai/anvil/overview');
    const res = await proxy(req as never);
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
    const res = await proxy(req as never);
    expect(res.status).not.toBe(302);
  });

  it('redirects and clears cookie when token is expired', async () => {
    const token = await signToken(-60);
    const req = makeRequest('https://docs.eddacraft.ai/anvil/overview', {
      'anvil-docs-session': token,
    });
    const res = await proxy(req as never);
    expect(res.status).toBe(302);
    const setCookie = res.headers.get('set-cookie') ?? '';
    expect(setCookie).toContain('anvil-docs-session=');
    expect(setCookie).toContain('Max-Age=0');
  });

  it('redirects when token is garbage', async () => {
    const req = makeRequest('https://docs.eddacraft.ai/anvil/overview', {
      'anvil-docs-session': 'not.a.jwt',
    });
    const res = await proxy(req as never);
    expect(res.status).toBe(302);
  });

  it('redirects and clears cookie when token lacks docs entitlement', async () => {
    const token = await signToken(3600, { sub: 'test@example.com' });
    const req = makeRequest('https://docs.eddacraft.ai/anvil/overview', {
      'anvil-docs-session': token,
    });
    const res = await proxy(req as never);
    expect(res.status).toBe(302);
    const setCookie = res.headers.get('set-cookie') ?? '';
    expect(setCookie).toContain('anvil-docs-session=');
    expect(setCookie).toContain('Max-Age=0');
  });

  it('preserves deep path in next param', async () => {
    const req = makeRequest('https://docs.eddacraft.ai/anvil/quickstart/setup');
    const res = await proxy(req as never);
    expect(res.headers.get('location')).toContain('next=%2Fanvil%2Fquickstart%2Fsetup');
  });
});

describe('proxy upstream behaviour', () => {
  let fetchSpy: ReturnType<typeof vi.spyOn>;

  beforeAll(() => {
    process.env.LICENSE_PUBLIC_KEY = TEST_PUBLIC_KEY_PEM;
  });

  beforeEach(() => {
    resetKeyCache();
    fetchSpy = vi.spyOn(globalThis, 'fetch').mockResolvedValue(new Response('ok', { status: 200 }));
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('routes public paths to PUBLIC_DOCS_URL', async () => {
    const req = makeRequest('https://docs.eddacraft.ai/kindling/overview');
    await proxy(req as never);
    const [url] = fetchSpy.mock.calls[0];
    expect(url.toString()).toMatch(/^https:\/\/eddacraft-docs-public\.vercel\.app/);
  });

  it('routes /anvil paths to ANVIL_DOCS_URL with valid token', async () => {
    const token = await signToken();
    const req = makeRequest('https://docs.eddacraft.ai/anvil/overview', {
      'anvil-docs-session': token,
    });
    await proxy(req as never);
    const [url] = fetchSpy.mock.calls[0];
    expect(url.toString()).toMatch(/^https:\/\/eddacraft-anvil-docs-private\.vercel\.app/);
  });

  it('injects x-docs-upstream-secret header', async () => {
    const req = makeRequest('https://docs.eddacraft.ai/kindling/overview');
    await proxy(req as never);
    const [, init] = fetchSpy.mock.calls[0];
    const headers = init?.headers as Headers;
    expect(headers.get('x-docs-upstream-secret')).toBe('test-secret');
  });

  it('does not forward cookie or authorization headers', async () => {
    const token = await signToken();
    const req = new Request('https://docs.eddacraft.ai/kindling/overview', {
      headers: {
        cookie: `anvil-docs-session=${token}`,
        authorization: 'Bearer leaked',
      },
    });
    await proxy(req as never);
    const [, init] = fetchSpy.mock.calls[0];
    const headers = init?.headers as Headers;
    expect(headers.get('cookie')).toBeNull();
    expect(headers.get('authorization')).toBeNull();
  });

  it('rewrites upstream redirect Location to shell origin', async () => {
    fetchSpy.mockResolvedValueOnce(
      new Response(null, {
        status: 302,
        headers: {
          location: 'https://eddacraft-docs-public.vercel.app/kindling/intro',
        },
      })
    );
    const req = makeRequest('https://docs.eddacraft.ai/kindling/overview');
    const res = await proxy(req as never);
    expect(res.headers.get('location')).toBe('https://docs.eddacraft.ai/kindling/intro');
  });

  it('returns 503 on fetch timeout', async () => {
    fetchSpy.mockImplementationOnce(
      () =>
        new Promise((_resolve, reject) => {
          const err = new Error('aborted');
          err.name = 'AbortError';
          reject(err);
        })
    );
    const req = makeRequest('https://docs.eddacraft.ai/kindling/overview');
    const res = await proxy(req as never);
    expect(res.status).toBe(503);
    expect(await res.text()).toBe('Upstream timeout');
  });

  // Regression: in production this proxy was emitting empty 200 responses
  // (Content-Length: 0, no body) for every proxied page, so the docs UI
  // rendered as a blank white page after sign-in. Root cause: undici
  // transparently decompressed the upstream body, but the upstream's
  // `content-encoding: br` and `content-length: <compressed>` headers
  // were forwarded unchanged, and the runtime dropped the mismatched body.
  it('forwards the body and strips the upstream content-encoding/length headers', async () => {
    const html = '<!doctype html><html><body>upstream content</body></html>';
    fetchSpy.mockResolvedValueOnce(
      new Response(html, {
        status: 200,
        headers: {
          'content-type': 'text/html; charset=utf-8',
          // These two would have been honest about the *original* compressed
          // payload — but undici has already decoded the body before we see
          // it, so leaving them on the response sends a misleading shape to
          // the client.
          'content-encoding': 'br',
          'content-length': '17',
        },
      })
    );
    const req = makeRequest('https://docs.eddacraft.ai/kindling/overview');
    const res = await proxy(req as never);

    const expectedBytes = new TextEncoder().encode(html).byteLength;

    expect(res.status).toBe(200);
    expect(res.headers.get('content-encoding')).toBeNull();
    // Content-Length is now set explicitly to the buffered byte count
    // (instead of being stripped) so the edge has an unambiguous length
    // and can't drop the body for a length/body mismatch.
    expect(res.headers.get('content-length')).toBe(String(expectedBytes));
    expect(res.headers.get('content-type')).toBe('text/html; charset=utf-8');
    expect(res.headers.get('x-docs-shell-upstream-status')).toBe('200');
    expect(res.headers.get('x-docs-shell-upstream-bytes')).toBe(String(expectedBytes));
    expect(res.headers.get('x-docs-shell-build')).toBeTruthy();
    expect(await res.text()).toBe(html);
  });
});
