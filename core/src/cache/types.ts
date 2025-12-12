/**
 * Cache types and interfaces for Anvil gate caching
 */

/**
 * Cache entry with metadata
 */
export interface CacheEntry<T> {
  /** The cached value */
  value: T;
  /** Timestamp when the entry was created */
  created_at: number;
  /** Timestamp when the entry expires (optional) */
  expires_at?: number;
  /** Cache key used to store this entry */
  key: string;
  /** Hash of the inputs that generated this cache entry */
  input_hash: string;
}

/**
 * Options for setting cache entries
 */
export interface CacheSetOptions {
  /** Time-to-live in milliseconds (optional) */
  ttl?: number;
  /** Hash of the inputs that generated this value */
  input_hash: string;
}

/**
 * Cache statistics
 */
export interface CacheStats {
  /** Total number of cache hits */
  hits: number;
  /** Total number of cache misses */
  misses: number;
  /** Total number of entries in cache */
  entries: number;
  /** Total size of cache in bytes (approximate) */
  size_bytes: number;
  /** Hit rate as a percentage (0-100) */
  hit_rate: number;
}

/**
 * Cache provider interface
 * All cache implementations must implement this interface
 */
export interface CacheProvider {
  /** Provider name for debugging */
  readonly name: string;

  /**
   * Get a cached value by key
   * @returns The cached entry or null if not found/expired
   */
  get<T>(key: string): Promise<CacheEntry<T> | null>;

  /**
   * Set a cached value
   * @param key The cache key
   * @param value The value to cache
   * @param options Cache options including TTL and input hash
   */
  set<T>(key: string, value: T, options: CacheSetOptions): Promise<void>;

  /**
   * Invalidate a single cache entry
   * @returns true if entry was found and removed
   */
  invalidate(key: string): Promise<boolean>;

  /**
   * Invalidate cache entries matching a pattern
   * @param pattern Glob-like pattern (e.g., "gate:*", "check:eslint:*")
   * @returns Number of entries invalidated
   */
  invalidatePattern(pattern: string): Promise<number>;

  /**
   * Get cache statistics
   */
  getStats(): Promise<CacheStats>;

  /**
   * Clear all cache entries
   */
  clear(): Promise<void>;

  /**
   * Check if provider is available (e.g., directory exists for file cache)
   */
  isAvailable(): Promise<boolean>;
}

/**
 * Cache key components for gate check results
 */
export interface GateCacheKeyInput {
  /** Check name (e.g., "eslint", "coverage") */
  check_name: string;
  /** Plan hash */
  plan_hash: string;
  /** Check configuration hash */
  config_hash: string;
  /** Workspace root (normalised) */
  workspace_root: string;
  /** Optional extra discriminators */
  extra?: Record<string, string>;
}

/**
 * Result of a cached gate check lookup
 */
export interface CachedGateResult {
  /** Whether the result was found in cache */
  cached: boolean;
  /** The gate result if cached */
  result?: import('../types/gate.types.js').GateResult;
  /** Cache entry metadata */
  entry?: CacheEntry<import('../types/gate.types.js').GateResult>;
  /** Time saved by cache hit (in ms) */
  time_saved_ms?: number;
}
