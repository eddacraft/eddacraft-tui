import { Hono } from 'hono';
import { z } from 'zod';
import { zValidator } from '@hono/zod-validator';
import { randomBytes } from 'node:crypto';
import { getClient } from '../db/client.js';
import { insertGithubDeviceSession } from '../db/queries.js';
import { createDebugger } from '../lib/debug.js';
import { getGitHubCliCredentials } from '../lib/github-cli-credentials.js';
import { encryptDeviceCode } from '../lib/github-device-crypto.js';
import { hashToken } from '../lib/token.js';
import { globalRateLimiter, rateLimiter } from '../middleware/rate-limit.js';

const debug = createDebugger('auth-github-device');

const GITHUB_DEVICE_CODE_URL = 'https://github.com/login/device/code';
// read:user — github_id for account linking; user:email — verified emails for
// the first-link match (ADR-066 decision 4).
const GITHUB_OAUTH_SCOPE = 'read:user user:email';
// Below the Vercel function ceiling so a hung upstream fails us, not the
// platform (ADR-066 ops precondition).
const UPSTREAM_TIMEOUT_MS = 8_000;
// RFC 8628 §3.2: poll interval defaults to 5s when the server omits it.
const DEFAULT_INTERVAL_S = 5;

// Strict empty body: this endpoint accepts NO fields. In particular no email
// and no user reference — the bound user is derived solely from the GitHub
// token at poll-confirmation time, which is what keeps the #1779 class of
// caller-supplied identity structurally impossible (ADR-066 security
// invariant). `.strict()` makes a smuggled field a 400, not a silent ignore.
const startSchema = z.object({}).strict();

// Upper bounds defend the Date/int arithmetic against a hostile or broken
// upstream value (GitHub actually returns expires_in≈900, interval 5): an
// unbounded expires_in would overflow the Date epoch ceiling and turn into an
// unhandled 500 at insert time. Out-of-range maps to the same 502 as any
// other malformed upstream body.
const githubDeviceCodeResponseSchema = z.object({
  device_code: z.string().min(1),
  user_code: z.string().min(1),
  verification_uri: z.string().min(1),
  expires_in: z.number().int().positive().max(86_400),
  interval: z.number().int().positive().max(3_600).optional(),
});

function generatePollToken(): string {
  return randomBytes(32).toString('hex');
}

const authGithubDevice = new Hono();

/**
 * POST /auth/github-device/start
 *
 * Begins a brokered GitHub Device Authorization Grant (RFC 8628) session for
 * the CLI: requests a device/user code pair from GitHub, persists the session
 * for cross-instance polling (hashed poll_token, encrypted device_code), and
 * hands the CLI the verification details plus an opaque poll_token.
 *
 * Rate limited per-IP and globally because it proxies a credentialed upstream
 * call (ADR-066 ops precondition).
 */
authGithubDevice.post(
  '/start',
  rateLimiter({ windowMs: 60_000, max: 10 }),
  globalRateLimiter({ windowMs: 60_000, max: 60 }),
  zValidator('json', startSchema),
  async (c) => {
    debug('POST /auth/github-device/start');

    let clientId: string;
    try {
      ({ clientId } = getGitHubCliCredentials());
    } catch {
      debug('start rejected: github cli credentials unavailable');
      return c.json({ error: 'github_device_flow_unavailable' }, 503);
    }

    const startedAt = Date.now();
    let upstream: Response;
    try {
      upstream = await fetch(GITHUB_DEVICE_CODE_URL, {
        method: 'POST',
        headers: { Accept: 'application/json', 'Content-Type': 'application/json' },
        body: JSON.stringify({ client_id: clientId, scope: GITHUB_OAUTH_SCOPE }),
        signal: AbortSignal.timeout(UPSTREAM_TIMEOUT_MS),
      });
    } catch (err: unknown) {
      debug('device/code upstream fetch failed', {
        errorClass: err instanceof Error ? err.name : typeof err,
        ms: Date.now() - startedAt,
      });
      return c.json({ error: 'github_unavailable' }, 502);
    }

    if (!upstream.ok) {
      debug('device/code upstream non-OK', {
        status: upstream.status,
        ms: Date.now() - startedAt,
      });
      return c.json({ error: 'github_unavailable' }, 502);
    }

    const parsed = githubDeviceCodeResponseSchema.safeParse(
      await upstream.json().catch(() => null)
    );
    if (!parsed.success) {
      debug('device/code upstream body malformed', { ms: Date.now() - startedAt });
      return c.json({ error: 'github_unavailable' }, 502);
    }

    const gh = parsed.data;
    const intervalS = gh.interval ?? DEFAULT_INTERVAL_S;
    const pollToken = generatePollToken();

    const sql = getClient();
    await insertGithubDeviceSession(sql, {
      pollTokenHash: hashToken(pollToken),
      deviceCodeEnc: encryptDeviceCode(pollToken, gh.device_code),
      intervalS,
      expiresAt: new Date(Date.now() + gh.expires_in * 1000),
    });

    debug('github device session created', {
      intervalS,
      expiresIn: gh.expires_in,
      ms: Date.now() - startedAt,
    });

    return c.json({
      userCode: gh.user_code,
      verificationUri: gh.verification_uri,
      interval: intervalS,
      expiresIn: gh.expires_in,
      pollToken,
    });
  }
);

export { authGithubDevice };
