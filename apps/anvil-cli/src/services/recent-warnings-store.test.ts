import { mkdtemp, mkdir, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { afterEach, describe, expect, it } from 'vitest';
import type { Warning } from '@eddacraft/anvil-core';
import { safeCleanup } from '../../../../tools/test-utils/safe-cleanup.js';
import { loadRecentWarnings, saveRecentWarnings } from './recent-warnings-store.js';

const tempDirs: string[] = [];

function createWarning(overrides: Partial<Warning> = {}): Warning {
  return {
    id: 'AP-003',
    category: 'anti-pattern',
    severity: 'warning',
    confidence: 'high',
    title: 'Explicit any type',
    message: 'Avoid using any',
    explanation: 'It weakens type safety',
    suggestion: 'Use unknown with narrowing',
    location: {
      file: 'src/example.ts',
      line: 12,
      column: 5,
    },
    ...overrides,
  };
}

async function createWorkspace(): Promise<string> {
  const workspace = await mkdtemp(join(tmpdir(), 'anvil-cli-recent-warnings-'));
  tempDirs.push(workspace);
  return workspace;
}

afterEach(async () => {
  for (const dir of tempDirs.splice(0)) {
    await safeCleanup(dir);
  }
});

describe('recent-warnings-store', () => {
  it('returns empty list when store does not exist', async () => {
    const workspace = await createWorkspace();

    const warnings = await loadRecentWarnings(workspace);

    expect(warnings).toEqual([]);
  });

  it('saves and reloads warnings with generated IDs', async () => {
    const workspace = await createWorkspace();
    const warning = createWarning();

    await saveRecentWarnings(workspace, [warning]);

    const warnings = await loadRecentWarnings(workspace);

    expect(warnings).toHaveLength(1);
    expect(warnings[0].warningId).toBe('AP-003-src/example.ts:12');
    expect(warnings[0].title).toBe('Explicit any type');
  });

  it('returns empty list for corrupted JSON', async () => {
    const workspace = await createWorkspace();
    const cacheDir = join(workspace, '.anvil', 'cache');
    await mkdir(cacheDir, { recursive: true });
    await writeFile(join(cacheDir, 'recent-warnings.json'), '{{not json}}', 'utf-8');

    const warnings = await loadRecentWarnings(workspace);

    expect(warnings).toEqual([]);
  });

  it('returns empty list when warnings field is not an array', async () => {
    const workspace = await createWorkspace();
    const cacheDir = join(workspace, '.anvil', 'cache');
    await mkdir(cacheDir, { recursive: true });
    await writeFile(
      join(cacheDir, 'recent-warnings.json'),
      JSON.stringify({ version: '1.0.0', warnings: 'not-an-array' }),
      'utf-8'
    );

    const warnings = await loadRecentWarnings(workspace);

    expect(warnings).toEqual([]);
  });

  it('filters out warnings that fail schema validation', async () => {
    const workspace = await createWorkspace();
    const valid = createWarning();
    const invalid = { id: 'BAD', title: 'missing fields' };
    const cacheDir = join(workspace, '.anvil', 'cache');
    await mkdir(cacheDir, { recursive: true });
    await writeFile(
      join(cacheDir, 'recent-warnings.json'),
      JSON.stringify({
        version: '1.0.0',
        generatedAt: new Date().toISOString(),
        warnings: [valid, invalid],
      }),
      'utf-8'
    );

    const warnings = await loadRecentWarnings(workspace);

    expect(warnings).toHaveLength(1);
    expect(warnings[0].warningId).toBe('AP-003-src/example.ts:12');
  });
});
