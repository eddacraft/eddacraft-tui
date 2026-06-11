import { z } from 'zod';

/**
 * GitHub identity fetch shared by the web OAuth callback
 * (`routes/auth-github.ts`) and the CLI device-flow poll broker
 * (`routes/auth-github-device.ts`, GHCLIAUTH-005). Identity is derived solely
 * from the access token — the numeric `id` is the authoritative linking key
 * (ADR-066 decision 4) and only `verified` emails ever reach the first-link
 * match surface.
 */

const GITHUB_USER_URL = 'https://api.github.com/user';
const GITHUB_EMAILS_URL = 'https://api.github.com/user/emails';
// ADR-066 ops precondition: every github.com fetch on the login hot path
// carries an explicit timeout below the Vercel function ceiling.
const UPSTREAM_TIMEOUT_MS = 8_000;

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

export interface GitHubIdentity {
  id: number;
  login: string;
  email: string;
  verifiedEmails: string[];
}

export async function fetchGitHubUser(accessToken: string): Promise<GitHubIdentity> {
  const [userRes, emailsRes] = await Promise.all([
    fetch(GITHUB_USER_URL, {
      headers: { Authorization: `Bearer ${accessToken}`, Accept: 'application/json' },
      signal: AbortSignal.timeout(UPSTREAM_TIMEOUT_MS),
    }),
    fetch(GITHUB_EMAILS_URL, {
      headers: { Authorization: `Bearer ${accessToken}`, Accept: 'application/json' },
      signal: AbortSignal.timeout(UPSTREAM_TIMEOUT_MS),
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
