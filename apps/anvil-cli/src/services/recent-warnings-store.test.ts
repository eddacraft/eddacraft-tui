import { mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { afterEach, describe, expect, it } from 'vitest';
import type { Warning } from '@eddacraft/anvil-core/antipattern';
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
    await rm(dir, { recursive: true, force: true });
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
});
