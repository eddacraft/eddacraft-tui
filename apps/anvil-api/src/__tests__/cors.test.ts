import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import type { Hono } from 'hono';

// Pin a known origin into the allowlist before the module is imported, since
// `index.ts` reads ANVIL_CORS_ORIGINS at module-load time.
const ALLOWED_ORIGIN = 'https://cors-test.example';
const originalCorsOrigins = process.env['ANVIL_CORS_ORIGINS'];

let app: Hono<any>;

beforeAll(async () => {
  process.env['ANVIL_CORS_ORIGINS'] = ALLOWED_ORIGIN;
  ({ default: app } = await import('../index.js'));
});

afterAll(() => {
  if (originalCorsOrigins === undefined) {
    delete process.env['ANVIL_CORS_ORIGINS'];
  } else {
    process.env['ANVIL_CORS_ORIGINS'] = originalCorsOrigins;
  }
});

describe('CORS preflight', () => {
  // Guards the deliberate maxAge=300 chosen in index.ts. Bumping this back to
  // a long TTL means an API outage poisons browsers for the full TTL after
  // recovery (see the commit that introduced the 300s value).
  it('sets Access-Control-Max-Age=300 on preflight responses', async () => {
    const res = await app.request('/api/v1/waitlist', {
      method: 'OPTIONS',
      headers: {
        Origin: ALLOWED_ORIGIN,
        'Access-Control-Request-Method': 'POST',
        'Access-Control-Request-Headers': 'content-type',
      },
    });

    expect(res.status).toBe(204);
    expect(res.headers.get('access-control-max-age')).toBe('300');
    expect(res.headers.get('access-control-allow-origin')).toBe(ALLOWED_ORIGIN);
  });
});
