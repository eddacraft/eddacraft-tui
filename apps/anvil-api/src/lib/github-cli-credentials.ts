/**
 * GHCLIAUTH-002 (ADR-066): credentials for the dedicated "Anvil CLI" GitHub
 * OAuth app that backs the CLI device-authorization-grant login. Sourced from
 * the Key Vault secrets `github-cli-client-id` / `github-cli-client-secret` and
 * wired into anvil-api **only** (see `infra/src/vercel.ts`) — kept separate from
 * the `eddacraft Docs` OAuth app so CLI login and docs auth do not share rate
 * limits, consent branding, or audit trails.
 */

export interface GitHubCliCredentials {
  clientId: string;
  clientSecret: string;
}

/**
 * Read the Anvil CLI OAuth app credentials from the environment. Throws if
 * either is missing — call at request time on the device-flow broker paths
 * (GHCLIAUTH-004/-005).
 */
export function getGitHubCliCredentials(): GitHubCliCredentials {
  const clientId = process.env['GITHUB_CLI_CLIENT_ID'];
  const clientSecret = process.env['GITHUB_CLI_CLIENT_SECRET'];
  // Two distinct throws so a boot/health log identifies exactly which secret is
  // missing during provisioning. Never interpolate the values into the message.
  if (!clientId) {
    throw new Error('GITHUB_CLI_CLIENT_ID is required');
  }
  if (!clientSecret) {
    throw new Error('GITHUB_CLI_CLIENT_SECRET is required');
  }
  return { clientId, clientSecret };
}

/**
 * Boot/health probe: report whether the CLI OAuth credentials are present,
 * without throwing. Absence is **informational** — the device-flow login is not
 * live until GHCLIAUTH-005/-006, so a missing credential must not degrade
 * overall service health or block a deploy before the secrets are provisioned.
 */
export function verifyGitHubCliCredentials(): { ok: true } | { ok: false; error: string } {
  try {
    getGitHubCliCredentials();
    return { ok: true };
  } catch (err) {
    return { ok: false, error: err instanceof Error ? err.message : String(err) };
  }
}
