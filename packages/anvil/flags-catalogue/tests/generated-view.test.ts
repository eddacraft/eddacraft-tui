import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

describe('FLAGCAT-014 generated catalogue view', () => {
  it('stays current with flags/surfaces.json and flags/manifest.json', () => {
    const repositoryRoot = fileURLToPath(new URL('../../../../', import.meta.url));
    const result = spawnSync(process.execPath, ['scripts/docs/generate-product-catalogue.mjs'], {
      cwd: repositoryRoot,
      encoding: 'utf8',
    });
    expect(result.status, result.stderr || result.stdout).toBe(0);
  });
});
