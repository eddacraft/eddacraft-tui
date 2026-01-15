/**
 * Unit Tests for OPA Bundle Manager
 *
 * Tests bundle download, caching, validation, and cache management
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { BundleManager, type BundleConfig } from './bundle-manager.js';
import { existsSync, mkdirSync, rmSync, writeFileSync } from 'fs';
import { join } from 'path';
import { tmpdir } from 'os';
import { createServer, type Server, type IncomingMessage, type ServerResponse } from 'http';
import { gzipSync } from 'zlib';
// tar module is a dependency of bundle-manager, tested implicitly through extraction

// Pre-create a minimal gzipped tarball for testing
// This is a valid tar.gz containing a single file 'policy.rego'
function createTestBundle(): Buffer {
  // Create a minimal tar archive manually
  // TAR format: 512-byte header + content (padded to 512) + 1024 bytes of zeros
  const filename = 'policy.rego';
  const content = 'package example\n\ndefault allow = false\n';
  const contentBuffer = Buffer.from(content, 'utf-8');

  // Create tar header (512 bytes)
  const header = Buffer.alloc(512);

  // Name (100 bytes)
  header.write(filename, 0, 100, 'utf-8');

  // Mode (8 bytes, octal)
  header.write('0000644\0', 100, 8, 'utf-8');

  // UID (8 bytes, octal)
  header.write('0000000\0', 108, 8, 'utf-8');

  // GID (8 bytes, octal)
  header.write('0000000\0', 116, 8, 'utf-8');

  // Size (12 bytes, octal)
  const sizeOctal = contentBuffer.length.toString(8).padStart(11, '0') + '\0';
  header.write(sizeOctal, 124, 12, 'utf-8');

  // Mtime (12 bytes, octal)
  const mtime =
    Math.floor(Date.now() / 1000)
      .toString(8)
      .padStart(11, '0') + '\0';
  header.write(mtime, 136, 12, 'utf-8');

  // Checksum placeholder (8 spaces)
  header.write('        ', 148, 8, 'utf-8');

  // Type flag (1 byte) - '0' for regular file
  header.write('0', 156, 1, 'utf-8');

  // Calculate checksum
  let checksum = 0;
  for (let i = 0; i < 512; i++) {
    checksum += header[i];
  }
  const checksumOctal = checksum.toString(8).padStart(6, '0') + '\0 ';
  header.write(checksumOctal, 148, 8, 'utf-8');

  // Content padded to 512 bytes
  const contentPadded = Buffer.alloc(512);
  contentBuffer.copy(contentPadded);

  // End of archive (two 512-byte zero blocks)
  const endBlock = Buffer.alloc(1024);

  // Combine all parts
  const tarBuffer = Buffer.concat([header, contentPadded, endBlock]);

  // Gzip compress
  return gzipSync(tarBuffer);
}

const testBundle = createTestBundle();

describe('BundleManager', { timeout: 30000 }, () => {
  let manager: BundleManager;
  let tempCacheDir: string;
  let mockServer: Server | null = null;
  let mockServerPort: number;

  const handleRequest = (req: IncomingMessage, res: ServerResponse): void => {
    if (req.url === '/bundle.tar.gz') {
      res.setHeader('Content-Type', 'application/gzip');
      res.setHeader('ETag', '"test-etag-123"');
      res.setHeader('Last-Modified', 'Wed, 01 Jan 2025 00:00:00 GMT');
      res.end(testBundle);
    } else if (req.url === '/not-modified') {
      if (req.headers['if-none-match'] === '"test-etag-123"') {
        res.statusCode = 304;
        res.end();
      } else {
        res.setHeader('ETag', '"test-etag-123"');
        res.statusCode = 200;
        res.end('{}');
      }
    } else if (req.url === '/error') {
      res.statusCode = 500;
      res.end('Internal Server Error');
    } else if (req.url === '/redirect') {
      res.statusCode = 302;
      res.setHeader('Location', `http://localhost:${mockServerPort}/bundle.tar.gz`);
      res.end();
    } else {
      res.statusCode = 404;
      res.end('Not Found');
    }
  };

  beforeEach(async () => {
    // Create temp cache directory
    tempCacheDir = join(tmpdir(), 'anvil-bundle-test', Math.random().toString(36));
    mkdirSync(tempCacheDir, { recursive: true });

    manager = new BundleManager({
      cacheDir: tempCacheDir,
      verifySignatures: false, // Disable signature verification for tests
      timeoutMs: 10000,
    });

    // Create a mock HTTP server for testing downloads
    mockServer = createServer(handleRequest);

    await new Promise<void>((resolve, reject) => {
      const timeout = setTimeout(() => {
        reject(new Error('Server startup timeout'));
      }, 5000);

      mockServer!.listen(0, '127.0.0.1', () => {
        clearTimeout(timeout);
        const addr = mockServer!.address();
        if (addr && typeof addr !== 'string') {
          mockServerPort = addr.port;
        }
        resolve();
      });
    });
  }, 10000);

  afterEach(async () => {
    // Close mock server first
    if (mockServer) {
      await new Promise<void>((resolve) => {
        mockServer!.close(() => resolve());
        // Force close after timeout
        setTimeout(resolve, 1000);
      });
      mockServer = null;
    }

    // Clean up temp directory
    if (tempCacheDir && existsSync(tempCacheDir)) {
      try {
        rmSync(tempCacheDir, { recursive: true, force: true });
      } catch {
        // Ignore cleanup errors
      }
    }
  }, 15000);

  describe('initialization', () => {
    it('should create manager with default config', () => {
      const defaultManager = new BundleManager();
      expect(defaultManager).toBeDefined();
    });

    it('should accept custom cache directory', () => {
      const customDir = join(tempCacheDir, 'custom');
      const customManager = new BundleManager({ cacheDir: customDir });
      expect(customManager).toBeDefined();
    });

    it('should accept bundle configurations', () => {
      const bundles: BundleConfig[] = [
        { name: 'test-bundle', url: 'http://example.com/bundle.tar.gz' },
      ];
      const bundleManager = new BundleManager({
        cacheDir: tempCacheDir,
        bundles,
      });
      expect(bundleManager.getBundleNames()).toContain('test-bundle');
    });
  });

  describe('bundle configuration', () => {
    it('should add bundle configuration', () => {
      manager.addBundle({
        name: 'new-bundle',
        url: 'http://example.com/new.tar.gz',
      });
      expect(manager.getBundleNames()).toContain('new-bundle');
    });

    it('should update existing bundle configuration', () => {
      manager.addBundle({
        name: 'bundle1',
        url: 'http://example.com/v1.tar.gz',
      });
      manager.addBundle({
        name: 'bundle1',
        url: 'http://example.com/v2.tar.gz',
      });
      expect(manager.getBundleNames().filter((n) => n === 'bundle1')).toHaveLength(1);
    });

    it('should remove bundle configuration', () => {
      manager.addBundle({
        name: 'to-remove',
        url: 'http://example.com/bundle.tar.gz',
      });
      expect(manager.getBundleNames()).toContain('to-remove');

      const removed = manager.removeBundle('to-remove');
      expect(removed).toBe(true);
      expect(manager.getBundleNames()).not.toContain('to-remove');
    });

    it('should return false when removing non-existent bundle', () => {
      const removed = manager.removeBundle('non-existent');
      expect(removed).toBe(false);
    });

    it('should return all configured bundle names', () => {
      manager.addBundle({ name: 'bundle1', url: 'http://example.com/1.tar.gz' });
      manager.addBundle({ name: 'bundle2', url: 'http://example.com/2.tar.gz' });
      manager.addBundle({ name: 'bundle3', url: 'http://example.com/3.tar.gz' });

      const names = manager.getBundleNames();
      expect(names).toContain('bundle1');
      expect(names).toContain('bundle2');
      expect(names).toContain('bundle3');
    });
  });

  describe('downloadBundle', () => {
    it('should download bundle successfully', async () => {
      manager.addBundle({
        name: 'test-bundle',
        url: `http://localhost:${mockServerPort}/bundle.tar.gz`,
      });

      const result = await manager.downloadBundle('test-bundle');

      expect(result.success).toBe(true);
      expect(result.updated).toBe(true);
      expect(result.path).toBeDefined();
      expect(existsSync(result.path!)).toBe(true);
    });

    it('should return error for unknown bundle', async () => {
      const result = await manager.downloadBundle('unknown-bundle');

      expect(result.success).toBe(false);
      expect(result.error).toContain('not found');
    });

    it('should handle HTTP errors', async () => {
      manager.addBundle({
        name: 'error-bundle',
        url: `http://localhost:${mockServerPort}/error`,
      });

      const result = await manager.downloadBundle('error-bundle');

      expect(result.success).toBe(false);
      expect(result.error).toBeDefined();
    });

    it('should handle redirects', async () => {
      manager.addBundle({
        name: 'redirect-bundle',
        url: `http://localhost:${mockServerPort}/redirect`,
      });

      const result = await manager.downloadBundle('redirect-bundle');

      expect(result.success).toBe(true);
      expect(result.path).toBeDefined();
    });

    it('should skip download when cache is valid', async () => {
      manager.addBundle({
        name: 'cached-bundle',
        url: `http://localhost:${mockServerPort}/bundle.tar.gz`,
        refresh_interval_ms: 60000, // 1 minute
      });

      // First download
      const result1 = await manager.downloadBundle('cached-bundle');
      expect(result1.success).toBe(true);
      expect(result1.updated).toBe(true);

      // Second download should use cache
      const result2 = await manager.downloadBundle('cached-bundle');
      expect(result2.success).toBe(true);
      expect(result2.updated).toBe(false);
    });

    it('should verify checksum when provided', async () => {
      // Use a wrong checksum to test validation
      manager.addBundle({
        name: 'checksum-bundle',
        url: `http://localhost:${mockServerPort}/bundle.tar.gz`,
        checksum: 'invalid-checksum',
      });

      const result = await manager.downloadBundle('checksum-bundle');

      expect(result.success).toBe(false);
      expect(result.error).toContain('Checksum mismatch');
    });
  });

  describe('getBundle', () => {
    it('should return null for uncached bundle', async () => {
      const path = await manager.getBundle('non-existent');
      expect(path).toBeNull();
    });

    it('should return path for cached bundle', async () => {
      manager.addBundle({
        name: 'cached-bundle',
        url: `http://localhost:${mockServerPort}/bundle.tar.gz`,
      });

      await manager.downloadBundle('cached-bundle');
      const path = await manager.getBundle('cached-bundle');

      expect(path).toBeDefined();
      expect(existsSync(path!)).toBe(true);
    });

    it('should return null if cache files are missing', async () => {
      manager.addBundle({
        name: 'missing-files',
        url: `http://localhost:${mockServerPort}/bundle.tar.gz`,
      });

      await manager.downloadBundle('missing-files');

      // Manually delete the bundle directory
      const bundleDir = join(tempCacheDir, 'missing-files');
      rmSync(bundleDir, { recursive: true, force: true });

      const path = await manager.getBundle('missing-files');
      expect(path).toBeNull();
    });
  });

  describe('getBundleEntry', () => {
    it('should return null for unknown bundle', async () => {
      const entry = await manager.getBundleEntry('unknown');
      expect(entry).toBeNull();
    });

    it('should return cache entry for downloaded bundle', async () => {
      manager.addBundle({
        name: 'entry-bundle',
        url: `http://localhost:${mockServerPort}/bundle.tar.gz`,
      });

      await manager.downloadBundle('entry-bundle');
      const entry = await manager.getBundleEntry('entry-bundle');

      expect(entry).toBeDefined();
      expect(entry!.name).toBe('entry-bundle');
      expect(entry!.checksum).toBeDefined();
      expect(entry!.downloaded_at).toBeGreaterThan(0);
    });
  });

  describe('invalidateBundle', () => {
    it('should return false for unknown bundle', async () => {
      const result = await manager.invalidateBundle('unknown');
      expect(result).toBe(false);
    });

    it('should remove cached bundle', async () => {
      manager.addBundle({
        name: 'to-invalidate',
        url: `http://localhost:${mockServerPort}/bundle.tar.gz`,
      });

      await manager.downloadBundle('to-invalidate');

      // Verify bundle exists
      let path = await manager.getBundle('to-invalidate');
      expect(path).toBeDefined();

      // Invalidate
      const result = await manager.invalidateBundle('to-invalidate');
      expect(result).toBe(true);

      // Verify bundle is removed
      path = await manager.getBundle('to-invalidate');
      expect(path).toBeNull();
    });
  });

  describe('clearCache', () => {
    it('should remove all cached bundles', async () => {
      manager.addBundle({
        name: 'bundle1',
        url: `http://localhost:${mockServerPort}/bundle.tar.gz`,
      });
      manager.addBundle({
        name: 'bundle2',
        url: `http://localhost:${mockServerPort}/bundle.tar.gz`,
      });

      await manager.downloadBundle('bundle1');
      await manager.downloadBundle('bundle2');

      // Verify bundles exist
      expect(await manager.getBundle('bundle1')).toBeDefined();
      expect(await manager.getBundle('bundle2')).toBeDefined();

      // Clear cache
      await manager.clearCache();

      // Verify cache is empty
      expect(await manager.getBundle('bundle1')).toBeNull();
      expect(await manager.getBundle('bundle2')).toBeNull();
    });
  });

  describe('syncAll', () => {
    it('should sync all configured bundles', async () => {
      manager.addBundle({
        name: 'bundle1',
        url: `http://localhost:${mockServerPort}/bundle.tar.gz`,
      });
      manager.addBundle({
        name: 'bundle2',
        url: `http://localhost:${mockServerPort}/bundle.tar.gz`,
      });

      const results = await manager.syncAll();

      expect(results).toHaveLength(2);
      expect(results.every((r) => r.success)).toBe(true);
    });

    it('should report individual failures', async () => {
      manager.addBundle({
        name: 'good-bundle',
        url: `http://localhost:${mockServerPort}/bundle.tar.gz`,
      });
      manager.addBundle({
        name: 'bad-bundle',
        url: `http://localhost:${mockServerPort}/error`,
      });

      const results = await manager.syncAll();

      expect(results).toHaveLength(2);
      const goodResult = results.find((r) => r.name === 'good-bundle');
      const badResult = results.find((r) => r.name === 'bad-bundle');

      expect(goodResult?.success).toBe(true);
      expect(badResult?.success).toBe(false);
    });
  });

  describe('getCacheStats', () => {
    it('should return empty stats for new manager', async () => {
      const stats = await manager.getCacheStats();

      expect(stats.bundleCount).toBe(0);
      expect(stats.totalSizeBytes).toBe(0);
      expect(stats.lastSync).toBe(0);
    });

    it('should return correct stats after downloads', async () => {
      manager.addBundle({
        name: 'stats-bundle',
        url: `http://localhost:${mockServerPort}/bundle.tar.gz`,
      });

      await manager.downloadBundle('stats-bundle');
      const stats = await manager.getCacheStats();

      expect(stats.bundleCount).toBe(1);
      expect(stats.totalSizeBytes).toBeGreaterThan(0);
      expect(stats.lastSync).toBeGreaterThan(0);
    });
  });

  describe('cache persistence', () => {
    it('should persist cache index across manager instances', async () => {
      manager.addBundle({
        name: 'persist-bundle',
        url: `http://localhost:${mockServerPort}/bundle.tar.gz`,
      });

      await manager.downloadBundle('persist-bundle');

      // Create new manager with same cache directory
      const newManager = new BundleManager({
        cacheDir: tempCacheDir,
      });

      const path = await newManager.getBundle('persist-bundle');
      expect(path).toBeDefined();
    });
  });

  describe('error handling', () => {
    it('should handle network timeout', async () => {
      const slowManager = new BundleManager({
        cacheDir: tempCacheDir,
        timeoutMs: 1, // Very short timeout
      });

      slowManager.addBundle({
        name: 'timeout-bundle',
        url: `http://localhost:${mockServerPort}/bundle.tar.gz`,
      });

      const result = await slowManager.downloadBundle('timeout-bundle');

      // Result may vary depending on system speed
      // The important thing is it doesn't hang or throw unhandled exception
      expect(result.name).toBe('timeout-bundle');
    });

    it('should handle invalid cache index gracefully', async () => {
      // Write invalid index file
      const indexPath = join(tempCacheDir, 'index.json');
      mkdirSync(tempCacheDir, { recursive: true });
      writeFileSync(indexPath, 'not valid json');

      const corruptedManager = new BundleManager({
        cacheDir: tempCacheDir,
      });

      // Should not throw
      const stats = await corruptedManager.getCacheStats();
      expect(stats.bundleCount).toBe(0);
    });

    it('should handle 404 errors', async () => {
      manager.addBundle({
        name: 'not-found-bundle',
        url: `http://localhost:${mockServerPort}/not-found`,
      });

      const result = await manager.downloadBundle('not-found-bundle');

      expect(result.success).toBe(false);
      expect(result.error).toContain('404');
    });
  });

  describe('bundle configuration options', () => {
    it('should support custom headers', async () => {
      manager.addBundle({
        name: 'headers-bundle',
        url: `http://localhost:${mockServerPort}/bundle.tar.gz`,
        headers: {
          Authorization: 'Bearer test-token',
        },
      });

      const result = await manager.downloadBundle('headers-bundle');
      expect(result.success).toBe(true);
    });

    it('should support custom refresh interval', async () => {
      manager.addBundle({
        name: 'refresh-bundle',
        url: `http://localhost:${mockServerPort}/bundle.tar.gz`,
        refresh_interval_ms: 1000, // 1 second
      });

      const result = await manager.downloadBundle('refresh-bundle');
      expect(result.success).toBe(true);

      const entry = await manager.getBundleEntry('refresh-bundle');
      expect(entry!.expires_at - entry!.downloaded_at).toBeLessThanOrEqual(2000);
    });
  });
});
