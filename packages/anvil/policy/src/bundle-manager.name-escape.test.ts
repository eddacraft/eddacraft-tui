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

  it('getBundle refuses a cache-entry path that escapes the cache dir (tampered index)', async () => {
    // An attacker who can write index.json uses a safe bundle *name* but an
    // out-of-cache *path*; getBundle must not hand that path to the loader.
    const outside = join(tempRoot, 'outside-policies');
    mkdirSync(outside, { recursive: true });
    writeFileSync(join(outside, 'evil.rego'), 'package evil');

    writeFileSync(
      join(cacheDir, 'index.json'),
      JSON.stringify({
        version: 1,
        last_sync: 0,
        entries: {
          'safe-name': {
            name: 'safe-name',
            url: 'https://example.com/x',
            path: outside, // escapes cacheDir despite the safe key
            downloaded_at: 1,
            expires_at: Date.now() + 60_000,
            checksum: 'x',
            size_bytes: 0,
            signature_verified: true,
          },
        },
      })
    );

    const poisoned = new BundleManager({ cacheDir, verifySignatures: false });
    expect(await poisoned.getBundle('safe-name')).toBeNull();
  });

  it('getBundle returns the path for a legitimately-cached bundle', async () => {
    const inside = join(cacheDir, 'good-bundle');
    mkdirSync(inside, { recursive: true });
    writeFileSync(
      join(cacheDir, 'index.json'),
      JSON.stringify({
        version: 1,
        last_sync: 0,
        entries: {
          'good-bundle': {
            name: 'good-bundle',
            url: 'https://example.com/x',
            path: inside,
            downloaded_at: 1,
            expires_at: Date.now() + 60_000,
            checksum: 'x',
            size_bytes: 0,
            signature_verified: true,
          },
        },
      })
    );

    const mgr = new BundleManager({ cacheDir, verifySignatures: false });
    expect(await mgr.getBundle('good-bundle')).toBe(inside);
  });

  /**
   * Regression for fnd_sig-feat-library-255c3bcb97-3572_34f9e7fcbc:
   * downloadBundle unexpired-cache and 304 paths must not return a tampered
   * out-of-cache entry.path (getBundle already refuses; these paths did not).
   */
  it('downloadBundle refuses an unexpired cache-entry path that escapes the cache dir', async () => {
    const outside = join(tempRoot, 'outside-policies');
    mkdirSync(outside, { recursive: true });
    writeFileSync(join(outside, 'evil.rego'), 'package evil');

    // Attacker creates cacheDir/safe-name so the unexpired shortcut condition
    // (existsSync(bundleDir) + future expires_at) passes.
    const inside = join(cacheDir, 'safe-name');
    mkdirSync(inside, { recursive: true });

    writeFileSync(
      join(cacheDir, 'index.json'),
      JSON.stringify({
        version: 1,
        last_sync: 0,
        entries: {
          'safe-name': {
            name: 'safe-name',
            url: 'https://example.com/x',
            path: outside, // escapes cacheDir despite the safe key
            downloaded_at: 1,
            expires_at: Date.now() + 60_000,
            checksum: 'x',
            size_bytes: 0,
            signature_verified: true,
          },
        },
      })
    );

    const poisoned = new BundleManager({
      cacheDir,
      verifySignatures: false,
      bundles: [{ name: 'safe-name', url: 'https://example.com/x' }],
    });

    const result = await poisoned.downloadBundle('safe-name');
    // Must never hand the escaped path to callers as a trusted bundle dir.
    expect(result.path).not.toBe(outside);
    if (result.success && result.path) {
      expect(result.path === inside || result.path.startsWith(cacheDir + '/')).toBe(true);
    }
  });

  it('downloadBundle refuses a 304 cache-entry path that escapes the cache dir', async () => {
    const { createServer } = await import('node:http');
    const outside = join(tempRoot, 'outside-304');
    mkdirSync(outside, { recursive: true });
    writeFileSync(join(outside, 'evil.rego'), 'package evil');

    const inside = join(cacheDir, 'safe-304');
    mkdirSync(inside, { recursive: true });

    const server = createServer((req, res) => {
      if (req.headers['if-none-match'] === '"poison-etag"') {
        res.statusCode = 304;
        res.end();
        return;
      }
      res.statusCode = 200;
      res.setHeader('ETag', '"poison-etag"');
      res.end('not-a-real-bundle');
    });

    await new Promise<void>((resolve) => {
      server.listen(0, '127.0.0.1', () => resolve());
    });
    const addr = server.address();
    if (!addr || typeof addr === 'string') {
      server.close();
      throw new Error('failed to bind mock server');
    }
    const port = addr.port;

    try {
      writeFileSync(
        join(cacheDir, 'index.json'),
        JSON.stringify({
          version: 1,
          last_sync: 0,
          entries: {
            'safe-304': {
              name: 'safe-304',
              url: `http://127.0.0.1:${port}/bundle.tar.gz`,
              path: outside,
              downloaded_at: 1,
              // Expired so downloadBundle hits the network and can receive 304
              expires_at: Date.now() - 1,
              checksum: 'x',
              size_bytes: 0,
              signature_verified: true,
              etag: '"poison-etag"',
            },
          },
        })
      );

      const poisoned = new BundleManager({
        cacheDir,
        verifySignatures: false,
        bundles: [
          {
            name: 'safe-304',
            url: `http://127.0.0.1:${port}/bundle.tar.gz`,
          },
        ],
      });

      const result = await poisoned.downloadBundle('safe-304');
      expect(result.path).not.toBe(outside);
      if (result.success && result.path) {
        expect(result.path === inside || result.path.startsWith(cacheDir + '/')).toBe(true);
      }
    } finally {
      await new Promise<void>((resolve, reject) => {
        server.close((err) => (err ? reject(err) : resolve()));
      });
    }
  });
});
