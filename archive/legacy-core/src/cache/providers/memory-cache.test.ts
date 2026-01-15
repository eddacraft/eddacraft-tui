/**
 * Tests for MemoryCacheProvider
 */

import { describe, it, expect, beforeEach } from 'vitest';
import { MemoryCacheProvider } from './memory-cache.js';

describe('MemoryCacheProvider', () => {
  let cache: MemoryCacheProvider;

  beforeEach(() => {
    cache = new MemoryCacheProvider({
      maxEntries: 100,
      defaultTtl: 60000, // 1 minute
    });
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
      const shortTtlCache = new MemoryCacheProvider({ defaultTtl: 10 }); // 10ms TTL

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

    it('supports ** wildcard for cross-segment matching', async () => {
      await cache.set('gate:check:eslint:abc', 'v1', { input_hash: 'h1' });
      await cache.set('gate:check:coverage:xyz', 'v2', { input_hash: 'h2' });
      await cache.set('other:key', 'v3', { input_hash: 'h3' });

      // ** should match any characters including colons
      const count = await cache.invalidatePattern('gate:**');

      expect(count).toBe(2);
      expect(await cache.get('other:key')).not.toBeNull();
    });
  });

  describe('clear', () => {
    it('removes all entries', async () => {
      await cache.set('key1', 'value1', { input_hash: 'hash1' });
      await cache.set('key2', 'value2', { input_hash: 'hash2' });

      await cache.clear();

      expect(await cache.get('key1')).toBeNull();
      expect(await cache.get('key2')).toBeNull();

      const stats = await cache.getStats();
      expect(stats.entries).toBe(0);
    });
  });

  describe('LRU eviction', () => {
    it('evicts least recently used entries when max exceeded', async () => {
      const smallCache = new MemoryCacheProvider({ maxEntries: 3 });

      await smallCache.set('key1', 'value1', { input_hash: 'h1' });
      await smallCache.set('key2', 'value2', { input_hash: 'h2' });
      await smallCache.set('key3', 'value3', { input_hash: 'h3' });

      // Access key1 to make it recently used
      await smallCache.get('key1');

      // Add new entry, should evict key2 (oldest not accessed)
      await smallCache.set('key4', 'value4', { input_hash: 'h4' });

      expect(await smallCache.get('key1')).not.toBeNull();
      expect(await smallCache.get('key2')).toBeNull(); // Evicted
      expect(await smallCache.get('key3')).not.toBeNull();
      expect(await smallCache.get('key4')).not.toBeNull();
    });
  });

  describe('cleanup', () => {
    it('removes expired entries', async () => {
      const shortTtlCache = new MemoryCacheProvider({ defaultTtl: 10 });

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
    it('always returns true', async () => {
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
});
