import { readFileSync, writeFileSync, mkdirSync, unlinkSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { homedir } from 'node:os';

export interface Credentials {
  license: string;
  refreshToken: string;
  expiresAt: string;
  email: string;
}

export function getCredentialsPath(): string {
  const configDir = process.env['XDG_CONFIG_HOME'] ?? join(homedir(), '.config');
  return join(configDir, 'anvil', 'credentials.json');
}

export function loadCredentials(): Credentials | null {
  const credPath = getCredentialsPath();
  try {
    const raw = readFileSync(credPath, 'utf-8');
    return JSON.parse(raw) as Credentials;
  } catch {
    return null;
  }
}

export function saveCredentials(creds: Credentials): void {
  const credPath = getCredentialsPath();
  mkdirSync(dirname(credPath), { recursive: true });
  writeFileSync(credPath, JSON.stringify(creds, null, 2), { mode: 0o600 });
}

export function clearCredentials(): void {
  const credPath = getCredentialsPath();
  try {
    unlinkSync(credPath);
  } catch {
    // ignore — file may not exist
  }
}

/**
 * Get a valid JWT licence, refreshing if needed.
 * Returns the JWT string or null if re-authentication is needed.
 */
export async function getValidLicence(apiUrl: string): Promise<string | null> {
  const creds = loadCredentials();
  if (!creds) return null;

  const expiresAt = new Date(creds.expiresAt);
  const oneHourFromNow = new Date(Date.now() + 60 * 60 * 1000);

  if (expiresAt > oneHourFromNow) {
    return creds.license;
  }

  try {
    const res = await fetch(`${apiUrl}/api/v1/auth/session/refresh`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ refreshToken: creds.refreshToken }),
    });

    if (!res.ok) {
      clearCredentials();
      return null;
    }

    const data = (await res.json()) as {
      license: string;
      refreshToken: string;
      expiresAt: string;
    };

    saveCredentials({
      license: data.license,
      refreshToken: data.refreshToken,
      expiresAt: data.expiresAt,
      email: creds.email,
    });

    return data.license;
  } catch {
    // Network error — return existing token if not fully expired
    if (expiresAt > new Date()) {
      return creds.license;
    }
    return null;
  }
}

/**
 * Error thrown when an authenticated session is required but not present.
 */
export class AuthRequiredError extends Error {
  constructor(message = 'Not authenticated. Run `anvil auth login` to authenticate.') {
    super(message);
    this.name = 'AuthRequiredError';
  }
}

/**
 * Ensure the user is authenticated, with a helpful error message.
 *
 * Throws {@link AuthRequiredError} if no valid licence is available.
 */
export async function requireAuth(apiUrl: string): Promise<string> {
  const license = await getValidLicence(apiUrl);
  if (!license) {
    throw new AuthRequiredError();
  }
  return license;
}
