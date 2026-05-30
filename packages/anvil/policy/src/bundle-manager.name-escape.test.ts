/**
 * Regression tests for issue #1826 finding
 * `fnd_sig-feat-library-c6a8d0fc79-6f16_d7dba93911`:
 * "Bundle names can escape and delete outside the cache directory".
 *
 * A bundle name is joined into filesystem paths and used with recursive
 * `rmSync`/`mkdirSync`. An unsanitised name containing path separators or
 * `..` can escape the cache directory and delete arbitrary paths.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { BundleManager } from './bundle-manager.js';
import { existsSync, mkdirSync, mkdtempSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { safeCleanup } from '../../../../tools/test-utils/safe-cleanup.js';

describe('BundleManager — bundle name path escape (issue #1826)', () => {
  let tempRoot: string;
  let cacheDir: string;
  let manager: BundleManager;

  beforeEach(() => {
    tempRoot = mkdtempSync(join(tmpdir(), 'anvil-bundle-name-test-'));
    cacheDir = join(tempRoot, 'cache');
    mkdirSync(cacheDir, { recursive: true });
    manager = new BundleManager({ cacheDir, verifySignatures: false });
  });

  afterEach(async () => {
    await safeCleanup(tempRoot);
  });

  const UNSAFE = ['../escape', '..', '.', 'a/b', 'foo/../bar', 'sub\\win', '/abs/path', ''];

  it('rejects unsafe bundle names in addBundle', () => {
    for (const name of UNSAFE) {
      expect(
        () => manager.addBundle({ name, url: 'https://example.com/b.tar.gz' }),
        `name=${JSON.stringify(name)}`
      ).toThrow();
    }
  });

  it('rejects unsafe bundle names in the constructor', () => {
    expect(
      () =>
        new BundleManager({
          cacheDir,
          bundles: [{ name: '../escape', url: 'https://example.com/b.tar.gz' }],
        })
    ).toThrow();
  });

  it('accepts ordinary bundle names', () => {
    expect(() =>
      manager.addBundle({ name: 'my-bundle', url: 'https://example.com/b.tar.gz' })
    ).not.toThrow();
    expect(manager.getBundleNames()).toContain('my-bundle');
  });

  it('invalidateBundle refuses an escaping name and does not delete outside the cache dir', async () => {
    // Sentinel directory that a "../sentinel" name would target.
    const sentinel = join(tempRoot, 'sentinel');
    mkdirSync(sentinel, { recursive: true });
    writeFileSync(join(sentinel, 'keep.txt'), 'do not delete');

    // Poison the on-disk cache index with an entry keyed by an escaping name,
    // simulating a tampered index.json.
    writeFileSync(
      join(cacheDir, 'index.json'),
      JSON.stringify({
        version: 1,
        last_sync: 0,
        entries: {
          '../sentinel': {
            name: '../sentinel',
            url: 'https://example.com/x',
            path: sentinel,
            downloaded_at: 1,
            expires_at: 1,
            checksum: 'x',
            size_bytes: 0,
            signature_verified: false,
          },
        },
      })
    );

    const poisoned = new BundleManager({ cacheDir, verifySignatures: false });
    await expect(poisoned.invalidateBundle('../sentinel')).rejects.toThrow();
    expect(existsSync(join(sentinel, 'keep.txt'))).toBe(true);
  });
});
