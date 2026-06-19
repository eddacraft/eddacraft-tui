import { describe, it, expect, vi } from 'vitest';
import { z } from 'zod';
import { AdminClient, AdminError } from '../client.js';

function makeFetch(response: { status: number; body: string; ok?: boolean }): typeof fetch {
  return vi.fn(async () => {
    return new Response(response.body, { status: response.status });
  }) as unknown as typeof fetch;
}

function makeClient(fetchImpl: typeof fetch): AdminClient {
  return new AdminClient({
    url: 'https://api.example.com',
    key: 'test-key',
    actor: 'test@example.com',
    fetchImpl,
  });
}

describe('AdminClient', () => {
  it('sends Authorization and X-Admin-Actor headers on GET', async () => {
    const calls: { url: string; init: RequestInit | undefined }[] = [];
    const fetchImpl = vi.fn(async (url: unknown, init?: RequestInit) => {
      calls.push({ url: String(url), init });
      return new Response(JSON.stringify({ ok: true }), { status: 200 });
    }) as unknown as typeof fetch;

    const client = makeClient(fetchImpl);
    const result = await client.get<{ ok: boolean }>('/admin/waitlist');

    expect(result).toEqual({ ok: true });
    expect(calls[0]?.url).toBe('https://api.example.com/admin/waitlist');
    const headers = calls[0]?.init?.headers as Record<string, string>;
    expect(headers.Authorization).toBe('Bearer test-key');
    expect(headers['X-Admin-Actor']).toBe('test@example.com');
  });

  it('appends query params and drops undefined values', async () => {
    const calls: { url: string }[] = [];
    const fetchImpl = vi.fn(async (url: unknown) => {
      calls.push({ url: String(url) });
      return new Response('{}', { status: 200 });
    }) as unknown as typeof fetch;

    const client = makeClient(fetchImpl);
    await client.get('/admin/audit', { action: 'user.approved', actor: undefined, limit: 10 });

    const url = new URL(calls[0]!.url);
    expect(url.searchParams.get('action')).toBe('user.approved');
    expect(url.searchParams.has('actor')).toBe(false);
    expect(url.searchParams.get('limit')).toBe('10');
  });

  it('sends Content-Type and JSON body on POST', async () => {
    const calls: { init: RequestInit | undefined }[] = [];
    const fetchImpl = vi.fn(async (_url: unknown, init?: RequestInit) => {
      calls.push({ init });
      return new Response('{"approved":[]}', { status: 200 });
    }) as unknown as typeof fetch;

    const client = makeClient(fetchImpl);
    await client.post('/admin/approve', { email: 'a@b.c' });

    const headers = calls[0]?.init?.headers as Record<string, string>;
    expect(headers['Content-Type']).toBe('application/json');
    expect(calls[0]?.init?.method).toBe('POST');
    expect(calls[0]?.init?.body).toBe('{"email":"a@b.c"}');
  });

  it('throws AdminError with exitCode 1 and body error string on 4xx', async () => {
    const client = makeClient(
      makeFetch({ status: 404, body: '{"error":"Email not found on waitlist"}' })
    );
    await expect(client.post('/admin/approve', {})).rejects.toMatchObject({
      exitCode: 1,
      status: 404,
      message: 'Email not found on waitlist',
    });
  });

  it('throws AdminError with exitCode 2 on 5xx', async () => {
    const client = makeClient(makeFetch({ status: 503, body: 'upstream unavailable' }));
    await expect(client.get('/admin/waitlist')).rejects.toMatchObject({
      exitCode: 2,
      status: 503,
    });
  });

  it('throws AdminError with exitCode 3 on network failure', async () => {
    const fetchImpl = vi.fn(async () => {
      throw new Error('ECONNREFUSED');
    }) as unknown as typeof fetch;
    const client = makeClient(fetchImpl);
    await expect(client.get('/admin/waitlist')).rejects.toMatchObject({
      exitCode: 3,
    });
  });

  it('trims trailing slash from base URL', async () => {
    const calls: { url: string }[] = [];
    const fetchImpl = vi.fn(async (url: unknown) => {
      calls.push({ url: String(url) });
      return new Response('{}', { status: 200 });
    }) as unknown as typeof fetch;
    const client = new AdminClient({
      url: 'https://api.example.com///',
      key: 'k',
      actor: 'a',
      fetchImpl,
    });
    await client.get('/admin/waitlist');
    expect(calls[0]?.url).toBe('https://api.example.com/admin/waitlist');
  });

  it('AdminError carries exitCode, status, and body', () => {
    const err = new AdminError('boom', 2, 502, 'body');
    expect(err).toBeInstanceOf(Error);
    expect(err.exitCode).toBe(2);
    expect(err.status).toBe(502);
    expect(err.body).toBe('body');
  });

  describe('response validation', () => {
    const schema = z.object({ id: z.number(), name: z.string() });

    it('returns the validated payload unchanged when it matches', async () => {
      const client = makeClient(makeFetch({ status: 200, body: '{"id":1,"name":"ok"}' }));
      const result = await client.get('/x', undefined, schema);
      expect(result).toEqual({ id: 1, name: 'ok' });
    });

    it('throws AdminError with exitCode 6 on shape mismatch and names the field path', async () => {
      const client = makeClient(
        makeFetch({ status: 200, body: '{"id":"not-a-number","name":"ok"}' })
      );
      await expect(client.get('/x', undefined, schema)).rejects.toMatchObject({
        exitCode: 6,
        status: 200,
      });
      await expect(client.get('/x', undefined, schema)).rejects.toThrow(
        /response validation failed at id:/
      );
    });

    it('reports the nested field path for array-indexed fields', async () => {
      const nested = z.object({ items: z.array(z.object({ email: z.string() })) });
      const client = makeClient(
        makeFetch({ status: 200, body: '{"items":[{"email":"a@b.c"},{"email":42}]}' })
      );
      await expect(client.get('/x', undefined, nested)).rejects.toThrow(
        /response validation failed at items\.1\.email:/
      );
    });

    it('reports <root> when the outer type is wrong', async () => {
      const client = makeClient(makeFetch({ status: 200, body: '[]' }));
      await expect(client.get('/x', undefined, schema)).rejects.toThrow(
        /response validation failed at <root>:/
      );
    });

    it('skips validation when no schema is supplied (backwards-compatible)', async () => {
      const client = makeClient(makeFetch({ status: 200, body: '{"anything":true}' }));
      const result = await client.get<{ anything: boolean }>('/x');
      expect(result.anything).toBe(true);
    });

    it('still prefers the 4xx/5xx path over validation', async () => {
      // 4xx comes first — validation never runs against an error body.
      const client = makeClient(makeFetch({ status: 400, body: '{"error":"bad"}' }));
      await expect(client.get('/x', undefined, schema)).rejects.toMatchObject({
        exitCode: 1,
        status: 400,
      });
    });

    it('preserves the raw body on the error for debugging', async () => {
      const body = '{"id":"bad","name":"ok"}';
      const client = makeClient(makeFetch({ status: 200, body }));
      try {
        await client.get('/x', undefined, schema);
        throw new Error('expected AdminError');
      } catch (err) {
        expect(err).toBeInstanceOf(AdminError);
        expect((err as AdminError).body).toBe(body);
      }
    });

    it('validates POST responses via the schema argument', async () => {
      const client = makeClient(makeFetch({ status: 200, body: '{"revoked":"nope"}' }));
      await expect(
        client.post('/admin/revoke', { email: 'a@b.c' }, z.object({ revoked: z.number() }))
      ).rejects.toMatchObject({ exitCode: 6 });
    });
  });
});
