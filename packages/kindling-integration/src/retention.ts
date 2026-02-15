/**
 * Retention Management (KINDLING-016)
 *
 * Handles pruning of old observations and storage statistics.
 * Works against the abstract IKindlingStore interface.
 *
 * Since the abstract store does not expose a direct "delete older than" method,
 * retention is implemented via a dedicated IRetentionCapableStore interface
 * that concrete stores can optionally implement.
 */

import type { KindlingConfig } from './config.js';

// =============================================================================
// Retention Store Interface
// =============================================================================

/**
 * Extended store interface for stores that support retention operations.
 *
 * Not all stores need to implement this. The NoOpKindlingStore does not.
 * Concrete SQLite or database-backed stores should implement this interface.
 */
export interface IRetentionCapableStore {
  /**
   * Delete all observations with timestamps older than the given ISO8601 date.
   *
   * @param olderThan - ISO8601 datetime cutoff
   * @returns Number of observations deleted
   */
  deleteObservationsOlderThan(olderThan: string): Promise<number>;

  /**
   * Get storage statistics.
   *
   * @returns Observation count and estimated storage size in bytes
   */
  getStats(): Promise<StorageStats>;
}

/**
 * Storage statistics
 */
export interface StorageStats {
  /** Total number of observations in the store */
  observation_count: number;
  /** Estimated storage size in bytes */
  estimated_size_bytes: number;
}

// =============================================================================
// Type Guard
// =============================================================================

/**
 * Check if a store supports retention operations.
 */
export function isRetentionCapable(store: unknown): store is IRetentionCapableStore {
  if (store === null || typeof store !== 'object') {
    return false;
  }
  const candidate = store as Record<string, unknown>;
  return (
    typeof candidate['deleteObservationsOlderThan'] === 'function' &&
    typeof candidate['getStats'] === 'function'
  );
}

// =============================================================================
// Pruning
// =============================================================================

/**
 * Result of a pruning operation
 */
export interface PruneResult {
  /** Whether pruning was actually performed */
  pruned: boolean;
  /** Number of observations deleted (0 if not pruned) */
  deleted_count: number;
  /** The cutoff date used */
  cutoff_date: string;
  /** Reason if pruning was skipped */
  skip_reason?: string;
}

/**
 * Prune observations older than the configured retention period.
 *
 * If the store does not implement IRetentionCapableStore, this is a no-op
 * that returns a skip result.
 *
 * @param store - The store to prune (must implement IRetentionCapableStore)
 * @param config - Kindling configuration with retention settings
 * @returns Prune result
 */
export async function pruneOldObservations(
  store: unknown,
  config: KindlingConfig
): Promise<PruneResult> {
  if (!config.enabled) {
    return {
      pruned: false,
      deleted_count: 0,
      cutoff_date: '',
      skip_reason: 'Kindling is disabled',
    };
  }

  if (!isRetentionCapable(store)) {
    return {
      pruned: false,
      deleted_count: 0,
      cutoff_date: '',
      skip_reason: 'Store does not support retention operations',
    };
  }

  const cutoffDate = new Date();
  cutoffDate.setDate(cutoffDate.getDate() - config.retention.days);
  const cutoffIso = cutoffDate.toISOString();

  const deletedCount = await store.deleteObservationsOlderThan(cutoffIso);

  return {
    pruned: true,
    deleted_count: deletedCount,
    cutoff_date: cutoffIso,
  };
}

// =============================================================================
// Statistics
// =============================================================================

/**
 * Get storage statistics from the store.
 *
 * If the store does not implement IRetentionCapableStore, returns zero values.
 *
 * @param store - The store to query
 * @returns Storage statistics
 */
export async function getStorageStats(store: unknown): Promise<StorageStats> {
  if (!isRetentionCapable(store)) {
    return {
      observation_count: 0,
      estimated_size_bytes: 0,
    };
  }

  return store.getStats();
}
