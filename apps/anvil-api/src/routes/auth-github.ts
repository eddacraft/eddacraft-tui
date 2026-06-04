import { Hono } from 'hono';
import { z } from 'zod';
import { zValidator } from '@hono/zod-validator';
import { getClient } from '../db/client.js';
import { linkOrCreateGitHubUser, insertAuditLog } from '../db/queries.js';
import { mintSession } from '../lib/session.js';
import { createDebugger } from '../lib/debug.js';

const debug = createDebugger('api');

const GITHUB_TOKEN_URL = 'https://github.com/login/oauth/access_token';
const GITHUB_USER_URL = 'https://api.github.com/user';
const GITHUB_EMAILS_URL = 'https://api.github.com/user/emails';

const callbackSchema = z.object({
  code: z.string().min(1).max(256),
});

const GitHubTokenSchema = z.object({
  access_token: z.string(),
  token_type: z.string(),
});

const GitHubUserSchema = z.object({
  id: z.number(),
  login: z.string(),
  name: z.string().nullable().optional(),
  avatar_url: z.string().nullable().optional(),
});

const GitHubEmailSchema = z.array(
  z.object({
    email: z.string().email(),
    primary: z.boolean(),
    verified: z.boolean(),
  })
);

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

async function fetchGitHubUser(
  accessToken: string
): Promise<{ id: number; login: string; email: string; verifiedEmails: string[] }> {
  const [userRes, emailsRes] = await Promise.all([
    fetch(GITHUB_USER_URL, {
      headers: { Authorization: `Bearer ${accessToken}`, Accept: 'application/json' },
    }),
    fetch(GITHUB_EMAILS_URL, {
      headers: { Authorization: `Bearer ${accessToken}`, Accept: 'application/json' },
    }),
  ]);

  if (!userRes.ok) {
    throw new Error(`GitHub user fetch failed: ${userRes.status}`);
  }
  if (!emailsRes.ok) {
    throw new Error(`GitHub emails fetch failed: ${emailsRes.status}`);
  }

  const user = GitHubUserSchema.parse(await userRes.json());
  const emails = GitHubEmailSchema.parse(await emailsRes.json());

  const primary = emails.find((e) => e.primary && e.verified);
  if (!primary) {
    throw new Error('No verified primary email on GitHub account');
  }
  // All verified emails (incl. the primary) are the first-link match surface —
  // a user whose primary is a `noreply` address still binds via a verified
  // secondary (GHCLIAUTH-003 / ADR-066). Unverified emails are never included.
  const verifiedEmails = emails.filter((e) => e.verified).map((e) => e.email.toLowerCase().trim());

  return {
    id: user.id,
    login: user.login,
    email: primary.email.toLowerCase().trim(),
    verifiedEmails,
  };
}

async function revokeGitHubToken(accessToken: string): Promise<void> {
  try {
    const { clientId, clientSecret } = getGitHubCredentials();
    const auth = Buffer.from(`${clientId}:${clientSecret}`).toString('base64');
    await fetch(`https://api.github.com/applications/${clientId}/token`, {
      method: 'DELETE',
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

  let ghUser: { id: number; login: string; email: string; verifiedEmails: string[] };
  try {
    const accessToken = await exchangeCodeForToken(code);
    ghUser = await fetchGitHubUser(accessToken);
    // Fire-and-forget: revoke the GitHub token now that we have the profile
    revokeGitHubToken(accessToken).catch(() => {});
  } catch (err) {
    debug('github auth failed', { error: err instanceof Error ? err.message : String(err) });
    return c.json({ error: 'GitHub authentication failed' }, 401);
  }

  const sql = getClient();
  // Resolve the beta_users row: match on github_id, else first-link an active
  // invited row via any verified email, else create a pending row
  // (GHCLIAUTH-003). The active-status gate below stays here, per-caller.
  const { user, isNewPending, didFirstLink } = await linkOrCreateGitHubUser(sql, ghUser);

  if (isNewPending) {
    await insertAuditLog(sql, 'github_oauth_signup', user.email, {
      githubId: ghUser.id,
      githubLogin: ghUser.login,
    });
    debug('created pending user via github oauth', { email: user.email, githubId: ghUser.id });
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
    debug('github oauth for non-active user', { email: user.email, status: user.status });
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
