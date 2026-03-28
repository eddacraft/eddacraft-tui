import { readFileSync, writeFileSync, existsSync } from 'node:fs';
import { join } from 'node:path';
import { getAuthDir } from './auth-store.js';
import { saveLicence, clearLicence } from './licence-store.js';
import { getApiUrl } from './api-client.js';
import { debug } from '../utils/output.js';

export const REFRESH_COOLDOWN_MS = 60_000;

let _refreshStatePathOverride: string | null = null;

export function _setRefreshStatePath(path: string | null): void {
  _refreshStatePathOverride = path;
}

function getRefreshStatePath(): string {
  return _refreshStatePathOverride ?? join(getAuthDir(), 'refresh-state');
}

export function getLastRefreshAttempt(): number {
  const path = getRefreshStatePath();
  try {
    if (!existsSync(path)) return 0;
    const raw = readFileSync(path, 'utf-8').trim();
    return parseInt(raw, 10) || 0;
  } catch {
    return 0;
  }
}

function saveRefreshAttempt(): void {
  writeFileSync(getRefreshStatePath(), String(Date.now()));
}

export type RefreshResult = 'refreshed' | 'revoked' | 'skipped_cooldown' | 'error';

type FetchFn = typeof globalThis.fetch;

export async function scheduleRefresh(
  token: string,
  fetchFn: FetchFn = globalThis.fetch
): Promise<RefreshResult> {
  const lastAttempt = getLastRefreshAttempt();
  if (Date.now() - lastAttempt < REFRESH_COOLDOWN_MS) {
    debug('licence refresh: skipped (cooldown)');
    return 'skipped_cooldown';
  }

  saveRefreshAttempt();

  try {
    const apiUrl = getApiUrl();
    const response = await fetchFn(`${apiUrl}/api/v1/auth/license/refresh`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ token }),
      signal: AbortSignal.timeout(10_000),
    });

    const data = (await response.json()) as { license?: string; valid?: boolean };

    if (data.license) {
      saveLicence(data.license);
      debug('licence refresh: success');
      return 'refreshed';
    }

    if (data.valid === false) {
      clearLicence();
      debug('licence refresh: revoked');
      return 'revoked';
    }

    debug('licence refresh: unexpected response');
    return 'error';
  } catch (err) {
    debug(`licence refresh: failed: ${err}`);
    return 'error';
  }
}
