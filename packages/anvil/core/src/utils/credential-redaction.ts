/**
 * Credential-shape detection and Git remote userinfo stripping.
 *
 * Used so provenance, authorship records, and Git notes never persist
 * Copilot-like tokens or credential-bearing remotes.
 */

const REDACTED = '[redacted]';

const CREDENTIAL_ENV_NAME =
  /(?:^|_)(?:TOKEN|SECRET|PASSWORD|PASSWD|APIKEY|API_KEY|PAT|CREDENTIAL|PASS)(?:_|$)/i;

const GITHUB_TOKEN_PREFIX = /^(ghp_|ghu_|ghs_|gho_|ghr_|github_pat_)/i;
const GITLAB_TOKEN_PREFIX = /^(glpat-|gloas-|glrt-)/i;
const JWT_SHAPE = /^eyJ[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+$/;
const HEX_TOKEN = /^[0-9a-fA-F]{40,}$/;
const BEARER = /^Bearer\s+\S+/i;
const SK_PREFIX = /^sk-[A-Za-z0-9_-]{16,}$/;

/**
 * True when an environment variable name is a credential, not a session id.
 */
export function isCredentialEnvVarName(name: string): boolean {
  return CREDENTIAL_ENV_NAME.test(name);
}

/**
 * True when a value looks like a token, PAT, JWT, or similar secret.
 */
export function isCredentialShapedValue(value: string): boolean {
  const trimmed = value.trim();
  if (!trimmed) return false;
  return (
    GITHUB_TOKEN_PREFIX.test(trimmed) ||
    GITLAB_TOKEN_PREFIX.test(trimmed) ||
    JWT_SHAPE.test(trimmed) ||
    HEX_TOKEN.test(trimmed) ||
    BEARER.test(trimmed) ||
    SK_PREFIX.test(trimmed)
  );
}

/**
 * Replace a token-shaped value with a stable placeholder; leave ordinary ids.
 */
export function redactCredentialShapedValue(value: string): string {
  return isCredentialShapedValue(value) ? REDACTED : value;
}

/**
 * Strip URL userinfo from Git remotes before persistence or display.
 *
 * HTTPS remotes always drop userinfo (passwords and token usernames).
 * SSH URLs drop userinfo only when a password or token-shaped username is present,
 * so `ssh://git@host/path` stays stable. SCP-style `git@host:path` is unchanged.
 */
export function stripGitRemoteUserinfo(url: string): string {
  const trimmed = url.trim();
  if (!trimmed) return trimmed;

  try {
    const parsed = new URL(trimmed);
    if (!parsed.username && !parsed.password) {
      return trimmed;
    }

    const protocol = parsed.protocol.toLowerCase();
    const isHttp = protocol === 'http:' || protocol === 'https:';
    const hasSecretUserinfo =
      parsed.password.length > 0 || isCredentialShapedValue(parsed.username);

    if (isHttp || hasSecretUserinfo) {
      parsed.username = '';
      parsed.password = '';
      return parsed.toString();
    }

    return trimmed;
  } catch {
    return trimmed;
  }
}
