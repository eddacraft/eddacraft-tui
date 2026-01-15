/**
 * Tests for NullCacheProvider
 */

import { describe, it, expect, beforeEach } from 'vitest';
import { NullCacheProvider } from './null-cache.js';

describe('NullCacheProvider', () => {
  let cache: NullCacheProvider;

  beforeEach(() => {
    cache = new NullCacheProvider();
  });

  describe('get', () => {
    it('always returns null', async () => {
      const entry = await cache.get('any-key');
      expect(entry).toBeNull();
    });

    it('tracks misses', async () => {
      await cache.get('key1');
      await cache.get('key2');

      const stats = await cache.getStats();
      expect(stats.misses).toBe(2);
    });
  });

  describe('set', () => {
    it('is a no-op', async () => {
      await cache.set('key', 'value', { input_hash: 'hash' });

      const entry = await cache.get('key');
      expect(entry).toBeNull();
    });
  });

  describe('invalidate', () => {
    it('always returns false', async () => {
      const result = await cache.invalidate('any-key');
      expect(result).toBe(false);
    });
  });

  describe('invalidatePattern', () => {
    it('always returns 0', async () => {
      const count = await cache.invalidatePattern('*');
      expect(count).toBe(0);
    });
  });

  describe('getStats', () => {
    it('returns zero stats except misses', async () => {
      await cache.get('miss1');
      await cache.get('miss2');

      const stats = await cache.getStats();

      expect(stats.hits).toBe(0);
      expect(stats.misses).toBe(2);
      expect(stats.entries).toBe(0);
      expect(stats.size_bytes).toBe(0);
      expect(stats.hit_rate).toBe(0);
    });
  });

  describe('clear', () => {
    it('resets miss count', async () => {
      await cache.get('miss');
      await cache.clear();

      const stats = await cache.getStats();
      expect(stats.misses).toBe(0);
    });
  });

  describe('isAvailable', () => {
    it('always returns true', async () => {
      expect(await cache.isAvailable()).toBe(true);
    });
  });

  describe('name', () => {
    it('returns "null"', () => {
      expect(cache.name).toBe('null');
    });
  });
});
