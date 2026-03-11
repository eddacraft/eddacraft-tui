import { readFileSync, writeFileSync, mkdirSync, unlinkSync, existsSync, chmodSync } from 'node:fs';
import { join } from 'node:path';
import { getAuthDir } from './auth-store.js';
import { debug } from '../utils/output.js';

const LICENCE_FILENAME = 'license';

function getUserLicencePath(): string {
  return join(getAuthDir(), LICENCE_FILENAME);
}

export function resolveLicencePath(projectRoot?: string): string | null {
  const envPath = process.env['ANVIL_LICENSE'];
  if (envPath && existsSync(envPath)) return envPath;

  if (projectRoot) {
    const projectPath = join(projectRoot, '.anvil', LICENCE_FILENAME);
    if (existsSync(projectPath)) return projectPath;
  }

  const userPath = getUserLicencePath();
  if (existsSync(userPath)) return userPath;

  return null;
}

export function loadLicence(projectRoot?: string): string | null {
  const path = resolveLicencePath(projectRoot);
  if (!path) return null;

  try {
    return readFileSync(path, 'utf-8').trim();
  } catch {
    debug('loadLicence: failed to read licence file');
    return null;
  }
}

export function saveLicence(jwt: string): void {
  const dir = getAuthDir();
  if (!existsSync(dir)) {
    mkdirSync(dir, { recursive: true, mode: 0o700 });
  }

  const path = getUserLicencePath();
  writeFileSync(path, jwt, { mode: 0o600 });
  chmodSync(path, 0o600);
}

export function clearLicence(): void {
  const path = getUserLicencePath();
  if (existsSync(path)) {
    unlinkSync(path);
  }
}
