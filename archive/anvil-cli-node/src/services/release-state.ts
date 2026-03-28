import { existsSync, mkdirSync, readFileSync, writeFileSync, unlinkSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { debug } from '../utils/output.js';
import type { ReleaseState } from './release-types.js';

const STATE_FILENAME = '.anvil/release-state.json';

export function getStatePath(workspaceRoot: string): string {
  return join(workspaceRoot, STATE_FILENAME);
}

export function loadReleaseState(workspaceRoot: string): ReleaseState | null {
  const filePath = getStatePath(workspaceRoot);
  if (!existsSync(filePath)) return null;

  try {
    const raw = readFileSync(filePath, 'utf8');
    return JSON.parse(raw) as ReleaseState;
  } catch {
    debug('loadReleaseState: failed to read/parse release state, returning null');
    return null;
  }
}

export function saveReleaseState(workspaceRoot: string, state: ReleaseState): void {
  const filePath = getStatePath(workspaceRoot);
  const dir = dirname(filePath);
  if (!existsSync(dir)) {
    mkdirSync(dir, { recursive: true });
  }
  state.updatedAt = new Date().toISOString();
  writeFileSync(filePath, JSON.stringify(state, null, 2) + '\n', 'utf8');
}

export function clearReleaseState(workspaceRoot: string): void {
  const filePath = getStatePath(workspaceRoot);
  if (existsSync(filePath)) {
    unlinkSync(filePath);
  }
}
