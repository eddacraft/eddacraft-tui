/**
 * Null cache provider (no-op)
 * Used when caching is disabled via --no-cache flag
 */

import type { CacheProvider, CacheEntry, CacheSetOptions, CacheStats } from '../types.js';
import { createDebugger } from '@eddacraft/anvil-core';

const debug = createDebugger('cache');

/**
 * Null cache provider - disables caching
 *
 * All operations are no-ops that return appropriate null/empty values.
 * Used when:
 * - --no-cache flag is passed
 * - Caching is disabled in configuration
 * - Testing scenarios where caching should be bypassed
 */
export class NullCacheProvider implements CacheProvider {
  readonly name = 'null';

  private stats = {
    misses: 0,
  };

  async get<T>(_key: string): Promise<CacheEntry<T> | null> {
    debug(`null-cache get: key=${_key} (always miss)`);
    this.stats.misses++;
    return null;
  }

  async set<T>(_key: string, _value: T, _options: CacheSetOptions): Promise<void> {
    debug(`null-cache set: key=${_key} (no-op)`);
  }

  async invalidate(_key: string): Promise<boolean> {
    return false;
  }

  async invalidatePattern(_pattern: string): Promise<number> {
    return 0;
  }

  async getStats(): Promise<CacheStats> {
    return {
      hits: 0,
      misses: this.stats.misses,
      entries: 0,
      size_bytes: 0,
      hit_rate: 0,
    };
  }

  async clear(): Promise<void> {
    this.stats.misses = 0;
  }

  async isAvailable(): Promise<boolean> {
    return true; // Null cache is always "available"
  }
}
