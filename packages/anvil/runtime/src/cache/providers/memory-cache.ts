/**
 * In-memory cache provider
 * Used for watch mode and short-lived sessions
 */

import type { CacheProvider, CacheEntry, CacheSetOptions, CacheStats } from '../types.js';
import { createDebugger } from '@eddacraft/anvil-core';

const debug = createDebugger('cache');

/**
 * Memory cache configuration
 */
export interface MemoryCacheConfig {
  /** Maximum number of entries (default: 1000) */
  maxEntries?: number;
  /** Default TTL in milliseconds (default: 5 minutes for watch mode) */
  defaultTtl?: number;
}

/**
 * In-memory cache provider
 *
 * Features:
 * - Fast access for watch mode
 * - LRU eviction when max entries exceeded
 * - Automatic expiration cleanup
 */
export class MemoryCacheProvider implements CacheProvider {
  readonly name = 'memory';

  private readonly cache = new Map<string, CacheEntry<unknown>>();
  private readonly accessOrder: string[] = [];
  private readonly maxEntries: number;
  private readonly defaultTtl: number;

  private stats = {
    hits: 0,
    misses: 0,
  };

  constructor(config: MemoryCacheConfig = {}) {
    this.maxEntries = config.maxEntries ?? 1000;
    this.defaultTtl = config.defaultTtl ?? 5 * 60 * 1000; // 5 minutes
  }

  async get<T>(key: string): Promise<CacheEntry<T> | null> {
    debug('memory-cache get: key=%s size=%d', key, this.cache.size);
    const entry = this.cache.get(key) as CacheEntry<T> | undefined;

    if (!entry) {
      debug('memory-cache miss: key=%s (not found)', key);
      this.stats.misses++;
      return null;
    }

    // Check expiration
    if (entry.expires_at && Date.now() > entry.expires_at) {
      debug('memory-cache miss: key=%s (expired)', key);
      await this.invalidate(key);
      this.stats.misses++;
      return null;
    }

    // Update access order (move to end)
    this.updateAccessOrder(key);
    debug('memory-cache hit: key=%s', key);
    this.stats.hits++;

    return entry;
  }

  async set<T>(key: string, value: T, options: CacheSetOptions): Promise<void> {
    debug('memory-cache set: key=%s size=%d/%d', key, this.cache.size, this.maxEntries);
    const now = Date.now();
    const expiresAt = options.ttl ? now + options.ttl : now + this.defaultTtl;

    const entry: CacheEntry<T> = {
      value,
      created_at: now,
      expires_at: expiresAt,
      key,
      input_hash: options.input_hash,
    };

    // Check if we need to evict
    if (!this.cache.has(key) && this.cache.size >= this.maxEntries) {
      debug('memory-cache set: evicting LRU (at capacity %d)', this.maxEntries);
      this.evictLRU();
    }

    this.cache.set(key, entry);
    this.updateAccessOrder(key);
  }

  async invalidate(key: string): Promise<boolean> {
    debug('memory-cache invalidate: key=%s', key);
    const existed = this.cache.has(key);
    this.cache.delete(key);
    this.removeFromAccessOrder(key);
    return existed;
  }

  async invalidatePattern(pattern: string): Promise<number> {
    debug('memory-cache invalidatePattern: pattern=%s', pattern);
    const regex = this.patternToRegex(pattern);
    const keysToDelete: string[] = [];

    for (const key of this.cache.keys()) {
      if (regex.test(key)) {
        keysToDelete.push(key);
      }
    }

    debug('memory-cache invalidatePattern: matched %d entries', keysToDelete.length);
    for (const key of keysToDelete) {
      await this.invalidate(key);
    }

    return keysToDelete.length;
  }

  async getStats(): Promise<CacheStats> {
    const entries = this.cache.size;
    let sizeBytes = 0;

    // Estimate size (rough approximation)
    for (const entry of this.cache.values()) {
      sizeBytes += this.estimateSize(entry);
    }

    const totalRequests = this.stats.hits + this.stats.misses;
    const hitRate = totalRequests > 0 ? (this.stats.hits / totalRequests) * 100 : 0;

    return {
      hits: this.stats.hits,
      misses: this.stats.misses,
      entries,
      size_bytes: sizeBytes,
      hit_rate: Math.round(hitRate * 100) / 100,
    };
  }

  async clear(): Promise<void> {
    this.cache.clear();
    this.accessOrder.length = 0;
    this.stats = { hits: 0, misses: 0 };
  }

  async isAvailable(): Promise<boolean> {
    return true; // Memory cache is always available
  }

  /**
   * Clean up expired entries
   */
  async cleanup(): Promise<number> {
    debug('memory-cache cleanup: scanning %d entries', this.cache.size);
    const now = Date.now();
    const expiredKeys: string[] = [];

    for (const [key, entry] of this.cache.entries()) {
      if (entry.expires_at && entry.expires_at < now) {
        expiredKeys.push(key);
      }
    }

    debug('memory-cache cleanup: removing %d expired entries', expiredKeys.length);
    for (const key of expiredKeys) {
      await this.invalidate(key);
    }

    return expiredKeys.length;
  }

  /**
   * Get current entry count
   */
  get size(): number {
    return this.cache.size;
  }

  private updateAccessOrder(key: string): void {
    this.removeFromAccessOrder(key);
    this.accessOrder.push(key);
  }

  private removeFromAccessOrder(key: string): void {
    const index = this.accessOrder.indexOf(key);
    if (index !== -1) {
      this.accessOrder.splice(index, 1);
    }
  }

  private evictLRU(): void {
    // Clean up expired entries first
    const now = Date.now();
    for (const [key, entry] of this.cache.entries()) {
      if (entry.expires_at && entry.expires_at < now) {
        this.cache.delete(key);
        this.removeFromAccessOrder(key);
        if (this.cache.size < this.maxEntries) {
          return;
        }
      }
    }

    // If still over limit, evict least recently used
    while (this.cache.size >= this.maxEntries && this.accessOrder.length > 0) {
      const lruKey = this.accessOrder.shift();
      if (lruKey) {
        this.cache.delete(lruKey);
      }
    }
  }

  private patternToRegex(pattern: string): RegExp {
    const MAX_PATTERN_LENGTH = 200;
    const MAX_WILDCARDS = 10;

    if (pattern.length > MAX_PATTERN_LENGTH) {
      throw new Error(`Cache pattern too long: ${pattern.length} > ${MAX_PATTERN_LENGTH}`);
    }

    const wildcardCount = (pattern.match(/\*/g) || []).length;
    if (wildcardCount > MAX_WILDCARDS) {
      throw new Error(`Too many wildcards in pattern: ${wildcardCount} > ${MAX_WILDCARDS}`);
    }

    const DOUBLE_STAR_PLACEHOLDER = '\x00DOUBLESTAR\x00';
    const escaped = pattern
      .replace(/\*\*/g, DOUBLE_STAR_PLACEHOLDER)
      .replace(/[.+?^${}()|[\]\\]/g, '\\$&')
      .replace(/\*/g, '[^:]*')
      .replace(new RegExp(DOUBLE_STAR_PLACEHOLDER, 'g'), '[^:]*(?::[^:]*)*');
    return new RegExp(`^${escaped}$`);
  }

  private estimateSize(entry: CacheEntry<unknown>): number {
    // Rough estimation of object size in memory
    try {
      return JSON.stringify(entry).length * 2; // UTF-16 chars
    } catch {
      return 1024; // Default estimate for non-serialisable objects
    }
  }
}
