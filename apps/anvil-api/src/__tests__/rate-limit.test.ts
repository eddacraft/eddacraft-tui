import { describe, it, expect } from 'vitest';
import { Hono } from 'hono';
import { rateLimiter } from '../middleware/rate-limit.js';

/**
 * Mount a fresh `rateLimiter` per test. Each call to `rateLimiter()` mints its
 * own in-memory store, so tests never bleed counters into one another.
 */
function mount(opts?: { windowMs?: number; max?: number }) {
  const app = new Hono();
  app.use('*', rateLimiter({ windowMs: opts?.windowMs ?? 60_000, max: opts?.max ?? 1 }));
  app.get('/ping', (c) => c.json({ ok: true }));
  return app;
}

function hit(app: Hono, headers: Record<string, string>) {
  return app.request('/ping', { headers });
}

describe('rateLimiter — trusted client-identity keying (CIB-140)', () => {
  it('keys on the platform-trusted x-real-ip, not the attacker-set X-Forwarded-For prefix', async () => {
    // Both requests carry the SAME edge-established identity (x-real-ip) but
    // DIFFERENT, attacker-chosen X-Forwarded-For prefixes. Keyed on the trusted
    // IP they collapse into one bucket, so the second request is rejected. Had
    // it regressed to `x-forwarded-for[0]` they would land in separate buckets
    // and both pass.
    const app = mount({ max: 1 });

    const first = await hit(app, {
      'x-forwarded-for': 'evil, 203.0.113.7',
      'x-real-ip': '203.0.113.7',
    });
    expect(first.status).toBe(200);

    const second = await hit(app, {
      'x-forwarded-for': 'totally-different-spoof, 198.51.100.9',
      'x-real-ip': '203.0.113.7',
    });
    expect(second.status).toBe(429);
  });

  it('does not key on X-Forwarded-For at all — a shared spoofed prefix does not merge distinct clients', async () => {
    // Same spoofed X-Forwarded-For prefix, different trusted IPs → separate
    // buckets, so neither is throttled.
    const app = mount({ max: 1 });

    const a = await hit(app, { 'x-forwarded-for': 'evil', 'x-real-ip': '203.0.113.1' });
    const b = await hit(app, { 'x-forwarded-for': 'evil', 'x-real-ip': '203.0.113.2' });

    expect(a.status).toBe(200);
    expect(b.status).toBe(200);
  });

  it('x-real-ip takes precedence over x-vercel-forwarded-for', async () => {
    const app = mount({ max: 1 });

    // Same x-real-ip, different x-vercel-forwarded-for → keyed on x-real-ip, so
    // the second collapses into the same bucket and is rejected.
    const first = await hit(app, {
      'x-real-ip': '203.0.113.7',
      'x-vercel-forwarded-for': '198.51.100.1',
    });
    expect(first.status).toBe(200);

    const second = await hit(app, {
      'x-real-ip': '203.0.113.7',
      'x-vercel-forwarded-for': '198.51.100.2',
    });
    expect(second.status).toBe(429);
  });

  it('falls back to x-vercel-forwarded-for (single opaque IP) when x-real-ip is absent', async () => {
    const app = mount({ max: 1 });

    expect((await hit(app, { 'x-vercel-forwarded-for': '203.0.113.7' })).status).toBe(200);
    expect((await hit(app, { 'x-vercel-forwarded-for': '203.0.113.7' })).status).toBe(429);
    // A genuinely different single IP gets its own bucket.
    expect((await hit(app, { 'x-vercel-forwarded-for': '198.51.100.9' })).status).toBe(200);
  });

  it('treats a comma-bearing x-vercel-forwarded-for as untrusted → shared sentinel bucket', async () => {
    // No chain parsing: a comma means we cannot attribute a single trusted hop,
    // so the value is rejected and both requests share the sentinel bucket.
    const app = mount({ max: 1 });

    expect((await hit(app, { 'x-vercel-forwarded-for': 'evil, 203.0.113.7' })).status).toBe(200);
    expect((await hit(app, { 'x-vercel-forwarded-for': 'other, 198.51.100.9' })).status).toBe(429);
  });

  it('rejects non-IP garbage in x-real-ip → shared sentinel bucket (no key minting)', async () => {
    // Two requests with distinct garbage x-real-ip values must NOT mint two
    // fresh buckets; both fail closed into the one shared sentinel bucket.
    const app = mount({ max: 1 });

    expect((await hit(app, { 'x-real-ip': 'not-an-ip' })).status).toBe(200);
    expect((await hit(app, { 'x-real-ip': 'another-bogus-value' })).status).toBe(429);
  });

  it('rejects an over-length (>45 char) x-real-ip value → shared sentinel bucket', async () => {
    const app = mount({ max: 1 });
    const tooLong = '1'.repeat(46); // 46 chars — one past the max IPv6 textual length

    expect((await hit(app, { 'x-real-ip': tooLong })).status).toBe(200);
    // A different over-length value also falls through to the same sentinel.
    expect((await hit(app, { 'x-real-ip': '2'.repeat(60) })).status).toBe(429);
  });

  it('accepts a valid IPv6 x-real-ip as a distinct bucket key', async () => {
    const app = mount({ max: 1 });

    expect((await hit(app, { 'x-real-ip': '2001:db8::1' })).status).toBe(200);
    expect((await hit(app, { 'x-real-ip': '2001:db8::1' })).status).toBe(429);
    // A different IPv6 gets its own bucket.
    expect((await hit(app, { 'x-real-ip': '2001:db8::2' })).status).toBe(200);
  });

  it('fails CLOSED into one shared bucket when no platform header is present', async () => {
    // Two header-less requests (local dev / tests / non-Vercel edge) must share
    // ONE limit key rather than each minting a fresh per-request bucket.
    const app = mount({ max: 1 });

    expect((await hit(app, {})).status).toBe(200);
    expect((await hit(app, {})).status).toBe(429);
  });

  it('exposes RateLimit headers on the success path', async () => {
    const app = mount({ max: 5 });
    const res = await hit(app, { 'x-real-ip': '203.0.113.7' });
    expect(res.headers.get('X-RateLimit-Limit')).toBe('5');
    expect(res.headers.get('X-RateLimit-Remaining')).toBe('4');
    expect(res.headers.get('X-RateLimit-Reset')).toBeTruthy();
  });
});
