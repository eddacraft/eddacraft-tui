/**
 * Tests for FileCacheProvider
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdirSync, rmSync } from 'fs';
import { join } from 'path';
import { tmpdir } from 'os';
import { randomUUID } from 'crypto';
import { FileCacheProvider } from './file-cache.js';

describe('FileCacheProvider', () => {
  let testDir: string;
  let cache: FileCacheProvider;

  beforeEach(() => {
    testDir = join(tmpdir(), `anvil-cache-test-${randomUUID()}`);
    mkdirSync(testDir, { recursive: true });
    cache = new FileCacheProvider(testDir, {
      defaultTtl: 60000, // 1 minute
    });
  });

  afterEach(() => {
    try {
      rmSync(testDir, { recursive: true, force: true });
    } catch {
      // Ignore cleanup errors
    }
  });

  describe('get/set', () => {
    it('stores and retrieves values', async () => {
      await cache.set('test-key', { data: 'value' }, { input_hash: 'hash123' });

      const entry = await cache.get<{ data: string }>('test-key');

      expect(entry).not.toBeNull();
      expect(entry?.value).toEqual({ data: 'value' });
      expect(entry?.input_hash).toBe('hash123');
      expect(entry?.key).toBe('test-key');
    });

    it('returns null for non-existent keys', async () => {
      const entry = await cache.get('non-existent');
      expect(entry).toBeNull();
    });

    it('persists to disk', async () => {
      await cache.set('persistent', 'value', { input_hash: 'hash' });

      // Create new cache instance pointing to same directory
      const cache2 = new FileCacheProvider(testDir);
      const entry = await cache2.get('persistent');

      expect(entry?.value).toBe('value');
    });

    it('tracks cache stats', async () => {
      await cache.set('key1', 'value1', { input_hash: 'hash1' });

      // Miss
      await cache.get('non-existent');

      // Hit
      await cache.get('key1');

      const stats = await cache.getStats();
      expect(stats.hits).toBe(1);
      expect(stats.misses).toBe(1);
    });
  });

  describe('expiration', () => {
    it('returns null for expired entries', async () => {
      const shortTtlCache = new FileCacheProvider(testDir, { defaultTtl: 10 }); // 10ms

      await shortTtlCache.set('expires-soon', 'value', { input_hash: 'hash' });

      // Wait for expiration
      await new Promise((resolve) => setTimeout(resolve, 20));

      const entry = await shortTtlCache.get('expires-soon');
      expect(entry).toBeNull();
    });

    it('respects custom TTL', async () => {
      await cache.set('custom-ttl', 'value', { input_hash: 'hash', ttl: 10 });

      // Should exist immediately
      let entry = await cache.get('custom-ttl');
      expect(entry).not.toBeNull();

      // Wait for expiration
      await new Promise((resolve) => setTimeout(resolve, 20));

      entry = await cache.get('custom-ttl');
      expect(entry).toBeNull();
    });
  });

  describe('invalidate', () => {
    it('removes single entries', async () => {
      await cache.set('key1', 'value1', { input_hash: 'hash1' });
      await cache.set('key2', 'value2', { input_hash: 'hash2' });

      const removed = await cache.invalidate('key1');

      expect(removed).toBe(true);
      expect(await cache.get('key1')).toBeNull();
      expect(await cache.get('key2')).not.toBeNull();
    });

    it('returns false for non-existent keys', async () => {
      const removed = await cache.invalidate('non-existent');
      expect(removed).toBe(false);
    });
  });

  describe('invalidatePattern', () => {
    it('removes entries matching pattern', async () => {
      await cache.set('gate:check:eslint:abc', 'v1', { input_hash: 'h1' });
      await cache.set('gate:check:eslint:def', 'v2', { input_hash: 'h2' });
      await cache.set('gate:check:coverage:xyz', 'v3', { input_hash: 'h3' });

      const count = await cache.invalidatePattern('gate:check:eslint:*');

      expect(count).toBe(2);
      expect(await cache.get('gate:check:eslint:abc')).toBeNull();
      expect(await cache.get('gate:check:eslint:def')).toBeNull();
      expect(await cache.get('gate:check:coverage:xyz')).not.toBeNull();
    });
  });

  describe('clear', () => {
    it('removes all entries', async () => {
      await cache.set('key1', 'value1', { input_hash: 'hash1' });
      await cache.set('key2', 'value2', { input_hash: 'hash2' });

      await cache.clear();

      expect(await cache.get('key1')).toBeNull();
      expect(await cache.get('key2')).toBeNull();
    });
  });

  describe('cleanup', () => {
    it('removes expired entries', async () => {
      const shortTtlCache = new FileCacheProvider(testDir, { defaultTtl: 10 });

      await shortTtlCache.set('expires', 'value', { input_hash: 'hash' });
      await shortTtlCache.set('stays', 'value', { input_hash: 'hash', ttl: 60000 });

      // Wait for first to expire
      await new Promise((resolve) => setTimeout(resolve, 20));

      const count = await shortTtlCache.cleanup();

      expect(count).toBe(1);
      expect(await shortTtlCache.get('stays')).not.toBeNull();
    });
  });

  describe('isAvailable', () => {
    it('returns true for valid directory', async () => {
      expect(await cache.isAvailable()).toBe(true);
    });
  });

  describe('getStats', () => {
    it('returns accurate statistics', async () => {
      await cache.set('key1', 'value1', { input_hash: 'h1' });
      await cache.set('key2', 'value2', { input_hash: 'h2' });

      await cache.get('key1'); // Hit
      await cache.get('key1'); // Hit
      await cache.get('missing'); // Miss

      const stats = await cache.getStats();

      expect(stats.entries).toBe(2);
      expect(stats.hits).toBe(2);
      expect(stats.misses).toBe(1);
      expect(stats.hit_rate).toBeCloseTo(66.67, 1);
      expect(stats.size_bytes).toBeGreaterThan(0);
    });
  });

  describe('name', () => {
    it('returns "file"', () => {
      expect(cache.name).toBe('file');
    });
  });
});
