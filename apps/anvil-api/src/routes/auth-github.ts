import { Hono } from 'hono';
import { z } from 'zod';
import { zValidator } from '@hono/zod-validator';
import { getClient } from '../db/client.js';
import {
  linkOrCreateGitHubUser,
  insertAuditLog,
  GitHubAccountLinkConflictError,
} from '../db/queries.js';
import { mintSession } from '../lib/session.js';
import { fetchGitHubUser, type GitHubIdentity } from '../lib/github-user.js';
import { createDebugger } from '../lib/debug.js';

const debug = createDebugger('api');

const GITHUB_TOKEN_URL = 'https://github.com/login/oauth/access_token';

const callbackSchema = z.object({
  code: z.string().min(1).max(256),
});

const GitHubTokenSchema = z.object({
  access_token: z.string(),
  token_type: z.string(),
});

function getGitHubCredentials(): { clientId: string; clientSecret: string } {
  const clientId = process.env['GITHUB_CLIENT_ID'];
  const clientSecret = process.env['GITHUB_CLIENT_SECRET'];
  if (!clientId || !clientSecret) {
    throw new Error('GITHUB_CLIENT_ID and GITHUB_CLIENT_SECRET are required');
  }
  return { clientId, clientSecret };
}

async function exchangeCodeForToken(code: string): Promise<string> {
  const { clientId, clientSecret } = getGitHubCredentials();

  const res = await fetch(GITHUB_TOKEN_URL, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      Accept: 'application/json',
    },
    body: JSON.stringify({
      client_id: clientId,
      client_secret: clientSecret,
      code,
    }),
  });

  if (!res.ok) {
    throw new Error(`GitHub token exchange failed: ${res.status}`);
  }

  const body = (await res.json()) as Record<string, unknown>;
  if (body.error) {
    throw new Error(`GitHub OAuth error: ${body.error_description ?? body.error}`);
  }

  const parsed = GitHubTokenSchema.parse(body);
  return parsed.access_token;
}

async function revokeGitHubToken(accessToken: string): Promise<void> {
  try {
    const { clientId, clientSecret } = getGitHubCredentials();
    const auth = Buffer.from(`${clientId}:${clientSecret}`).toString('base64');
    await fetch(`https://api.github.com/applications/${clientId}/token`, {
      method: 'DELETE',
      signal: AbortSignal.timeout(8_000),
      headers: {
        Authorization: `Basic ${auth}`,
        Accept: 'application/json',
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({ access_token: accessToken }),
    });
  } catch (err) {
    debug('failed to revoke github token', {
      error: err instanceof Error ? err.message : String(err),
    });
  }
}

const authGithub = new Hono();

/**
 * POST /auth/github/callback
 *
 * Exchange a GitHub OAuth code for a BAUTH JWT and refresh token.
 * Creates a new user with status=pending if email is not found.
 *
 * Trust boundary: this endpoint is called server-to-server from the
 * docs-site callback function. CSRF/state validation happens entirely
 * in the docs-site layer (apps/docs-site/api/auth/callback.ts) before
 * this endpoint is called. The API trusts the caller to have validated
 * the OAuth state parameter.
 */
authGithub.post('/callback', zValidator('json', callbackSchema), async (c) => {
  debug('POST /auth/github/callback');
  const { code } = c.req.valid('json');

  let ghUser: GitHubIdentity;
  try {
    const accessToken = await exchangeCodeForToken(code);
    ghUser = await fetchGitHubUser(accessToken);
    // Revoke the GitHub token before finalising the callback. The helper is
    // best-effort, so an upstream revocation failure does not fail auth.
    await revokeGitHubToken(accessToken);
  } catch (err) {
    debug('github auth failed', { error: err instanceof Error ? err.message : String(err) });
    return c.json({ error: 'GitHub authentication failed' }, 401);
  }

  const sql = getClient();
  // Resolve the beta_users row: match on github_id, else first-link an active
  // invited row via any verified email, else create a pending row
  // (GHCLIAUTH-003). The active-status gate below stays here, per-caller.
  let result: Awaited<ReturnType<typeof linkOrCreateGitHubUser>>;
  try {
    result = await linkOrCreateGitHubUser(sql, ghUser);
  } catch (err) {
    if (err instanceof GitHubAccountLinkConflictError) {
      // A verified email resolved to an active row already linked to a
      // different github_id — a rejected (re)link/takeover attempt. Audit it,
      // then fail closed with the same generic 401 as other auth failures
      // (no account enumeration).
      await insertAuditLog(sql, 'github_oauth_link_conflict', ghUser.email, {
        githubId: ghUser.id,
        githubLogin: ghUser.login,
      });
      debug('github link conflict — rejected', { githubId: ghUser.id });
    } else {
      debug('github account linking failed', {
        error: err instanceof Error ? err.message : String(err),
      });
    }
    return c.json({ error: 'GitHub authentication failed' }, 401);
  }
  const { user, isNewPending, didFirstLink } = result;

  if (isNewPending) {
    await insertAuditLog(sql, 'github_oauth_signup', user.email, {
      githubId: ghUser.id,
      githubLogin: ghUser.login,
    });
    debug('created pending user via github oauth', { userId: user.id, githubId: ghUser.id });
  } else if (didFirstLink) {
    // Audit the moment a GitHub id is bound to a pre-existing active invite —
    // the one path that realises ADR-066's accepted email==account residual
    // risk, so it must leave a distinct, correlatable trail.
    await insertAuditLog(sql, 'github_oauth_link', user.email, {
      githubId: ghUser.id,
      githubLogin: ghUser.login,
    });
    debug('linked github identity to existing user', { userId: user.id, githubId: ghUser.id });
  }

  if (user.status !== 'active') {
    await insertAuditLog(sql, 'github_oauth_blocked', user.email, {
      githubId: ghUser.id,
      status: user.status,
    });
    debug('github oauth for non-active user', { userId: user.id, status: user.status });
    return c.json({ error: 'Account pending approval' }, 403);
  }

  // Mint the licence + refresh token. Scope resolution keeps a user invited
  // with a graded scope (e.g. `preview`) on that scope through the GitHub OAuth
  // flow; defaults to `['beta']` for first-time sign-ups.
  const session = await mintSession(sql, {
    user,
    identity: { provider: 'github', id: String(ghUser.id) },
  });

  await insertAuditLog(sql, 'github_oauth_login', user.email, {
    githubId: ghUser.id,
    githubLogin: ghUser.login,
  });

  debug('github oauth login successful', { userId: user.id, githubId: ghUser.id });

  return c.json(session);
});

export { authGithub };
