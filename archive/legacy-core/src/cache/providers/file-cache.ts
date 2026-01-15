/**
 * File-based cache provider
 * Stores cache entries as JSON files in .anvil/cache/
 */

import { existsSync, unlinkSync } from 'fs';
import { readFile, writeFile, rm, mkdir } from 'fs/promises';
import { join } from 'path';
import { createHash } from 'crypto';
import { homedir } from 'os';
import { z } from 'zod';
import type { CacheProvider, CacheEntry, CacheSetOptions, CacheStats } from '../types.js';
import { createDebugger } from '../../utils/debug.js';

const debug = createDebugger('cache');

const CacheEntrySchema = z.object({
  value: z.unknown(),
  created_at: z.number(),
  expires_at: z.number().optional(),
  key: z.string(),
  input_hash: z.string().optional(),
});

const CacheIndexEntrySchema = z.object({
  file: z.string(),
  created_at: z.number(),
  expires_at: z.number().optional(),
  size_bytes: z.number(),
});

const CacheIndexSchema = z.object({
  version: z.number(),
  entries: z.record(z.string(), CacheIndexEntrySchema),
  stats: z.object({
    hits: z.number(),
    misses: z.number(),
  }),
});

/**
 * Default cache directory
 */
const DEFAULT_CACHE_DIR = '.anvil/cache';

/**
 * Default TTL: 24 hours
 */
const DEFAULT_TTL_MS = 24 * 60 * 60 * 1000;

/**
 * Cache index structure
 */
interface CacheIndex {
  version: number;
  entries: Record<
    string,
    {
      file: string;
      created_at: number;
      expires_at?: number;
      size_bytes: number;
    }
  >;
  stats: {
    hits: number;
    misses: number;
  };
}

/**
 * File-based cache provider configuration
 */
export interface FileCacheConfig {
  /** Base directory for cache (default: .anvil/cache in workspace root) */
  cacheDir?: string;
  /** Default TTL in milliseconds (default: 24 hours) */
  defaultTtl?: number;
  /** Maximum cache size in bytes (default: 100MB) */
  maxSizeBytes?: number;
  /** Whether to use global cache in home directory */
  useGlobalCache?: boolean;
}

/**
 * File-based cache provider
 *
 * Storage structure:
 * .anvil/cache/
 * ├── index.json         # Cache registry and stats
 * └── entries/           # Cache entry files
 *     └── {key-hash}.json
 */
export class FileCacheProvider implements CacheProvider {
  readonly name = 'file';

  private readonly cacheDir: string;
  private readonly entriesDir: string;
  private readonly indexPath: string;
  private readonly defaultTtl: number;
  private readonly maxSizeBytes: number;

  private index: CacheIndex | null = null;
  private indexDirty = false;

  constructor(workspaceRoot: string, config: FileCacheConfig = {}) {
    if (config.useGlobalCache) {
      this.cacheDir = join(homedir(), '.anvil', 'cache');
    } else {
      this.cacheDir = config.cacheDir || join(workspaceRoot, DEFAULT_CACHE_DIR);
    }

    this.entriesDir = join(this.cacheDir, 'entries');
    this.indexPath = join(this.cacheDir, 'index.json');
    this.defaultTtl = config.defaultTtl ?? DEFAULT_TTL_MS;
    this.maxSizeBytes = config.maxSizeBytes ?? 100 * 1024 * 1024; // 100MB
  }

  async get<T>(key: string): Promise<CacheEntry<T> | null> {
    const index = await this.loadIndex();

    const entryMeta = index.entries[key];
    if (!entryMeta) {
      index.stats.misses++;
      this.indexDirty = true;
      await this.saveIndex();
      return null;
    }

    // Check expiration
    if (entryMeta.expires_at && Date.now() > entryMeta.expires_at) {
      await this.invalidate(key);
      index.stats.misses++;
      this.indexDirty = true;
      await this.saveIndex();
      return null;
    }

    // Read entry file
    const entryPath = join(this.entriesDir, entryMeta.file);
    try {
      const content = await readFile(entryPath, 'utf-8');
      const parseResult = CacheEntrySchema.safeParse(JSON.parse(content));
      if (!parseResult.success) {
        debug('Invalid cache entry schema, removing from index', parseResult.error);
        await this.invalidate(key);
        index.stats.misses++;
        this.indexDirty = true;
        await this.saveIndex();
        return null;
      }
      const entry = parseResult.data as CacheEntry<T>;

      index.stats.hits++;
      this.indexDirty = true;
      await this.saveIndex();

      return entry;
    } catch (error) {
      debug('Cache entry file missing or corrupted, removing from index', error);
      await this.invalidate(key);
      index.stats.misses++;
      this.indexDirty = true;
      await this.saveIndex();
      return null;
    }
  }

  async set<T>(key: string, value: T, options: CacheSetOptions): Promise<void> {
    await this.ensureCacheDir();
    const index = await this.loadIndex();

    const now = Date.now();
    const expiresAt = options.ttl ? now + options.ttl : now + this.defaultTtl;

    const entry: CacheEntry<T> = {
      value,
      created_at: now,
      expires_at: expiresAt,
      key,
      input_hash: options.input_hash,
    };

    // Generate filename from key hash
    const fileHash = this.hashKey(key);
    const fileName = `${fileHash}.json`;
    const filePath = join(this.entriesDir, fileName);

    // Write entry file
    const content = JSON.stringify(entry, null, 2);
    await writeFile(filePath, content, 'utf-8');

    // Update index
    index.entries[key] = {
      file: fileName,
      created_at: now,
      expires_at: expiresAt,
      size_bytes: Buffer.byteLength(content, 'utf-8'),
    };

    this.indexDirty = true;
    await this.saveIndex();

    // Check if cache size exceeds limit
    await this.maybeEvict();
  }

  async invalidate(key: string): Promise<boolean> {
    const index = await this.loadIndex();

    const entryMeta = index.entries[key];
    if (!entryMeta) {
      return false;
    }

    // Remove entry file
    const filePath = join(this.entriesDir, entryMeta.file);
    try {
      unlinkSync(filePath);
    } catch (error) {
      debug('Cache entry file already deleted or inaccessible', error);
    }

    // Remove from index
    delete index.entries[key];
    this.indexDirty = true;
    await this.saveIndex();

    return true;
  }

  async invalidatePattern(pattern: string): Promise<number> {
    const index = await this.loadIndex();
    const regex = this.patternToRegex(pattern);

    const keysToInvalidate = Object.keys(index.entries).filter((key) => regex.test(key));

    for (const key of keysToInvalidate) {
      await this.invalidate(key);
    }

    return keysToInvalidate.length;
  }

  async getStats(): Promise<CacheStats> {
    const index = await this.loadIndex();

    const entries = Object.keys(index.entries).length;
    const sizeBytes = Object.values(index.entries).reduce((sum, e) => sum + e.size_bytes, 0);
    const totalRequests = index.stats.hits + index.stats.misses;
    const hitRate = totalRequests > 0 ? (index.stats.hits / totalRequests) * 100 : 0;

    return {
      hits: index.stats.hits,
      misses: index.stats.misses,
      entries,
      size_bytes: sizeBytes,
      hit_rate: Math.round(hitRate * 100) / 100,
    };
  }

  async clear(): Promise<void> {
    try {
      await rm(this.cacheDir, { recursive: true, force: true });
      this.index = null;
      this.indexDirty = false;
    } catch (error) {
      debug('Failed to clear cache directory (may not exist)', error);
    }
  }

  async isAvailable(): Promise<boolean> {
    try {
      await this.ensureCacheDir();
      return true;
    } catch (error) {
      debug('Cache directory not available or not writable', error);
      return false;
    }
  }

  /**
   * Clean up expired entries
   */
  async cleanup(): Promise<number> {
    const index = await this.loadIndex();
    const now = Date.now();

    const expiredKeys = Object.entries(index.entries)
      .filter(([, meta]) => meta.expires_at && meta.expires_at < now)
      .map(([key]) => key);

    for (const key of expiredKeys) {
      await this.invalidate(key);
    }

    return expiredKeys.length;
  }

  private async ensureCacheDir(): Promise<void> {
    if (!existsSync(this.entriesDir)) {
      await mkdir(this.entriesDir, { recursive: true });
    }
  }

  private async loadIndex(): Promise<CacheIndex> {
    if (this.index) {
      return this.index;
    }

    try {
      const content = await readFile(this.indexPath, 'utf-8');
      const parseResult = CacheIndexSchema.safeParse(JSON.parse(content));
      if (parseResult.success) {
        this.index = parseResult.data;
      } else {
        debug('Invalid cache index schema, creating new one', parseResult.error);
        this.index = {
          version: 1,
          entries: {},
          stats: { hits: 0, misses: 0 },
        };
        this.indexDirty = true;
      }
    } catch (error) {
      debug('Cache index missing or corrupted, creating new one', error);
      this.index = {
        version: 1,
        entries: {},
        stats: { hits: 0, misses: 0 },
      };
      this.indexDirty = true;
    }

    return this.index as CacheIndex;
  }

  private async saveIndex(): Promise<void> {
    if (!this.indexDirty || !this.index) {
      return;
    }

    await this.ensureCacheDir();
    await writeFile(this.indexPath, JSON.stringify(this.index, null, 2), 'utf-8');
    this.indexDirty = false;
  }

  private hashKey(key: string): string {
    // Simple hash for filename safety
    return createHash('sha256').update(key).digest('hex').slice(0, 32);
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

  private async maybeEvict(): Promise<void> {
    const stats = await this.getStats();

    if (stats.size_bytes <= this.maxSizeBytes) {
      return;
    }

    const index = await this.loadIndex();

    // Sort entries by creation time (oldest first)
    const sortedEntries = Object.entries(index.entries).sort(
      ([, a], [, b]) => a.created_at - b.created_at
    );

    // Evict oldest entries until under limit
    let currentSize = stats.size_bytes;
    for (const [key] of sortedEntries) {
      if (currentSize <= this.maxSizeBytes * 0.8) {
        // Evict until 80% of limit
        break;
      }
      const meta = index.entries[key];
      currentSize -= meta.size_bytes;
      await this.invalidate(key);
    }
  }
}
