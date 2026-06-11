import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { Hono } from 'hono';
import { authGithubDevice } from '../routes/auth-github-device.js';
import { insertGithubDeviceSession } from '../db/queries.js';
import { decryptDeviceCode } from '../lib/github-device-crypto.js';
import { globalRateLimiter } from '../middleware/rate-limit.js';

vi.mock('../db/client.js', () => ({
  getClient: vi.fn(() => vi.fn()),
}));

vi.mock('../db/queries.js', () => ({
  insertGithubDeviceSession: vi.fn(),
}));

vi.mock('../lib/token.js', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../lib/token.js')>();
  return {
    ...actual,
    hashToken: vi.fn((input: string) => `hash:${input}`),
  };
});

const app = new Hono();
app.route('/auth/github-device', authGithubDevice);

const ORIGINAL_CLIENT_ID = process.env['GITHUB_CLI_CLIENT_ID'];
const ORIGINAL_CLIENT_SECRET = process.env['GITHUB_CLI_CLIENT_SECRET'];

const GITHUB_DEVICE_CODE = '3584d83530557fdd1f46af8289938c8ef79f9dc5';

function githubStartPayload(overrides: Record<string, unknown> = {}) {
  return {
    device_code: GITHUB_DEVICE_CODE,
    user_code: 'WDJB-MJHT',
    verification_uri: 'https://github.com/login/device',
    expires_in: 899,
    interval: 5,
    ...overrides,
  };
}

/** Mock the GitHub device/code upstream with a fixed response for every call. */
function mockGithubUpstream(spec: { ok?: boolean; status?: number; json?: unknown } = {}) {
  const status = spec.status ?? 200;
  const body = JSON.stringify(spec.json ?? githubStartPayload());
  return vi.spyOn(globalThis, 'fetch').mockImplementation(async (input: RequestInfo | URL) => {
    const url = typeof input === 'string' ? input : input.toString();
    if (!url.startsWith('https://github.com/login/device/code')) {
      throw new Error(`no mock for fetch(${url})`);
    }
    return new Response(body, {
      status,
      headers: { 'Content-Type': 'application/json' },
    });
  });
}

let ipCounter = 0;
/** Unique per-test source IP so the per-IP limiter never bleeds across tests. */
function freshIp(): string {
  ipCounter += 1;
  return `192.0.2.${ipCounter}`;
}

function start(body: unknown, ip = freshIp()) {
  return app.request('/auth/github-device/start', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', 'x-forwarded-for': ip },
    body: JSON.stringify(body),
  });
}

beforeEach(() => {
  vi.clearAllMocks();
  vi.mocked(insertGithubDeviceSession).mockResolvedValue(undefined);
  process.env['GITHUB_CLI_CLIENT_ID'] = 'test-cli-client-id';
  process.env['GITHUB_CLI_CLIENT_SECRET'] = 'test-cli-client-secret';
  vi.spyOn(globalThis, 'fetch').mockImplementation(() => {
    throw new Error('fetch called without a per-test mock');
  });
});

afterEach(() => {
  vi.restoreAllMocks();
  if (ORIGINAL_CLIENT_ID === undefined) delete process.env['GITHUB_CLI_CLIENT_ID'];
  else process.env['GITHUB_CLI_CLIENT_ID'] = ORIGINAL_CLIENT_ID;
  if (ORIGINAL_CLIENT_SECRET === undefined) delete process.env['GITHUB_CLI_CLIENT_SECRET'];
  else process.env['GITHUB_CLI_CLIENT_SECRET'] = ORIGINAL_CLIENT_SECRET;
});

describe('POST /auth/github-device/start', () => {
  describe('happy path', () => {
    it('brokers the GitHub device-code request and returns the RFC 8628 shape', async () => {
      mockGithubUpstream();

      const res = await start({});
      expect(res.status).toBe(200);

      const body = (await res.json()) as Record<string, unknown>;
      expect(body).toEqual({
        userCode: 'WDJB-MJHT',
        verificationUri: 'https://github.com/login/device',
        interval: 5,
        expiresIn: 899,
        pollToken: expect.stringMatching(/^[0-9a-f]{64}$/),
      });
    });

    it('sends client_id + the read:user user:email scope with a timeout signal', async () => {
      const fetchSpy = mockGithubUpstream();

      await start({});

      expect(fetchSpy).toHaveBeenCalledTimes(1);
      const [, init] = fetchSpy.mock.calls[0]!;
      const sent = JSON.parse(String(init?.body)) as Record<string, unknown>;
      expect(sent).toEqual({
        client_id: 'test-cli-client-id',
        scope: 'read:user user:email',
      });
      expect(init?.signal).toBeInstanceOf(AbortSignal);
      // The client secret must never travel to the device/code endpoint.
      expect(String(init?.body)).not.toContain('test-cli-client-secret');
    });

    it('persists the session hashed/encrypted at rest with no user binding', async () => {
      mockGithubUpstream();

      const res = await start({});
      const body = (await res.json()) as { pollToken: string };

      expect(insertGithubDeviceSession).toHaveBeenCalledTimes(1);
      const [, args] = vi.mocked(insertGithubDeviceSession).mock.calls[0]!;

      // poll_token stored only as a hash of the returned token
      expect(args.pollTokenHash).toBe(`hash:${body.pollToken}`);
      expect(args.pollTokenHash).not.toBe(body.pollToken);

      // device_code stored encrypted: not recoverable from the row alone,
      // recoverable with the client-held poll token (the poll path needs it)
      expect(args.deviceCodeEnc).not.toContain(GITHUB_DEVICE_CODE);
      expect(decryptDeviceCode(body.pollToken, args.deviceCodeEnc)).toBe(GITHUB_DEVICE_CODE);

      // no user binding of any kind at start time (ADR-066 invariant)
      expect(Object.keys(args).sort()).toEqual([
        'deviceCodeEnc',
        'expiresAt',
        'intervalS',
        'pollTokenHash',
      ]);

      expect(args.intervalS).toBe(5);
      const expiresMs = args.expiresAt.getTime() - Date.now();
      expect(expiresMs).toBeGreaterThan(880_000);
      expect(expiresMs).toBeLessThanOrEqual(899_000);
    });

    it('passes a non-default GitHub interval through to the CLI and the row', async () => {
      mockGithubUpstream({ json: githubStartPayload({ interval: 10, expires_in: 1800 }) });

      const res = await start({});
      const body = (await res.json()) as Record<string, unknown>;
      expect(body['interval']).toBe(10);
      expect(body['expiresIn']).toBe(1800);

      const [, args] = vi.mocked(insertGithubDeviceSession).mock.calls[0]!;
      expect(args.intervalS).toBe(10);
    });

    it('defaults interval to 5 when GitHub omits it (RFC 8628 §3.2)', async () => {
      mockGithubUpstream({ json: githubStartPayload({ interval: undefined }) });

      const res = await start({});
      const body = (await res.json()) as Record<string, unknown>;
      expect(body['interval']).toBe(5);
    });
  });

  describe('no-email invariant', () => {
    it('rejects a body carrying an email and never reaches GitHub or the DB', async () => {
      const fetchSpy = vi.spyOn(globalThis, 'fetch');

      const res = await start({ email: 'user@example.com' });
      expect(res.status).toBe(400);
      expect(fetchSpy).not.toHaveBeenCalled();
      expect(insertGithubDeviceSession).not.toHaveBeenCalled();
    });

    it('rejects any other unexpected field', async () => {
      const res = await start({ userId: 'user-1' });
      expect(res.status).toBe(400);
      expect(insertGithubDeviceSession).not.toHaveBeenCalled();
    });
  });

  describe('credential gate', () => {
    it('returns 503 without calling GitHub when CLI credentials are absent', async () => {
      delete process.env['GITHUB_CLI_CLIENT_ID'];
      const fetchSpy = vi.spyOn(globalThis, 'fetch');

      const res = await start({});
      expect(res.status).toBe(503);
      expect((await res.json()) as Record<string, unknown>).toEqual({
        error: 'github_device_flow_unavailable',
      });
      expect(fetchSpy).not.toHaveBeenCalled();
      expect(insertGithubDeviceSession).not.toHaveBeenCalled();
    });
  });

  describe('upstream failure paths', () => {
    it('maps an upstream timeout to 502 and persists nothing', async () => {
      vi.spyOn(globalThis, 'fetch').mockRejectedValue(
        Object.assign(new Error('The operation was aborted due to timeout'), {
          name: 'TimeoutError',
        })
      );

      const res = await start({});
      expect(res.status).toBe(502);
      expect((await res.json()) as Record<string, unknown>).toEqual({
        error: 'github_unavailable',
      });
      expect(insertGithubDeviceSession).not.toHaveBeenCalled();
    });

    it('maps an upstream non-200 to 502 and persists nothing', async () => {
      mockGithubUpstream({ status: 503, json: { message: 'down' } });

      const res = await start({});
      expect(res.status).toBe(502);
      expect(insertGithubDeviceSession).not.toHaveBeenCalled();
    });

    it('maps a malformed upstream body to 502 and persists nothing', async () => {
      mockGithubUpstream({ json: { user_code: 'WDJB-MJHT' } });

      const res = await start({});
      expect(res.status).toBe(502);
      expect(insertGithubDeviceSession).not.toHaveBeenCalled();
    });

    it('rejects an absurd upstream expires_in instead of overflowing Date math', async () => {
      mockGithubUpstream({
        json: githubStartPayload({ expires_in: Number.MAX_SAFE_INTEGER }),
      });

      const res = await start({});
      expect(res.status).toBe(502);
      expect(insertGithubDeviceSession).not.toHaveBeenCalled();
    });

    it('surfaces a DB insert failure as a 500, not a fake success', async () => {
      mockGithubUpstream();
      vi.mocked(insertGithubDeviceSession).mockRejectedValue(new Error('db down'));

      const res = await start({});
      expect(res.status).toBe(500);
    });
  });

  describe('rate limiting', () => {
    it('rejects the 11th request in a window from one IP with 429', async () => {
      mockGithubUpstream();
      const ip = '198.51.100.77';

      for (let i = 0; i < 10; i++) {
        const res = await start({}, ip);
        expect(res.status).toBe(200);
      }
      const eleventh = await start({}, ip);
      expect(eleventh.status).toBe(429);
    });

    // The route mounts globalRateLimiter({ max: 60 }); exhausting that
    // module-level instance here would be order-dependent across the suite.
    // Instead prove the middleware's cross-IP shared budget on a fresh
    // instance — the route-level wiring is covered by the per-IP test above.
    it('shares one global budget across distinct IPs and answers 429 with Retry-After', async () => {
      const limited = new Hono();
      limited.post('/start', globalRateLimiter({ windowMs: 60_000, max: 3 }), (c) =>
        c.json({ ok: true })
      );

      const hit = (ip: string) =>
        limited.request('/start', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json', 'x-forwarded-for': ip },
          body: JSON.stringify({}),
        });

      for (let i = 1; i <= 3; i++) {
        const res = await hit(`203.0.113.${i}`);
        expect(res.status).toBe(200);
      }
      const fourth = await hit('203.0.113.4');
      expect(fourth.status).toBe(429);
      expect(Number(fourth.headers.get('Retry-After'))).toBeGreaterThan(0);
    });
  });
});
