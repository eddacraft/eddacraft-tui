import { readFileSync, writeFileSync, mkdirSync, unlinkSync, existsSync, chmodSync } from 'node:fs';
import { join } from 'node:path';
import { homedir } from 'node:os';
import { z } from 'zod';

const StoredAuthSchema = z.object({
  token: z.string(),
  user: z.object({ email: z.string() }),
  scopes: z.array(z.string()),
  expiresAt: z.string(),
  verifiedAt: z.string(),
});

export type StoredAuth = z.infer<typeof StoredAuthSchema>;

/** Override for testing. */
let _authDirOverride: string | null = null;

/** Set a custom auth directory (for testing). */
export function setAuthDir(dir: string | null): void {
  _authDirOverride = dir;
}

function getAuthDir(): string {
  return _authDirOverride ?? join(homedir(), '.anvil');
}

function getAuthPath(): string {
  return join(getAuthDir(), 'auth.json');
}

/**
 * Load stored auth credentials.
 * Returns null if no auth file exists or if it's expired.
 */
export function loadAuth(): StoredAuth | null {
  const authPath = getAuthPath();

  try {
    const raw = readFileSync(authPath, 'utf-8');
    const data = StoredAuthSchema.parse(JSON.parse(raw));

    // Check local expiry (server is the real authority, but we avoid
    // obviously-expired tokens hitting the network)
    const expiry = Date.parse(data.expiresAt);
    if (Number.isNaN(expiry) || expiry < Date.now()) {
      return null;
    }

    return data;
  } catch {
    return null;
  }
}

/**
 * Save auth credentials to disk with restrictive permissions.
 */
export function saveAuth(auth: StoredAuth): void {
  const dir = getAuthDir();
  if (!existsSync(dir)) {
    mkdirSync(dir, { recursive: true, mode: 0o700 });
  }

  const authPath = getAuthPath();
  writeFileSync(authPath, JSON.stringify(auth, null, 2), { mode: 0o600 });
  // Explicitly set permissions (in case writeFile mode is ignored on some OS)
  chmodSync(authPath, 0o600);
}

/**
 * Clear stored auth credentials.
 */
export function clearAuth(): void {
  const authPath = getAuthPath();
  if (existsSync(authPath)) {
    unlinkSync(authPath);
  }
}

/**
 * Check if the user is currently authenticated.
 */
export function isAuthenticated(): boolean {
  return loadAuth() !== null;
}
