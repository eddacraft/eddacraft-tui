import { Hono, type Context } from 'hono';
import { z } from 'zod';
import { zValidator } from '@hono/zod-validator';
import { randomBytes } from 'node:crypto';
import { getClient } from '../db/client.js';
import {
  insertGithubDeviceSession,
  findGithubDeviceSessionByPollTokenHash,
  claimGithubDevicePoll,
  storeGithubDeviceMint,
  linkOrCreateGitHubUser,
  insertAuditLog,
  GitHubAccountLinkConflictError,
  type GithubDeviceSession,
} from '../db/queries.js';
import { createDebugger, createInfoLogger } from '../lib/debug.js';
import { getGitHubCliCredentials } from '../lib/github-cli-credentials.js';
import { encryptDeviceCode, decryptDeviceCode } from '../lib/github-device-crypto.js';
import { fetchGitHubUser, type GitHubIdentity } from '../lib/github-user.js';
import { mintSession, type MintSessionResult } from '../lib/session.js';
import { hashToken } from '../lib/token.js';
import { globalRateLimiter, rateLimiter } from '../middleware/rate-limit.js';

const debug = createDebugger('auth-github-device');
// Ungated structured operational logger (GHCLIAUTH-009): records upstream-call
// outcomes — latency, outcome, error class, HTTP status — so production can be
// triaged without ANVIL_DEBUG. NEVER pass a secret (access_token, device_code,
// poll_token, licence, email, Authorization) as a field; log class/latency only.
const info = createInfoLogger('auth-github-device');

const GITHUB_DEVICE_CODE_URL = 'https://github.com/login/device/code';
const GITHUB_TOKEN_URL = 'https://github.com/login/oauth/access_token';
const GITHUB_DEVICE_GRANT_TYPE = 'urn:ietf:params:oauth:grant-type:device_code';
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
      const errorClass = err instanceof Error ? err.name : typeof err;
      const ms = Date.now() - startedAt;
      debug('device/code upstream fetch failed', { errorClass, ms });
      info('device_code.upstream', { outcome: 'fetch_error', errorClass, ms });
      return c.json({ error: 'github_unavailable' }, 502);
    }

    if (!upstream.ok) {
      const ms = Date.now() - startedAt;
      debug('device/code upstream non-OK', { status: upstream.status, ms });
      info('device_code.upstream', { outcome: 'non_ok', httpStatus: upstream.status, ms });
      return c.json({ error: 'github_unavailable' }, 502);
    }

    const parsed = githubDeviceCodeResponseSchema.safeParse(
      await upstream.json().catch(() => null)
    );
    if (!parsed.success) {
      const ms = Date.now() - startedAt;
      debug('device/code upstream body malformed', { ms });
      info('device_code.upstream', { outcome: 'malformed_body', httpStatus: upstream.status, ms });
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

    const ms = Date.now() - startedAt;
    debug('github device session created', {
      intervalS,
      expiresIn: gh.expires_in,
      ms,
    });
    info('device_code.upstream', {
      outcome: 'ok',
      httpStatus: upstream.status,
      intervalS,
      expiresIn: gh.expires_in,
      ms,
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

// Strict: pollToken and nothing else — same no-smuggled-identity stance as
// /start. The bound user comes solely from the GitHub token exchanged below.
const pollSchema = z
  .object({
    pollToken: z.string().min(1).max(200),
  })
  .strict();

const githubDeviceTokenResponseSchema = z.object({
  access_token: z.string().min(1),
});

function isSessionExpired(session: GithubDeviceSession): boolean {
  return new Date(session.expires_at).getTime() < Date.now();
}

/**
 * Re-return a minted session within TTL (ADR-066: "mint exactly once,
 * re-returnable within TTL" — a lost response must not turn a success into a
 * false expired). Fails closed to `expired` when the payload won't decrypt.
 */
function reReturnMintedSession(c: Context, session: GithubDeviceSession, pollToken: string) {
  if (isSessionExpired(session) || !session.minted_session_enc) {
    debug('minted session past TTL or payload missing');
    return c.json({ status: 'expired' });
  }
  const stored = decryptDeviceCode(pollToken, session.minted_session_enc);
  if (!stored) {
    debug('minted session payload failed to decrypt — failing closed');
    return c.json({ status: 'expired' });
  }
  let minted: MintSessionResult;
  try {
    minted = JSON.parse(stored) as MintSessionResult;
  } catch {
    debug('minted session payload not valid JSON — failing closed');
    return c.json({ status: 'expired' });
  }
  debug('re-returning minted session within TTL');
  return c.json({ status: 'confirmed', ...minted });
}

/**
 * POST /auth/github-device/poll
 *
 * Completes a brokered device-flow session: exchanges the stored device_code
 * with GitHub, derives the user solely from the resulting token
 * (`fetchGitHubUser` → github_id linking, GHCLIAUTH-003), enforces
 * active-status gate parity with `/auth/github/callback`, and mints the Anvil
 * licence exactly once — re-returnable within TTL.
 *
 * Per-token cooldown + the cross-instance gate are one atomic UPDATE
 * (`claimGithubDevicePoll`): at most one instance exchanges with GitHub per
 * device_code per interval window (ADR-066 ops precondition).
 */
authGithubDevice.post(
  '/poll',
  // The DB claim is the real per-token gate; these bound DB load from a
  // caller spraying one valid poll_token (per-IP) or many (global). A
  // well-behaved CLI polls ~12/min.
  rateLimiter({ windowMs: 60_000, max: 60 }),
  globalRateLimiter({ windowMs: 60_000, max: 300 }),
  zValidator('json', pollSchema),
  async (c) => {
    debug('POST /auth/github-device/poll');

    let clientId: string;
    try {
      ({ clientId } = getGitHubCliCredentials());
    } catch {
      debug('poll rejected: github cli credentials unavailable');
      return c.json({ error: 'github_device_flow_unavailable' }, 503);
    }

    const { pollToken } = c.req.valid('json');
    const pollTokenHash = hashToken(pollToken);
    const sql = getClient();

    const session = await findGithubDeviceSessionByPollTokenHash(sql, pollTokenHash);
    if (!session) {
      // Never issued or already cleaned up — indistinguishable by design.
      debug('poll: no session row — treating as expired');
      return c.json({ status: 'expired' });
    }

    if (session.minted_at) {
      return reReturnMintedSession(c, session, pollToken);
    }

    if (isSessionExpired(session)) {
      debug('poll: session expired');
      return c.json({ status: 'expired' });
    }

    // Per-token cooldown + cross-instance gate in one atomic claim.
    const claimed = await claimGithubDevicePoll(sql, pollTokenHash);
    if (!claimed) {
      // Two reasons the claim fails: the interval gate fired, or a concurrent
      // winner minted between our read and the claim. Re-check so the second
      // case returns the confirmed session instead of a wasted 429 interval.
      const recheck = await findGithubDeviceSessionByPollTokenHash(sql, pollTokenHash);
      if (recheck?.minted_at) {
        return reReturnMintedSession(c, recheck, pollToken);
      }
      debug('poll rate limited by interval gate', { intervalS: session.interval_s });
      return c.json({ error: 'slow_down', retryAfter: session.interval_s }, 429);
    }

    const deviceCode = decryptDeviceCode(pollToken, claimed.github_device_code_enc);
    if (!deviceCode) {
      debug('poll: device code failed to decrypt — failing closed');
      return c.json({ status: 'expired' });
    }

    const startedAt = Date.now();
    let upstream: Response;
    try {
      upstream = await fetch(GITHUB_TOKEN_URL, {
        method: 'POST',
        headers: { Accept: 'application/json', 'Content-Type': 'application/json' },
        // Public-client device grant: client_id only, never the secret
        // (ADR-066 decision 1).
        body: JSON.stringify({
          client_id: clientId,
          device_code: deviceCode,
          grant_type: GITHUB_DEVICE_GRANT_TYPE,
        }),
        signal: AbortSignal.timeout(UPSTREAM_TIMEOUT_MS),
      });
    } catch (err: unknown) {
      const errorClass = err instanceof Error ? err.name : typeof err;
      const ms = Date.now() - startedAt;
      debug('device token exchange fetch failed', { errorClass, ms });
      info('token_exchange.upstream', { outcome: 'fetch_error', errorClass, ms });
      return c.json({ error: 'github_unavailable' }, 502);
    }

    const body = (await upstream.json().catch(() => null)) as Record<string, unknown> | null;

    // RFC 8628 §3.5: GitHub answers 200 with an `error` field for the
    // non-terminal and terminal polling states — map each explicitly. The
    // states are only authoritative on a 200; an error body on a non-2xx is an
    // outage and must surface as 502, not as a polling state the CLI would
    // retry against forever.
    if (upstream.ok && body && typeof body.error === 'string') {
      // RFC 8628 state name is a non-secret protocol token — safe to log as the
      // error class. The device_code/access_token are never in `fields`.
      const ms = Date.now() - startedAt;
      switch (body.error) {
        case 'authorization_pending':
          // Deliberately NOT in the info stream: pending is the normal state
          // every ~5s per session and would drown terminal-outcome signal.
          // Per-poll granularity stays on the gated debug stream.
          debug('poll: authorization pending');
          return c.json({ status: 'pending' });
        case 'slow_down': {
          // Clamp the upstream value — a hostile/broken interval must not
          // drive the CLI into a tight loop (0/negative) or an endless sleep.
          const upstreamInterval =
            typeof body.interval === 'number' && Number.isFinite(body.interval)
              ? Math.min(Math.max(Math.round(body.interval), 1), 3_600)
              : null;
          const retryAfter = upstreamInterval ?? claimed.interval_s + 5;
          debug('poll: upstream slow_down', { retryAfter });
          info('token_exchange.upstream', {
            outcome: 'slow_down',
            errorClass: body.error,
            retryAfter,
            ms,
          });
          return c.json({ error: 'slow_down', retryAfter }, 429);
        }
        case 'expired_token':
          debug('poll: device code expired upstream');
          info('token_exchange.upstream', { outcome: 'expired', errorClass: body.error, ms });
          return c.json({ status: 'expired' });
        case 'access_denied':
          debug('poll: user declined authorization');
          info('token_exchange.upstream', { outcome: 'declined', errorClass: body.error, ms });
          return c.json({ status: 'declined' });
        default:
          debug('poll: unrecognised upstream error', { ms });
          info('token_exchange.upstream', {
            outcome: 'unrecognised_error',
            // Uncontrolled upstream string — clamp so a hostile value cannot
            // bloat the log line or masquerade as taxonomy.
            errorClass: String(body.error).slice(0, 64),
            ms,
          });
          return c.json({ error: 'github_unavailable' }, 502);
      }
    }

    const parsedToken = githubDeviceTokenResponseSchema.safeParse(body);
    if (!upstream.ok || !parsedToken.success) {
      const ms = Date.now() - startedAt;
      debug('device token exchange malformed/non-OK', { status: upstream.status, ms });
      info('token_exchange.upstream', {
        outcome: upstream.ok ? 'malformed_body' : 'non_ok',
        httpStatus: upstream.status,
        ms,
      });
      return c.json({ error: 'github_unavailable' }, 502);
    }

    const exchangeMs = Date.now() - startedAt;
    info('token_exchange.upstream', { outcome: 'ok', httpStatus: upstream.status, ms: exchangeMs });

    // Identity comes solely from the token (ADR-066 security invariant); the
    // GitHub token is revoked immediately after, before any licence leaves.
    const identityStartedAt = Date.now();
    let ghUser: GitHubIdentity;
    try {
      ghUser = await fetchGitHubUser(parsedToken.data.access_token);
    } catch (err) {
      const errorClass = err instanceof Error ? err.name : typeof err;
      const ms = Date.now() - identityStartedAt;
      debug('github identity fetch failed', {
        error: err instanceof Error ? err.message : String(err),
      });
      info('identity.upstream', { outcome: 'fetch_error', errorClass, ms });
      await revokeGitHubCliToken(parsedToken.data.access_token);
      return c.json({ error: 'github_authentication_failed' }, 401);
    }
    info('identity.upstream', { outcome: 'ok', ms: Date.now() - identityStartedAt });
    await revokeGitHubCliToken(parsedToken.data.access_token);

    // github_id linking per GHCLIAUTH-003: returning users match on github_id;
    // first-link matches any verified email; conflicts fail closed.
    let linkResult: Awaited<ReturnType<typeof linkOrCreateGitHubUser>>;
    try {
      linkResult = await linkOrCreateGitHubUser(sql, ghUser);
    } catch (err) {
      if (err instanceof GitHubAccountLinkConflictError) {
        await insertAuditLog(sql, 'github_oauth_link_conflict', ghUser.email, {
          githubId: ghUser.id,
          githubLogin: ghUser.login,
          method: 'device_flow',
        });
        debug('poll: github link conflict — rejected', { githubId: ghUser.id });
        info('login.outcome', { outcome: 'link_conflict' });
      } else {
        debug('poll: github account linking failed', {
          error: err instanceof Error ? err.message : String(err),
        });
        info('login.outcome', { outcome: 'link_error' });
      }
      return c.json({ error: 'github_authentication_failed' }, 401);
    }
    const { user, isNewPending, didFirstLink } = linkResult;

    if (isNewPending) {
      await insertAuditLog(sql, 'github_oauth_signup', user.email, {
        githubId: ghUser.id,
        githubLogin: ghUser.login,
        method: 'device_flow',
      });
      debug('poll: created pending user', { githubId: ghUser.id });
    } else if (didFirstLink) {
      await insertAuditLog(sql, 'github_oauth_link', user.email, {
        githubId: ghUser.id,
        githubLogin: ghUser.login,
        method: 'device_flow',
      });
      debug('poll: linked github identity to existing user', { githubId: ghUser.id });
    }

    // Active-status gate parity with /auth/github/callback — surfaced as a
    // clear terminal poll state, not a generic failure (ADR-066).
    if (user.status !== 'active') {
      await insertAuditLog(sql, 'github_oauth_blocked', user.email, {
        githubId: ghUser.id,
        status: user.status,
        method: 'device_flow',
      });
      debug('poll: non-active user blocked', { status: user.status });
      info('login.outcome', { outcome: 'blocked', userStatus: user.status });
      return c.json({ status: 'awaiting_approval' });
    }

    const mintResult = await mintSession(sql, {
      user,
      identity: { provider: 'github', id: String(ghUser.id) },
      loginMethod: 'github',
    });

    // Single-use claim: only the first mint is recorded; a concurrent loser
    // re-reads and re-returns the winner's stored session.
    const stored = await storeGithubDeviceMint(
      sql,
      pollTokenHash,
      encryptDeviceCode(pollToken, JSON.stringify(mintResult))
    );
    if (!stored) {
      debug('poll: lost the mint race — re-returning stored session');
      const winner = await findGithubDeviceSessionByPollTokenHash(sql, pollTokenHash);
      if (winner?.minted_at) {
        return reReturnMintedSession(c, winner, pollToken);
      }
      return c.json({ error: 'slow_down', retryAfter: session.interval_s }, 429);
    }

    await insertAuditLog(sql, 'github_oauth_login', user.email, {
      githubId: ghUser.id,
      githubLogin: ghUser.login,
      method: 'device_flow',
    });

    debug('poll: licence minted', { githubId: ghUser.id });
    info('login.outcome', { outcome: 'minted', isNewPending, didFirstLink });
    return c.json({ status: 'confirmed', ...mintResult });
  }
);

/**
 * Revoke a GitHub access token issued to the Anvil CLI OAuth app. Best-effort
 * (errors are logged, never thrown) — the token is short-lived regardless, but
 * revoking immediately shrinks the exposure window (ADR-066).
 */
async function revokeGitHubCliToken(accessToken: string): Promise<void> {
  try {
    const { clientId, clientSecret } = getGitHubCliCredentials();
    const auth = Buffer.from(`${clientId}:${clientSecret}`).toString('base64');
    await fetch(`https://api.github.com/applications/${clientId}/token`, {
      method: 'DELETE',
      headers: {
        Authorization: `Basic ${auth}`,
        Accept: 'application/json',
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({ access_token: accessToken }),
      signal: AbortSignal.timeout(UPSTREAM_TIMEOUT_MS),
    });
  } catch (err) {
    debug('failed to revoke github cli token', {
      error: err instanceof Error ? err.message : String(err),
    });
  }
}

export { authGithubDevice };
