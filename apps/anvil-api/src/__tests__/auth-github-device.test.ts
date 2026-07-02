import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { Hono } from 'hono';
import { authGithubDevice } from '../routes/auth-github-device.js';
import {
  insertGithubDeviceSession,
  findGithubDeviceSessionByPollTokenHash,
  claimGithubDevicePoll,
  storeGithubDeviceMint,
  linkOrCreateGitHubUser,
  insertAuditLog,
  GitHubAccountLinkConflictError,
} from '../db/queries.js';
import { mintSession } from '../lib/session.js';
import { decryptDeviceCode, encryptDeviceCode } from '../lib/github-device-crypto.js';
import { globalRateLimiter } from '../middleware/rate-limit.js';

vi.mock('../db/client.js', () => ({
  getClient: vi.fn(() => vi.fn()),
}));

vi.mock('../db/queries.js', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../db/queries.js')>();
  return {
    ...actual,
    insertGithubDeviceSession: vi.fn(),
    findGithubDeviceSessionByPollTokenHash: vi.fn(),
    claimGithubDevicePoll: vi.fn(),
    storeGithubDeviceMint: vi.fn(),
    linkOrCreateGitHubUser: vi.fn(),
    insertAuditLog: vi.fn(),
  };
});

vi.mock('../lib/session.js', () => ({
  mintSession: vi.fn(),
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
function mockGithubUpstream(spec: { status?: number; json?: unknown } = {}) {
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
/**
 * Unique per-test source IP so the per-IP limiter never bleeds across tests.
 * The limiter now IP-shape-validates its key (CIB-140), so the value must stay
 * a valid IPv4 — spread the counter across two octets rather than overflowing
 * the last one past 255.
 */
function freshIp(): string {
  ipCounter += 1;
  const third = Math.floor(ipCounter / 250) % 256;
  const fourth = (ipCounter % 250) + 1; // 1..250, never 0 or >255
  return `192.0.${third}.${fourth}`;
}

// The per-IP limiter keys on the Vercel-established client identity, not the
// client-suppliable `x-forwarded-for` (CIB-140). Drive it via `x-real-ip`,
// which is the header Vercel's edge sets to the observed client IP.
function start(body: unknown, ip = freshIp()) {
  return app.request('/auth/github-device/start', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', 'x-real-ip': ip },
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

    it('cannot be evaded by rotating a spoofed multi-hop X-Forwarded-For (CIB-140)', async () => {
      mockGithubUpstream();
      // Same Vercel-established identity (x-real-ip) every time, but each
      // request forges a different two-hop X-Forwarded-For prefix. The limiter
      // must key on the trusted identity, so the spoofed chain buys no extra
      // budget and the 11th request is still rejected.
      const trustedIp = '198.51.100.88';
      const spoof = (n: number) => `evil-${n}, 203.0.113.${n}`;

      const request = (n: number) =>
        app.request('/auth/github-device/start', {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
            'x-real-ip': trustedIp,
            'x-forwarded-for': spoof(n),
          },
          body: JSON.stringify({}),
        });

      for (let i = 0; i < 10; i++) {
        expect((await request(i)).status).toBe(200);
      }
      expect((await request(99)).status).toBe(429);
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

// ---------------------------------------------------------------------------
// POST /auth/github-device/poll (GHCLIAUTH-005)
// ---------------------------------------------------------------------------

const POLL_TOKEN = 'f'.repeat(64);
const GITHUB_ACCESS_TOKEN = 'gho_testaccesstoken1234567890';
const GITHUB_TOKEN_URL = 'https://github.com/login/oauth/access_token';
const GITHUB_USER_URL = 'https://api.github.com/user';
const GITHUB_EMAILS_URL = 'https://api.github.com/user/emails';
const GITHUB_REVOKE_URL = 'https://api.github.com/applications/test-cli-client-id/token';

const MINTED_SESSION = {
  license: 'lic-1',
  refreshToken: 'rt-1',
  expiresAt: '2026-06-18T00:00:00.000Z',
};

function sessionRow(overrides: Record<string, unknown> = {}) {
  return {
    id: 'sess-1',
    poll_token_hash: `hash:${POLL_TOKEN}`,
    github_device_code_enc: encryptDeviceCode(POLL_TOKEN, GITHUB_DEVICE_CODE),
    interval_s: 5,
    expires_at: new Date(Date.now() + 600_000).toISOString(),
    last_polled_at: null,
    minted_at: null,
    minted_session_enc: null,
    created_at: new Date().toISOString(),
    ...overrides,
  };
}

// No `ok` field: Response.ok derives from `status`, so status is the only
// control surface the mock honours.
type FetchSpec = { status?: number; json?: unknown };

/**
 * URL-prefix-keyed fetch mock (longest prefix wins). Unmocked URLs throw so a
 * missing mock surfaces loudly. Every call is recorded for order assertions.
 */
function mockFetchMap(responses: Record<string, FetchSpec>) {
  const calls: Array<{ url: string; init?: RequestInit }> = [];
  const keys = Object.keys(responses).sort((a, b) => b.length - a.length);
  vi.spyOn(globalThis, 'fetch').mockImplementation(
    async (input: RequestInfo | URL, init?: RequestInit) => {
      const url = typeof input === 'string' ? input : input.toString();
      calls.push({ url, init });
      const key = keys.find((k) => url.startsWith(k));
      if (!key) throw new Error(`no mock for fetch(${url})`);
      const spec = responses[key]!;
      return new Response(JSON.stringify(spec.json ?? {}), {
        status: spec.status ?? 200,
        headers: { 'Content-Type': 'application/json' },
      });
    }
  );
  return calls;
}

/** Happy-path GitHub mocks: successful exchange, identity fetch, revoke. */
function mockGithubPollUpstream(tokenJson?: unknown) {
  return mockFetchMap({
    [GITHUB_TOKEN_URL]: {
      json: tokenJson ?? { access_token: GITHUB_ACCESS_TOKEN, token_type: 'bearer' },
    },
    [GITHUB_USER_URL]: { json: { id: 424242, login: 'octo', name: null } },
    [GITHUB_EMAILS_URL]: {
      json: [{ email: 'dev@example.com', primary: true, verified: true }],
    },
    [GITHUB_REVOKE_URL]: { json: {} },
  });
}

function poll(body: unknown, ip = freshIp()) {
  return app.request('/auth/github-device/poll', {
    method: 'POST',
    // See `start`: the limiter keys on `x-real-ip` (Vercel-established), not
    // the spoofable `x-forwarded-for` (CIB-140).
    headers: { 'Content-Type': 'application/json', 'x-real-ip': ip },
    body: JSON.stringify(body),
  });
}

describe('POST /auth/github-device/poll', () => {
  beforeEach(() => {
    vi.mocked(findGithubDeviceSessionByPollTokenHash).mockResolvedValue(sessionRow() as never);
    vi.mocked(claimGithubDevicePoll).mockResolvedValue(sessionRow() as never);
    vi.mocked(storeGithubDeviceMint).mockResolvedValue(true);
    vi.mocked(linkOrCreateGitHubUser).mockResolvedValue({
      user: { id: 'user-1', email: 'dev@example.com', status: 'active' },
      isNewPending: false,
      didFirstLink: false,
    } as never);
    vi.mocked(insertAuditLog).mockResolvedValue({} as never);
    vi.mocked(mintSession).mockResolvedValue(MINTED_SESSION);
  });

  describe('confirmed path', () => {
    it('exchanges, gates, mints exactly once, and returns the session', async () => {
      const calls = mockGithubPollUpstream();

      const res = await poll({ pollToken: POLL_TOKEN });
      expect(res.status).toBe(200);
      expect((await res.json()) as Record<string, unknown>).toEqual({
        status: 'confirmed',
        ...MINTED_SESSION,
      });

      // exchange carries the decrypted device_code + client_id, never a secret
      const exchange = calls.find((c) => c.url === GITHUB_TOKEN_URL)!;
      const sent = JSON.parse(String(exchange.init?.body)) as Record<string, unknown>;
      expect(sent).toEqual({
        client_id: 'test-cli-client-id',
        device_code: GITHUB_DEVICE_CODE,
        grant_type: 'urn:ietf:params:oauth:grant-type:device_code',
      });
      expect(String(exchange.init?.body)).not.toContain('test-cli-client-secret');
      expect(exchange.init?.signal).toBeInstanceOf(AbortSignal);

      // identity derived from the token, and the token revoked before returning
      const urls = calls.map((c) => c.url);
      expect(urls).toContain(GITHUB_USER_URL);
      expect(urls.indexOf(GITHUB_REVOKE_URL)).toBeGreaterThan(urls.indexOf(GITHUB_USER_URL));

      expect(linkOrCreateGitHubUser).toHaveBeenCalledWith(
        expect.anything(),
        expect.objectContaining({ id: 424242, verifiedEmails: ['dev@example.com'] })
      );
      expect(mintSession).toHaveBeenCalledTimes(1);
      expect(mintSession).toHaveBeenCalledWith(
        expect.anything(),
        expect.objectContaining({ identity: { provider: 'github', id: '424242' } })
      );

      // the stored mint is the session encrypted under the client-held token
      const [, hash, enc] = vi.mocked(storeGithubDeviceMint).mock.calls[0]!;
      expect(hash).toBe(`hash:${POLL_TOKEN}`);
      expect(JSON.parse(decryptDeviceCode(POLL_TOKEN, enc)!)).toEqual(MINTED_SESSION);

      expect(insertAuditLog).toHaveBeenCalledWith(
        expect.anything(),
        'github_oauth_login',
        'dev@example.com',
        expect.anything()
      );
    });

    it('rejects a body carrying anything beyond pollToken', async () => {
      const res = await poll({ pollToken: POLL_TOKEN, email: 'x@y.z' });
      expect(res.status).toBe(400);
      expect(claimGithubDevicePoll).not.toHaveBeenCalled();
    });
  });

  describe('re-return within TTL (single-use mint)', () => {
    it('re-returns the stored session without touching GitHub or re-minting', async () => {
      vi.mocked(findGithubDeviceSessionByPollTokenHash).mockResolvedValue(
        sessionRow({
          minted_at: new Date().toISOString(),
          minted_session_enc: encryptDeviceCode(POLL_TOKEN, JSON.stringify(MINTED_SESSION)),
        }) as never
      );
      const fetchSpy = vi.spyOn(globalThis, 'fetch');

      const res = await poll({ pollToken: POLL_TOKEN });
      expect(res.status).toBe(200);
      expect((await res.json()) as Record<string, unknown>).toEqual({
        status: 'confirmed',
        ...MINTED_SESSION,
      });
      expect(fetchSpy).not.toHaveBeenCalled();
      expect(mintSession).not.toHaveBeenCalled();
      expect(claimGithubDevicePoll).not.toHaveBeenCalled();
    });

    it('expires a minted session past its TTL instead of re-returning it', async () => {
      vi.mocked(findGithubDeviceSessionByPollTokenHash).mockResolvedValue(
        sessionRow({
          minted_at: new Date(Date.now() - 1_000_000).toISOString(),
          minted_session_enc: encryptDeviceCode(POLL_TOKEN, JSON.stringify(MINTED_SESSION)),
          expires_at: new Date(Date.now() - 1_000).toISOString(),
        }) as never
      );

      const res = await poll({ pollToken: POLL_TOKEN });
      expect((await res.json()) as Record<string, unknown>).toEqual({ status: 'expired' });
    });

    it('recovers a lost mint race by re-returning the winner’s stored session', async () => {
      mockGithubPollUpstream();
      vi.mocked(storeGithubDeviceMint).mockResolvedValue(false);
      vi.mocked(findGithubDeviceSessionByPollTokenHash)
        .mockResolvedValueOnce(sessionRow() as never)
        .mockResolvedValueOnce(
          sessionRow({
            minted_at: new Date().toISOString(),
            minted_session_enc: encryptDeviceCode(POLL_TOKEN, JSON.stringify(MINTED_SESSION)),
          }) as never
        );

      const res = await poll({ pollToken: POLL_TOKEN });
      expect(res.status).toBe(200);
      expect((await res.json()) as Record<string, unknown>).toEqual({
        status: 'confirmed',
        ...MINTED_SESSION,
      });
      expect(mintSession).toHaveBeenCalledTimes(1);
    });
  });

  describe('RFC 8628 status mapping', () => {
    it('maps authorization_pending to pending', async () => {
      mockGithubPollUpstream({ error: 'authorization_pending' });

      const res = await poll({ pollToken: POLL_TOKEN });
      expect(res.status).toBe(200);
      expect((await res.json()) as Record<string, unknown>).toEqual({ status: 'pending' });
      expect(mintSession).not.toHaveBeenCalled();
    });

    it('passes slow_down through as 429 with the upstream interval', async () => {
      mockGithubPollUpstream({ error: 'slow_down', interval: 10 });

      const res = await poll({ pollToken: POLL_TOKEN });
      expect(res.status).toBe(429);
      expect((await res.json()) as Record<string, unknown>).toEqual({
        error: 'slow_down',
        retryAfter: 10,
      });
    });

    it('falls back to interval_s + 5 when slow_down carries no interval', async () => {
      mockGithubPollUpstream({ error: 'slow_down' });

      const res = await poll({ pollToken: POLL_TOKEN });
      expect(res.status).toBe(429);
      expect((await res.json()) as Record<string, unknown>).toEqual({
        error: 'slow_down',
        retryAfter: 10,
      });
    });

    it('clamps a hostile slow_down interval instead of passing it through', async () => {
      mockGithubPollUpstream({ error: 'slow_down', interval: 0 });

      const res = await poll({ pollToken: POLL_TOKEN });
      expect(res.status).toBe(429);
      const body = (await res.json()) as { retryAfter: number };
      expect(body.retryAfter).toBeGreaterThanOrEqual(1);
      expect(body.retryAfter).toBeLessThanOrEqual(3_600);
    });

    it('treats an RFC error body on a non-200 as an outage, not a polling state', async () => {
      mockFetchMap({
        [GITHUB_TOKEN_URL]: { status: 500, json: { error: 'authorization_pending' } },
      });

      const res = await poll({ pollToken: POLL_TOKEN });
      expect(res.status).toBe(502);
      expect((await res.json()) as Record<string, unknown>).toEqual({
        error: 'github_unavailable',
      });
    });

    it('maps expired_token to expired', async () => {
      mockGithubPollUpstream({ error: 'expired_token' });

      const res = await poll({ pollToken: POLL_TOKEN });
      expect((await res.json()) as Record<string, unknown>).toEqual({ status: 'expired' });
    });

    it('maps access_denied to declined', async () => {
      mockGithubPollUpstream({ error: 'access_denied' });

      const res = await poll({ pollToken: POLL_TOKEN });
      expect((await res.json()) as Record<string, unknown>).toEqual({ status: 'declined' });
    });

    it('maps an unrecognised upstream error to 502', async () => {
      mockGithubPollUpstream({ error: 'incorrect_device_code' });

      const res = await poll({ pollToken: POLL_TOKEN });
      expect(res.status).toBe(502);
    });
  });

  describe('session row gating', () => {
    it('treats an unknown poll token as expired', async () => {
      vi.mocked(findGithubDeviceSessionByPollTokenHash).mockResolvedValue(null);

      const res = await poll({ pollToken: POLL_TOKEN });
      expect((await res.json()) as Record<string, unknown>).toEqual({ status: 'expired' });
      expect(claimGithubDevicePoll).not.toHaveBeenCalled();
    });

    it('treats an expired session row as expired without touching GitHub', async () => {
      vi.mocked(findGithubDeviceSessionByPollTokenHash).mockResolvedValue(
        sessionRow({ expires_at: new Date(Date.now() - 1_000).toISOString() }) as never
      );
      const fetchSpy = vi.spyOn(globalThis, 'fetch');

      const res = await poll({ pollToken: POLL_TOKEN });
      expect((await res.json()) as Record<string, unknown>).toEqual({ status: 'expired' });
      expect(fetchSpy).not.toHaveBeenCalled();
    });

    it('rate-limits via the cross-instance poll gate when the claim is lost', async () => {
      vi.mocked(claimGithubDevicePoll).mockResolvedValue(null);
      const fetchSpy = vi.spyOn(globalThis, 'fetch');

      const res = await poll({ pollToken: POLL_TOKEN });
      expect(res.status).toBe(429);
      expect((await res.json()) as Record<string, unknown>).toEqual({
        error: 'slow_down',
        retryAfter: 5,
      });
      expect(fetchSpy).not.toHaveBeenCalled();
    });

    it('re-returns the session when the claim was lost to a concurrent winner', async () => {
      vi.mocked(claimGithubDevicePoll).mockResolvedValue(null);
      vi.mocked(findGithubDeviceSessionByPollTokenHash)
        .mockResolvedValueOnce(sessionRow() as never)
        .mockResolvedValueOnce(
          sessionRow({
            minted_at: new Date().toISOString(),
            minted_session_enc: encryptDeviceCode(POLL_TOKEN, JSON.stringify(MINTED_SESSION)),
          }) as never
        );

      const res = await poll({ pollToken: POLL_TOKEN });
      expect(res.status).toBe(200);
      expect((await res.json()) as Record<string, unknown>).toEqual({
        status: 'confirmed',
        ...MINTED_SESSION,
      });
      expect(mintSession).not.toHaveBeenCalled();
    });

    it('fails closed to expired when the stored device code does not decrypt', async () => {
      const foreign = sessionRow({
        github_device_code_enc: encryptDeviceCode('0'.repeat(64), GITHUB_DEVICE_CODE),
      });
      vi.mocked(findGithubDeviceSessionByPollTokenHash).mockResolvedValue(foreign as never);
      vi.mocked(claimGithubDevicePoll).mockResolvedValue(foreign as never);
      const fetchSpy = vi.spyOn(globalThis, 'fetch');

      const res = await poll({ pollToken: POLL_TOKEN });
      expect((await res.json()) as Record<string, unknown>).toEqual({ status: 'expired' });
      expect(fetchSpy).not.toHaveBeenCalled();
    });
  });

  describe('identity, gate parity, and fail-closed paths', () => {
    it('returns awaiting_approval + github_oauth_blocked audit for a non-active user', async () => {
      const calls = mockGithubPollUpstream();
      vi.mocked(linkOrCreateGitHubUser).mockResolvedValue({
        user: { id: 'user-2', email: 'pending@example.com', status: 'pending' },
        isNewPending: true,
        didFirstLink: false,
      } as never);

      const res = await poll({ pollToken: POLL_TOKEN });
      expect(res.status).toBe(200);
      expect((await res.json()) as Record<string, unknown>).toEqual({
        status: 'awaiting_approval',
      });
      expect(mintSession).not.toHaveBeenCalled();
      expect(insertAuditLog).toHaveBeenCalledWith(
        expect.anything(),
        'github_oauth_blocked',
        'pending@example.com',
        expect.objectContaining({ status: 'pending' })
      );
      // a brand-new pending user gets the signup audit before the block
      expect(insertAuditLog).toHaveBeenCalledWith(
        expect.anything(),
        'github_oauth_signup',
        'pending@example.com',
        expect.anything()
      );
      // the GitHub token is still revoked on the blocked path
      expect(calls.map((c) => c.url)).toContain(GITHUB_REVOKE_URL);
    });

    it('audits github_oauth_link with device_flow provenance on first-link', async () => {
      mockGithubPollUpstream();
      vi.mocked(linkOrCreateGitHubUser).mockResolvedValue({
        user: { id: 'user-1', email: 'dev@example.com', status: 'active' },
        isNewPending: false,
        didFirstLink: true,
      } as never);

      const res = await poll({ pollToken: POLL_TOKEN });
      expect(res.status).toBe(200);
      expect(insertAuditLog).toHaveBeenCalledWith(
        expect.anything(),
        'github_oauth_link',
        'dev@example.com',
        expect.objectContaining({ method: 'device_flow' })
      );
    });

    it('fails closed when the GitHub account has no verified primary email', async () => {
      mockFetchMap({
        [GITHUB_TOKEN_URL]: {
          json: { access_token: GITHUB_ACCESS_TOKEN, token_type: 'bearer' },
        },
        [GITHUB_USER_URL]: { json: { id: 424242, login: 'octo' } },
        [GITHUB_EMAILS_URL]: {
          json: [{ email: 'dev@example.com', primary: true, verified: false }],
        },
        [GITHUB_REVOKE_URL]: { json: {} },
      });

      const res = await poll({ pollToken: POLL_TOKEN });
      expect(res.status).toBe(401);
      expect((await res.json()) as Record<string, unknown>).toEqual({
        error: 'github_authentication_failed',
      });
      expect(linkOrCreateGitHubUser).not.toHaveBeenCalled();
      expect(mintSession).not.toHaveBeenCalled();
    });

    it('audits and fails closed on a github_id link conflict', async () => {
      mockGithubPollUpstream();
      vi.mocked(linkOrCreateGitHubUser).mockRejectedValue(new GitHubAccountLinkConflictError());

      const res = await poll({ pollToken: POLL_TOKEN });
      expect(res.status).toBe(401);
      expect((await res.json()) as Record<string, unknown>).toEqual({
        error: 'github_authentication_failed',
      });
      expect(insertAuditLog).toHaveBeenCalledWith(
        expect.anything(),
        'github_oauth_link_conflict',
        'dev@example.com',
        expect.anything()
      );
      expect(mintSession).not.toHaveBeenCalled();
    });
  });

  describe('upstream + credential failure paths', () => {
    it('returns 503 without touching the DB when CLI credentials are absent', async () => {
      delete process.env['GITHUB_CLI_CLIENT_ID'];

      const res = await poll({ pollToken: POLL_TOKEN });
      expect(res.status).toBe(503);
      expect(findGithubDeviceSessionByPollTokenHash).not.toHaveBeenCalled();
    });

    it('maps an exchange timeout to 502 without minting', async () => {
      vi.spyOn(globalThis, 'fetch').mockRejectedValue(
        Object.assign(new Error('timed out'), { name: 'TimeoutError' })
      );

      const res = await poll({ pollToken: POLL_TOKEN });
      expect(res.status).toBe(502);
      expect((await res.json()) as Record<string, unknown>).toEqual({
        error: 'github_unavailable',
      });
      expect(mintSession).not.toHaveBeenCalled();
    });

    it('maps a malformed exchange body to 502 without minting', async () => {
      mockGithubPollUpstream({ token_type: 'bearer' });

      const res = await poll({ pollToken: POLL_TOKEN });
      expect(res.status).toBe(502);
      expect(mintSession).not.toHaveBeenCalled();
    });
  });
});

// ---------------------------------------------------------------------------
// Structured-log hygiene (GHCLIAUTH-009)
//
// The ungated console.info operational logs must carry latency / outcome /
// error-class metadata ONLY — never a secret value. This drives a real start +
// poll exchange with GitHub mocked, captures every console.info line, and
// asserts none of the secrets in play leak into the serialised output.
// ---------------------------------------------------------------------------

describe('structured-log hygiene (GHCLIAUTH-009)', () => {
  const MINTED_LICENCE = 'lic-secret-do-not-log';
  // Realistic length so the substring assertion has teeth (a short value
  // like 'rt-1' could never meaningfully match a serialised log line).
  const MINTED_REFRESH_TOKEN = 'rt-secret-do-not-log-zq8VwY3kP1mN6cD4';

  beforeEach(() => {
    vi.mocked(findGithubDeviceSessionByPollTokenHash).mockResolvedValue(sessionRow() as never);
    vi.mocked(claimGithubDevicePoll).mockResolvedValue(sessionRow() as never);
    vi.mocked(storeGithubDeviceMint).mockResolvedValue(true);
    vi.mocked(linkOrCreateGitHubUser).mockResolvedValue({
      user: { id: 'user-1', email: 'dev@example.com', status: 'active' },
      isNewPending: false,
      didFirstLink: false,
    } as never);
    vi.mocked(insertAuditLog).mockResolvedValue({} as never);
    vi.mocked(mintSession).mockResolvedValue({
      ...MINTED_SESSION,
      license: MINTED_LICENCE,
      refreshToken: MINTED_REFRESH_TOKEN,
    });
  });

  /** All console.info output, joined, for substring leak assertions. */
  function captureInfo() {
    const infoSpy = vi.spyOn(console, 'info').mockImplementation(() => {});
    return () => infoSpy.mock.calls.map((args) => args.map((a) => String(a)).join(' ')).join('\n');
  }

  /**
   * Every secret value in play across the start + poll flow. None of these may
   * ever appear in an ungated info log line.
   */
  const SECRETS = [
    GITHUB_DEVICE_CODE, // device_code (start payload + poll exchange)
    GITHUB_ACCESS_TOKEN, // GitHub access token from the exchange
    MINTED_LICENCE, // minted Anvil licence string
    MINTED_REFRESH_TOKEN, // minted refresh token
    'test-cli-client-secret', // OAuth client secret
  ];

  function expectNoSecrets(serialised: string) {
    for (const secret of SECRETS) {
      expect(serialised).not.toContain(secret);
    }
  }

  it('emits info logs on a happy-path start + poll that carry no secret values', async () => {
    const dumpInfo = captureInfo();
    mockGithubUpstream();

    const startRes = await start({});
    expect(startRes.status).toBe(200);
    // The poll token returned by /start is a per-session secret too — capture it.
    const { pollToken } = (await startRes.json()) as { pollToken: string };

    mockGithubPollUpstream();
    const pollRes = await poll({ pollToken: POLL_TOKEN });
    expect(pollRes.status).toBe(200);

    const serialised = dumpInfo();
    // The flow must have produced operational info logs at all (not a no-op).
    expect(serialised).toContain('"event":"device_code.upstream"');
    expect(serialised).toContain('"event":"token_exchange.upstream"');
    expect(serialised).toContain('"event":"identity.upstream"');
    expect(serialised).toContain('"event":"login.outcome"');
    expect(serialised).toContain('"outcome":"minted"');

    expectNoSecrets(serialised);
    // The live per-session poll token from /start must not leak either.
    expect(serialised).not.toContain(pollToken);
    expect(serialised).not.toContain(POLL_TOKEN);
    // Hex-shaped secrets (device_code, poll token) would be masked by
    // sanitizeForLog rather than fail the substring checks above — so also
    // assert no legitimate info field ever triggers the redaction filter.
    expect(serialised).not.toContain('[REDACTED]');
  });

  it('suppresses the per-poll pending state from the info stream entirely', async () => {
    const dumpInfo = captureInfo();
    mockGithubPollUpstream({ error: 'authorization_pending' });

    const res = await poll({ pollToken: POLL_TOKEN });
    expect(res.status).toBe(200);

    // Pending fires every ~5s per session — it stays on the gated debug
    // stream only, so the info stream carries terminal outcomes, not spam.
    const serialised = dumpInfo();
    expect(serialised).not.toContain('"outcome":"pending"');
    expectNoSecrets(serialised);
    expect(serialised).not.toContain(POLL_TOKEN);
  });

  it('keeps secrets out of the info log on an upstream non-OK device/code', async () => {
    const dumpInfo = captureInfo();
    mockGithubUpstream({ status: 503, json: { message: 'down' } });

    const res = await start({});
    expect(res.status).toBe(502);

    const serialised = dumpInfo();
    expect(serialised).toContain('"outcome":"non_ok"');
    expectNoSecrets(serialised);
  });
});
