/**
 * Cache interface definitions
 *
 * Defines the contract for cache providers.
 */

/**
 * Cache entry with metadata
 */
export interface CacheEntry<T = unknown> {
  /** The cached value */
  value: T;
  /** When the entry was created */
  createdAt: Date;
  /** When the entry expires (optional) */
  expiresAt?: Date;
  /** Cache entry tags for invalidation */
  tags?: string[];
}

/**
 * Cache provider interface
 */
export interface ICacheProvider {
  /** Get a value from the cache */
  get<T>(key: string): Promise<T | undefined>;

  /** Set a value in the cache */
  set<T>(key: string, value: T, ttlMs?: number): Promise<void>;

  /** Check if a key exists */
  has(key: string): Promise<boolean>;

  /** Delete a key from the cache */
  delete(key: string): Promise<boolean>;

  /** Clear all entries from the cache */
  clear(): Promise<void>;

  /** Get cache statistics */
  stats?(): Promise<CacheStats>;
}

/**
 * Cache statistics
 */
export interface CacheStats {
  /** Number of entries in the cache */
  size: number;
  /** Number of cache hits */
  hits: number;
  /** Number of cache misses */
  misses: number;
  /** Hit rate as a percentage */
  hitRate: number;
}
