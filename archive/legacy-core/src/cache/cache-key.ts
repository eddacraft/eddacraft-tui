/**
 * Cache key generation utilities
 */

import { createHash } from 'crypto';
import type { GateCacheKeyInput } from './types.js';
import type { GateCheck } from '../types/gate.types.js';

/**
 * Generate a deterministic cache key for gate check results
 *
 * Key format: gate:check:{check_name}:{combined_hash}
 * Where combined_hash = SHA256(plan_hash + config_hash + workspace_root + extra)
 */
export function generateCacheKey(input: GateCacheKeyInput): string {
  const hash = createHash('sha256');

  // Add all inputs in deterministic order
  hash.update(input.plan_hash);
  hash.update(input.config_hash);
  hash.update(normaliseWorkspacePath(input.workspace_root));

  // Add extra discriminators in sorted order
  if (input.extra) {
    const sortedKeys = Object.keys(input.extra).sort();
    for (const key of sortedKeys) {
      hash.update(`${key}:${input.extra[key]}`);
    }
  }

  const combinedHash = hash.digest('hex').slice(0, 16);
  return `gate:check:${input.check_name}:${combinedHash}`;
}

/**
 * Generate a hash for check configuration
 * Used to invalidate cache when config changes
 */
export function hashCheckConfig(config: Record<string, unknown>): string {
  const hash = createHash('sha256');
  // Canonicalise JSON for deterministic hashing
  hash.update(JSON.stringify(sortObjectKeys(config)));
  return hash.digest('hex').slice(0, 16);
}

/**
 * Generate a hash for the full gate configuration
 * Used to invalidate all caches when global config changes
 */
export function hashGateConfig(checks: GateCheck[]): string {
  const hash = createHash('sha256');

  // Only hash enabled checks and their configs
  const enabledChecks = checks
    .filter((c) => c.enabled)
    .map((c) => ({
      name: c.name,
      config: c.config || {},
    }))
    .sort((a, b) => a.name.localeCompare(b.name));

  hash.update(JSON.stringify(enabledChecks));
  return hash.digest('hex').slice(0, 16);
}

/**
 * Generate an input hash for cache validation
 * This hash captures all inputs that affect the cache result
 */
export function generateInputHash(input: GateCacheKeyInput): string {
  const hash = createHash('sha256');
  hash.update(JSON.stringify(sortObjectKeys(input)));
  return hash.digest('hex');
}

/**
 * Parse a cache key into its components
 */
export function parseCacheKey(key: string): {
  type: string;
  subtype: string;
  name: string;
  hash: string;
} | null {
  const parts = key.split(':');
  if (parts.length !== 4) {
    return null;
  }
  return {
    type: parts[0],
    subtype: parts[1],
    name: parts[2],
    hash: parts[3],
  };
}

/**
 * Generate a pattern for invalidating all cache entries for a check
 */
export function checkInvalidationPattern(checkName: string): string {
  return `gate:check:${checkName}:*`;
}

/**
 * Generate a pattern for invalidating all gate cache entries
 */
export function allGateInvalidationPattern(): string {
  return 'gate:*';
}

/**
 * Normalise workspace path for consistent cache keys
 * - Removes trailing slashes
 * - Converts backslashes to forward slashes (Windows)
 * - Lowercases drive letters (Windows)
 */
function normaliseWorkspacePath(path: string): string {
  let normalised = path
    // Convert backslashes to forward slashes
    .replace(/\\/g, '/')
    // Remove trailing slashes
    .replace(/\/+$/, '');

  // Lowercase Windows drive letters (C: -> c:)
  if (/^[A-Z]:/.test(normalised)) {
    normalised = normalised[0].toLowerCase() + normalised.slice(1);
  }

  return normalised;
}

/**
 * Recursively sort object keys for deterministic JSON stringification
 */
function sortObjectKeys(obj: unknown): unknown {
  if (obj === null || typeof obj !== 'object') {
    return obj;
  }

  if (Array.isArray(obj)) {
    return obj.map(sortObjectKeys);
  }

  const sorted: Record<string, unknown> = {};
  const keys = Object.keys(obj as Record<string, unknown>).sort();
  for (const key of keys) {
    sorted[key] = sortObjectKeys((obj as Record<string, unknown>)[key]);
  }
  return sorted;
}
