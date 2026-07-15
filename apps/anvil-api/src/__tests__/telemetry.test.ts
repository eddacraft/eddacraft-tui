import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { Hono } from 'hono';
import { telemetry } from '../routes/telemetry.js';
import {
  beaconSchema,
  INSTALL_METHODS,
  MAX_FEATURES_PER_BEACON,
  TELEMETRY_SCHEMA_VERSION,
} from '../routes/telemetry-schemas.js';

const telemetryMocks = vi.hoisted(() => ({
  getClient: vi.fn(),
  sql: vi.fn(),
}));

vi.mock('../db/client.js', () => ({
  getClient: telemetryMocks.getClient,
}));

afterEach(() => {
  vi.restoreAllMocks();
});

const app = new Hono();
app.route('/telemetry', telemetry);

const originalDatabaseUrl = process.env['DATABASE_URL'];

/** A fully valid schema-version-1 beacon per the ADR-107 §3 allowlist. */
function validBeacon(): Record<string, unknown> {
  return {
    schema_version: TELEMETRY_SCHEMA_VERSION,
    install_id: '7c9e6679-7425-40de-944b-e07fc1f90ae7',
    version: '0.9.0-beta',
    install_method: 'homebrew',
    platform: 'x86_64-unknown-linux-gnu',
    channel: 'beta',
    flag_snapshot_version: '12',
    features: [
      { key: 'anvil.check', count: 42 },
      { key: 'anvil.gate', count: 3 },
    ],
  };
}

function post(body: unknown, headers: HeadersInit = {}) {
  return app.request('/telemetry', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', ...headers },
    body: JSON.stringify(body),
  });
}

/** Reassemble the SQL text of every tagged-template call the mock received. */
function capturedQueries(): Array<{ text: string; params: unknown[] }> {
  return telemetryMocks.sql.mock.calls.map((call) => {
    const [strings, ...params] = call as [readonly string[], ...unknown[]];
    return { text: strings.join(' ? '), params };
  });
}

describe('telemetry ingest route', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    process.env['DATABASE_URL'] = 'postgres://telemetry-test';
    telemetryMocks.getClient.mockReturnValue(telemetryMocks.sql);
    telemetryMocks.sql.mockResolvedValue([]);
  });

  afterEach(() => {
    if (originalDatabaseUrl === undefined) {
      delete process.env['DATABASE_URL'];
    } else {
      process.env['DATABASE_URL'] = originalDatabaseUrl;
    }
  });

  describe('POST /telemetry — accepted payloads', () => {
    it('accepts a valid beacon and stores it', async () => {
      const response = await post(validBeacon());

      expect(response.status).toBe(202);
      expect(await response.json()).toEqual({ accepted: true });
      expect(telemetryMocks.sql).toHaveBeenCalledTimes(1);
      const [insert] = capturedQueries();
      expect(insert!.text).toContain('INSERT INTO telemetry_beacons');
    });

    it('accepts a beacon with no feature usage since the last beacon', async () => {
      const response = await post({ ...validBeacon(), features: [] });

      expect(response.status).toBe(202);
    });

    it('accepts every documented install_method', async () => {
      expect(INSTALL_METHODS).toEqual([
        'homebrew',
        'scoop',
        'winget',
        'cargo_dist',
        'cargo_install',
        'dev_build',
        'unknown',
      ]);
      for (const method of INSTALL_METHODS) {
        const response = await post({ ...validBeacon(), install_method: method });
        expect(response.status).toBe(202);
      }
    });
  });

  describe('POST /telemetry — schema_version gate', () => {
    it('rejects a payload with no schema_version', async () => {
      const beacon = validBeacon();
      delete beacon['schema_version'];

      const response = await post(beacon);

      expect(response.status).toBe(400);
      const body = (await response.json()) as { error: string };
      expect(body.error).toContain('schema_version');
      expect(telemetryMocks.sql).not.toHaveBeenCalled();
    });

    it('rejects an unknown schema_version', async () => {
      const response = await post({ ...validBeacon(), schema_version: 999 });

      expect(response.status).toBe(400);
      const body = (await response.json()) as { error: string };
      expect(body.error).toContain('schema_version');
      expect(telemetryMocks.sql).not.toHaveBeenCalled();
    });

    it('rejects a non-numeric schema_version', async () => {
      const response = await post({ ...validBeacon(), schema_version: '1' });

      expect(response.status).toBe(400);
      expect(telemetryMocks.sql).not.toHaveBeenCalled();
    });
  });

  describe('POST /telemetry — allowlist posture (strict schema)', () => {
    it('rejects unknown top-level fields', async () => {
      const response = await post({ ...validBeacon(), hostname: 'my-laptop' });

      expect(response.status).toBe(400);
      expect(telemetryMocks.sql).not.toHaveBeenCalled();
    });

    it('rejects unknown fields inside feature usage entries', async () => {
      const response = await post({
        ...validBeacon(),
        features: [{ key: 'anvil.check', count: 1, repo: '/home/user/project' }],
      });

      expect(response.status).toBe(400);
      expect(telemetryMocks.sql).not.toHaveBeenCalled();
    });

    it('does not echo rejected payload values back in the error body', async () => {
      const response = await post({ ...validBeacon(), hostname: 'secret-host-name' });

      expect(response.status).toBe(400);
      expect(await response.text()).not.toContain('secret-host-name');
    });
  });

  describe('POST /telemetry — field validation', () => {
    it('rejects a non-UUID install_id', async () => {
      const response = await post({ ...validBeacon(), install_id: 'not-a-uuid' });

      expect(response.status).toBe(400);
      expect(telemetryMocks.sql).not.toHaveBeenCalled();
    });

    it('rejects an install_method outside the enum', async () => {
      const response = await post({ ...validBeacon(), install_method: 'apt' });

      expect(response.status).toBe(400);
    });

    it('rejects an oversized version string (length cap)', async () => {
      const response = await post({ ...validBeacon(), version: 'v'.repeat(65) });

      expect(response.status).toBe(400);
    });

    it('rejects an oversized platform string (length cap)', async () => {
      const response = await post({ ...validBeacon(), platform: 'x'.repeat(65) });

      expect(response.status).toBe(400);
    });

    it('rejects an oversized feature key (length cap)', async () => {
      const response = await post({
        ...validBeacon(),
        features: [{ key: 'k'.repeat(200), count: 1 }],
      });

      expect(response.status).toBe(400);
    });

    it('rejects free-form strings by construction (charset, not just length)', async () => {
      // PII-shaped values — an email, a path, a hostname with spaces — must
      // fail the token charset even when they fit the length cap.
      const freeForm = [
        { ...validBeacon(), channel: 'me@example.com' },
        { ...validBeacon(), version: '/home/user/repo' },
        { ...validBeacon(), platform: 'my laptop (work)' },
        { ...validBeacon(), flag_snapshot_version: 'v1 with spaces' },
      ];
      for (const payload of freeForm) {
        const response = await post(payload);
        expect(response.status).toBe(400);
      }
      expect(telemetryMocks.sql).not.toHaveBeenCalled();
    });

    it('rejects more feature entries than the cap', async () => {
      const features = Array.from({ length: MAX_FEATURES_PER_BEACON + 1 }, (_, i) => ({
        key: `feature.${i}`,
        count: 1,
      }));
      const response = await post({ ...validBeacon(), features });

      expect(response.status).toBe(400);
    });

    it('rejects negative and non-integer feature counts', async () => {
      for (const count of [-1, 1.5, Number.NaN]) {
        const response = await post({
          ...validBeacon(),
          features: [{ key: 'anvil.check', count }],
        });
        expect(response.status).toBe(400);
      }
    });
  });

  describe('POST /telemetry — transport validation', () => {
    it('rejects a non-JSON content type', async () => {
      const response = await app.request('/telemetry', {
        method: 'POST',
        headers: { 'Content-Type': 'text/plain' },
        body: JSON.stringify(validBeacon()),
      });

      expect(response.status).toBe(400);
    });

    it('rejects malformed JSON', async () => {
      const response = await app.request('/telemetry', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: '{not json',
      });

      expect(response.status).toBe(400);
    });

    it('rejects a JSON array body', async () => {
      const response = await post([validBeacon()]);

      expect(response.status).toBe(400);
    });

    it('returns 503 when DATABASE_URL is not configured', async () => {
      delete process.env['DATABASE_URL'];

      const response = await post(validBeacon());

      expect(response.status).toBe(503);
    });

    it('returns a generic 500 when the insert fails', async () => {
      telemetryMocks.sql.mockRejectedValue(new Error('boom'));

      const response = await post(validBeacon());

      expect(response.status).toBe(500);
      expect(await response.json()).toEqual({ error: 'Failed to record beacon' });
    });
  });

  describe('POST /telemetry — IP absence in storage (ADR-107 §3)', () => {
    it('never passes the request IP to any SQL statement', async () => {
      const clientIp = '203.0.113.9';
      const response = await post(validBeacon(), {
        'x-real-ip': clientIp,
        'x-forwarded-for': `${clientIp}, 10.0.0.1`,
        'x-vercel-forwarded-for': clientIp,
      });

      expect(response.status).toBe(202);
      expect(telemetryMocks.sql).toHaveBeenCalled();
      for (const { text, params } of capturedQueries()) {
        expect(JSON.stringify(params)).not.toContain(clientIp);
        expect(text).not.toContain(clientIp);
      }
    });

    it('inserts no ip column — the column list is the ADR-107 allowlist only', async () => {
      await post(validBeacon(), { 'x-real-ip': '203.0.113.9' });

      const [insert] = capturedQueries();
      // Guard against any ip-named column sneaking into the insert.
      expect(insert!.text).not.toMatch(/\bip\b|client_ip|remote_addr|x_real_ip/i);
    });
  });

  describe('POST /telemetry — timestamp coarsening (ADR-107 §3)', () => {
    it('passes no timestamp to the insert; received_on falls to the DATE column default', async () => {
      await post(validBeacon());

      const [insert] = capturedQueries();
      // No client- or server-side time-of-day value may reach the row: the
      // insert carries neither a Date param, an ISO datetime param, nor a
      // now()/timestamptz expression. Date coarsening is enforced by the
      // received_on DATE DEFAULT current_date column definition.
      for (const param of insert!.params) {
        expect(param).not.toBeInstanceOf(Date);
        expect(String(param)).not.toMatch(/\d{2}:\d{2}:\d{2}/);
      }
      expect(insert!.text).not.toMatch(/now\(\)|timestamptz|created_at/i);
    });
  });
});

describe('telemetry beacon schema (unit)', () => {
  it('parses a valid beacon and strips nothing (allowlist is exact)', () => {
    const beacon = {
      schema_version: 1,
      install_id: '7c9e6679-7425-40de-944b-e07fc1f90ae7',
      version: '0.9.0-beta',
      install_method: 'cargo_dist',
      platform: 'aarch64-apple-darwin',
      channel: 'stable',
      flag_snapshot_version: '3',
      features: [{ key: 'anvil.fix', count: 0 }],
    };

    const parsed = beaconSchema.parse(beacon);

    expect(parsed).toEqual(beacon);
  });

  it('rejects unknown keys rather than stripping them', () => {
    const result = beaconSchema.safeParse({
      schema_version: 1,
      install_id: '7c9e6679-7425-40de-944b-e07fc1f90ae7',
      version: '0.9.0-beta',
      install_method: 'cargo_dist',
      platform: 'aarch64-apple-darwin',
      channel: 'stable',
      flag_snapshot_version: '3',
      features: [],
      email: 'person@example.com',
    });

    expect(result.success).toBe(false);
  });
});

describe('telemetry route mounting (index.ts)', () => {
  it('is mounted under the versioned base path and covered by the shared rate limiter', async () => {
    const { default: indexApp } = await import('../index.js');
    telemetryMocks.getClient.mockReturnValue(telemetryMocks.sql);
    telemetryMocks.sql.mockResolvedValue([]);
    process.env['DATABASE_URL'] = 'postgres://telemetry-test';

    const response = await indexApp.request('/api/v1/telemetry', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(validBeacon()),
    });

    expect(response.status).toBe(202);
    // The app-level rateLimiter() stamps X-RateLimit-* headers on every
    // request it admits — their presence proves the ingest route sits
    // behind the shared limiter rather than being mounted around it.
    expect(response.headers.get('X-RateLimit-Limit')).not.toBeNull();
  });
});
