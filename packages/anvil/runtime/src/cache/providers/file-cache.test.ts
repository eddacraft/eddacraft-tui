/**
 * Tests for FileCacheProvider
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { mkdtempSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { FileCacheProvider } from './file-cache.js';
import { safeCleanup } from '../../../../../../tools/test-utils/safe-cleanup.js';

describe('FileCacheProvider', () => {
  let testDir: string;
  let cache: FileCacheProvider;

  beforeEach(() => {
    testDir = mkdtempSync(join(tmpdir(), 'anvil-cache-test-'));
    cache = new FileCacheProvider(testDir, {
      defaultTtl: 60000, // 1 minute
    });
  });

  afterEach(async () => {
    await safeCleanup(testDir);
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
      const shortTtlCache = new FileCacheProvider(testDir, { defaultTtl: 5000 });

      await shortTtlCache.set('expires-soon', 'value', { input_hash: 'hash' });

      // Advance past TTL using fake timers so Date.now() moves without real I/O races
      vi.useFakeTimers({ now: Date.now() });
      try {
        vi.advanceTimersByTime(6000);
        const entry = await shortTtlCache.get('expires-soon');
        expect(entry).toBeNull();
      } finally {
        vi.useRealTimers();
      }
    });

    it('respects custom TTL', async () => {
      await cache.set('custom-ttl', 'value', { input_hash: 'hash', ttl: 5000 });

      // Should exist immediately — no fake timers needed, real time hasn't advanced 5s
      let entry = await cache.get('custom-ttl');
      expect(entry).not.toBeNull();

      // Advance past TTL using fake timers
      vi.useFakeTimers({ now: Date.now() });
      try {
        vi.advanceTimersByTime(6000);
        entry = await cache.get('custom-ttl');
        expect(entry).toBeNull();
      } finally {
        vi.useRealTimers();
      }
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

  describe('HMAC integrity', () => {
    it('rejects cache entries with tampered content', async () => {
      await cache.set('tampered', { secret: 'data' }, { input_hash: 'h1' });

      // Tamper with the entry file on disk
      const { readdirSync, readFileSync, writeFileSync } = await import('node:fs');
      const entriesDir = join(testDir, '.anvil', 'cache', 'entries');
      const files = readdirSync(entriesDir);
      expect(files.length).toBeGreaterThan(0);

      const filePath = join(entriesDir, files[0]);
      const raw = readFileSync(filePath, 'utf-8');
      const newlineIdx = raw.indexOf('\n');
      const hmac = raw.slice(0, newlineIdx);
      // Replace content but keep original HMAC
      writeFileSync(filePath, `${hmac}\n{"value":"injected","created_at":0,"key":"tampered"}`);

      const entry = await cache.get('tampered');
      expect(entry).toBeNull();
    });

    it('rejects cache entries without HMAC', async () => {
      await cache.set('no-hmac', 'value', { input_hash: 'h1' });

      // Overwrite the file with content only (no HMAC line)
      const { readdirSync, writeFileSync } = await import('node:fs');
      const entriesDir = join(testDir, '.anvil', 'cache', 'entries');
      const files = readdirSync(entriesDir);
      const filePath = join(entriesDir, files[0]);
      writeFileSync(filePath, '{"value":"bare","created_at":0,"key":"no-hmac"}');

      const entry = await cache.get('no-hmac');
      expect(entry).toBeNull();
    });

    it('rejects cache entries with invalid hex in HMAC', async () => {
      await cache.set('bad-hex', 'value', { input_hash: 'h1' });

      const { readdirSync, readFileSync, writeFileSync } = await import('node:fs');
      const entriesDir = join(testDir, '.anvil', 'cache', 'entries');
      const files = readdirSync(entriesDir);
      const filePath = join(entriesDir, files[0]);
      const raw = readFileSync(filePath, 'utf-8');
      const content = raw.slice(raw.indexOf('\n') + 1);
      // Replace HMAC with non-hex characters of correct length
      const badHmac = 'zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz';
      writeFileSync(filePath, `${badHmac}\n${content}`);

      const entry = await cache.get('bad-hex');
      expect(entry).toBeNull();
    });

    it('accepts valid untampered entries', async () => {
      await cache.set('valid', { data: 'ok' }, { input_hash: 'h1' });

      const entry = await cache.get<{ data: string }>('valid');
      expect(entry).not.toBeNull();
      expect(entry?.value).toEqual({ data: 'ok' });
    });
  });

  describe('name', () => {
    it('returns "file"', () => {
      expect(cache.name).toBe('file');
    });
  });
});
