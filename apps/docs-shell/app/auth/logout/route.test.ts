import { describe, expect, it } from 'vitest';
import { NextRequest } from 'next/server';
import { GET, POST } from './route';

const origin = 'https://docs.eddacraft.ai';

function request(method: string, headers: Record<string, string> = {}) {
  return new NextRequest(`${origin}/auth/logout?next=https://evil.example`, {
    method,
    headers: { cookie: 'anvil-docs-session=existing-session', ...headers },
  });
}

describe('/auth/logout request integrity (SEC-015)', () => {
  it('does not clear a session on GET, including cross-site navigation', async () => {
    const response = await GET();
    expect(response.status).toBe(405);
    expect(response.headers.get('allow')).toBe('POST');
    expect(response.headers.has('set-cookie')).toBe(false);
    expect(response.headers.has('location')).toBe(false);
  });

  const rejectedHeaders: Record<string, string>[] = [
    {},
    { origin: 'null' },
    { origin: 'not-an-origin' },
    { origin: 'https://evil.example' },
    { origin: 'https://docs.eddacraft.ai.evil.example' },
    { origin: 'https://sibling.eddacraft.ai' },
    { origin: 'http://docs.eddacraft.ai' },
    { origin: 'https://docs.eddacraft.ai:444' },
    { origin: `${origin}/path` },
    { origin: `${origin}, https://evil.example` },
    { origin: 'https://evil.example', 'x-forwarded-host': 'evil.example' },
    { origin, 'sec-fetch-site': 'cross-site' },
    { origin, 'sec-fetch-site': 'same-site' },
    { origin, 'sec-fetch-site': 'none' },
  ];

  it.each(rejectedHeaders)(
    'rejects untrusted request headers %j without mutating cookies',
    async (headers) => {
      const response = await POST(request('POST', headers));
      expect(response.status).toBe(403);
      expect(response.headers.has('set-cookie')).toBe(false);
      expect(response.headers.has('location')).toBe(false);
    }
  );

  it.each([undefined, 'same-origin'])(
    'accepts exact Origin with Fetch Metadata %s',
    async (site) => {
      const response = await POST(
        request('POST', {
          origin,
          ...(site ? { 'sec-fetch-site': site } : {}),
        })
      );
      expect(response.status).toBe(303);
      expect(response.headers.get('location')).toBe(`${origin}/`);
      expect(response.headers.get('cache-control')).toBe('no-store');
      expect(response.cookies.get('anvil-docs-session')).toMatchObject({
        value: '',
        path: '/',
        maxAge: 0,
        httpOnly: true,
        secure: true,
        sameSite: 'lax',
      });
      expect(response.cookies.getAll()).toHaveLength(1);
    }
  );
});
