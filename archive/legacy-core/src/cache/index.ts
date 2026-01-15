/**
 * Cache module exports
 */

// Types
export type {
  CacheProvider,
  CacheEntry,
  CacheSetOptions,
  CacheStats,
  GateCacheKeyInput,
  CachedGateResult,
} from './types.js';

// Providers
export { FileCacheProvider, type FileCacheConfig } from './providers/file-cache.js';
export { MemoryCacheProvider, type MemoryCacheConfig } from './providers/memory-cache.js';
export { NullCacheProvider } from './providers/null-cache.js';

// Cache key utilities
export {
  generateCacheKey,
  hashCheckConfig,
  hashGateConfig,
  generateInputHash,
  parseCacheKey,
  checkInvalidationPattern,
  allGateInvalidationPattern,
} from './cache-key.js';

// Factory function for creating cache providers
import type { CacheProvider } from './types.js';
import { FileCacheProvider } from './providers/file-cache.js';
import { MemoryCacheProvider } from './providers/memory-cache.js';
import { NullCacheProvider } from './providers/null-cache.js';

export type CacheProviderType = 'file' | 'memory' | 'null';

export interface CreateCacheOptions {
  /** Provider type */
  type: CacheProviderType;
  /** Workspace root (required for file cache) */
  workspaceRoot?: string;
  /** Whether to disable caching */
  disabled?: boolean;
  /** Use global cache in home directory */
  useGlobalCache?: boolean;
  /** Default TTL in milliseconds */
  defaultTtl?: number;
}

/**
 * Create a cache provider based on options
 */
export function createCacheProvider(options: CreateCacheOptions): CacheProvider {
  if (options.disabled) {
    return new NullCacheProvider();
  }

  switch (options.type) {
    case 'file':
      if (!options.workspaceRoot) {
        throw new Error('workspaceRoot is required for file cache provider');
      }
      return new FileCacheProvider(options.workspaceRoot, {
        useGlobalCache: options.useGlobalCache,
        defaultTtl: options.defaultTtl,
      });

    case 'memory':
      return new MemoryCacheProvider({
        defaultTtl: options.defaultTtl,
      });

    case 'null':
      return new NullCacheProvider();

    default:
      throw new Error(`Unknown cache provider type: ${options.type}`);
  }
}
