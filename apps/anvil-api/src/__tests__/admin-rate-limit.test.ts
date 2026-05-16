import { describe, it, expect, beforeEach, vi } from 'vitest';
import { Hono } from 'hono';
import { adminRateLimit } from '../middleware/admin-rate-limit.js';

function mount(actor: string, opts: { windowMs?: number; max?: number; scope?: string } = {}) {
  const app = new Hono();
  // Lightweight stand-in for `adminAuth` so the limiter has the
  // identity it keys on without needing the full middleware stack.
  app.use('*', async (c, next) => {
    c.set('adminActor', actor);
    await next();
  });
  app.use(
    '*',
    adminRateLimit({
      windowMs: opts.windowMs ?? 60_000,
      max: opts.max ?? 3,
      scope: opts.scope ?? 'all',
    })
  );
  app.get('/ping', (c) => c.json({ ok: true }));
  return app;
}

describe('adminRateLimit', () => {
  beforeEach(() => {
    vi.useRealTimers();
  });

  it('allows up to max requests within the window for a single actor', async () => {
    const app = mount('alice@x.com', { max: 3 });
    for (let i = 0; i < 3; i++) {
      const res = await app.request('/ping');
      expect(res.status, `request ${i + 1}/3 should pass`).toBe(200);
    }
  });

  it('returns 429 with admin_rate_limited code once the cap is exceeded', async () => {
    const app = mount('alice@x.com', { max: 2 });
    await app.request('/ping');
    await app.request('/ping');
    const res = await app.request('/ping');
    expect(res.status).toBe(429);
    const body = (await res.json()) as Record<string, unknown>;
    expect(body['code']).toBe('admin_rate_limited');
    expect(body['scope']).toBe('all');
    expect(typeof body['retry_after_seconds']).toBe('number');
    expect(res.headers.get('Retry-After')).toBeTruthy();
  });

  it('keys per actor — alice exhausting her budget does not affect bob', async () => {
    const app = new Hono();
    let actor = 'alice@x.com';
    app.use('*', async (c, next) => {
      c.set('adminActor', actor);
      await next();
    });
    app.use('*', adminRateLimit({ windowMs: 60_000, max: 1, scope: 'all' }));
    app.get('/ping', (c) => c.json({ ok: true }));

    expect((await app.request('/ping')).status).toBe(200);
    expect((await app.request('/ping')).status).toBe(429);

    actor = 'bob@x.com';
    expect((await app.request('/ping')).status).toBe(200);
  });

  it('separates buckets by scope', async () => {
    const app = new Hono();
    app.use('*', async (c, next) => {
      c.set('adminActor', 'alice@x.com');
      await next();
    });
    app.use(
      '/send-migration',
      adminRateLimit({ windowMs: 60_000, max: 1, scope: 'send-migration' })
    );
    app.use('*', adminRateLimit({ windowMs: 60_000, max: 5, scope: 'all' }));
    app.get('/send-migration', (c) => c.json({ ok: true }));
    app.get('/other', (c) => c.json({ ok: true }));

    expect((await app.request('/send-migration')).status).toBe(200);
    expect((await app.request('/send-migration')).status).toBe(429);
    // The coarse `all` scope is independent — `/other` still answers.
    expect((await app.request('/other')).status).toBe(200);
  });

  it('exposes RateLimit headers on the success path', async () => {
    const app = mount('alice@x.com', { max: 5 });
    const res = await app.request('/ping');
    expect(res.headers.get('X-RateLimit-Limit')).toBe('5');
    expect(res.headers.get('X-RateLimit-Remaining')).toBe('4');
    expect(res.headers.get('X-RateLimit-Scope')).toBe('all');
    expect(res.headers.get('X-RateLimit-Reset')).toBeTruthy();
  });

  it('falls back to an `unauthenticated` bucket when no adminActor is set', async () => {
    const app = new Hono();
    app.use('*', adminRateLimit({ windowMs: 60_000, max: 1, scope: 'all' }));
    app.get('/ping', (c) => c.json({ ok: true }));
    expect((await app.request('/ping')).status).toBe(200);
    expect((await app.request('/ping')).status).toBe(429);
  });

  it('resets the bucket after the window expires', async () => {
    vi.useFakeTimers();
    const app = mount('alice@x.com', { windowMs: 1_000, max: 1 });
    expect((await app.request('/ping')).status).toBe(200);
    expect((await app.request('/ping')).status).toBe(429);
    vi.advanceTimersByTime(1_500);
    expect((await app.request('/ping')).status).toBe(200);
  });
});
