// apps/docs-shell/lib/bauth.test.ts
import { describe, it, expect, beforeEach, vi, afterEach } from 'vitest';
import { exchangeGithubCode } from './bauth';

const originalFetch = globalThis.fetch;

describe('exchangeGithubCode', () => {
  beforeEach(() => {
    process.env.BAUTH_API_URL = 'https://api.test.example';
  });

  afterEach(() => {
    globalThis.fetch = originalFetch;
  });

  it('returns ok with license on 200', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue(
      new Response(JSON.stringify({ license: 'jwt.here' }), {
        status: 200,
        headers: { 'Content-Type': 'application/json' },
      })
    ) as typeof fetch;

    const result = await exchangeGithubCode('gh-code');
    expect(result.status).toBe('ok');
    if (result.status === 'ok') expect(result.license).toBe('jwt.here');
  });

  it('returns pending on 403', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue(new Response('', { status: 403 })) as typeof fetch;
    const result = await exchangeGithubCode('gh-code');
    expect(result.status).toBe('pending');
  });

  it('returns error on 500', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue(new Response('', { status: 500 })) as typeof fetch;
    const result = await exchangeGithubCode('gh-code');
    expect(result.status).toBe('error');
    if (result.status === 'error') expect(result.reason).toBe('auth_failed');
  });

  it('returns error when body is missing license', async () => {
    globalThis.fetch = vi
      .fn()
      .mockResolvedValue(
        new Response(JSON.stringify({ wrong: 'shape' }), { status: 200 })
      ) as typeof fetch;
    const result = await exchangeGithubCode('gh-code');
    expect(result.status).toBe('error');
    if (result.status === 'error') expect(result.reason).toBe('invalid_response');
  });

  it('returns error when fetch throws', async () => {
    globalThis.fetch = vi.fn().mockRejectedValue(new Error('network down')) as typeof fetch;
    const result = await exchangeGithubCode('gh-code');
    expect(result.status).toBe('error');
    if (result.status === 'error') expect(result.reason).toBe('api_error');
  });
});
